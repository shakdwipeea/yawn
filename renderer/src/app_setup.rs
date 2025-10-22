use std::sync::mpsc::{self, Sender};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use web_sys::AddEventListenerOptions;
#[cfg(target_arch = "wasm32")]
use wgpu::Error;

use crate::message::WindowEvent;
#[cfg(target_arch = "wasm32")]
use crate::platform::web;
#[cfg(target_arch = "wasm32")]
use crate::platform::web::worker::MainWorker;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// Helper struct to store event listener closures
#[cfg(target_arch = "wasm32")]
pub struct EventListeners {
    pub resize_listener: Option<Closure<dyn FnMut()>>,
    pub mousemove_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub mousedown_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub wheel_listener: Option<Closure<dyn FnMut(web_sys::WheelEvent)>>,
    pub keyboard_listener: Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
}

#[cfg(target_arch = "wasm32")]
impl EventListeners {
    pub fn new() -> Self {
        Self {
            resize_listener: None,
            mousemove_listener: None,
            mousedown_listener: None,
            wheel_listener: None,
            keyboard_listener: None,
        }
    }
}

/// Setup default window event listeners that forward events to the worker thread
#[cfg(target_arch = "wasm32")]
pub fn setup_event_listeners(worker_chan: &Sender<WindowEvent>) -> Result<EventListeners, JsValue> {
    let window = web_sys::window().unwrap();
    let resize_worker_chan = worker_chan.clone();

    let resize_listener: Closure<dyn FnMut()> = Closure::new(move || {
        use crate::message::ResizeMessage;

        let window = web_sys::window().unwrap();
        let width = window.inner_width().ok().unwrap().as_f64().unwrap();
        let height = window.inner_height().ok().unwrap().as_f64().unwrap();

        resize_worker_chan
            .send(WindowEvent::Resize(ResizeMessage {
                width,
                height,
                scale_factor: window.device_pixel_ratio(),
            }))
            .unwrap();
    });

    window.add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref())?;

    let mousemove_worker_chan = worker_chan.clone();
    let mousemove_listener: Closure<dyn FnMut(web_sys::MouseEvent)> =
        Closure::new(move |event: web_sys::MouseEvent| {
            use crate::message::MouseMessage;
            if event.buttons() & 0x04 != 0 {
                event.prevent_default();
            }
            let mouse_event_data = MouseMessage::from_evt(event.clone());

            let mut event_data = WindowEvent::PointerMove(mouse_event_data.clone());
            if event.type_() == "click" {
                event_data = WindowEvent::PointerClick(mouse_event_data.clone());
            }

            mousemove_worker_chan.clone().send(event_data).unwrap();
        });

    window.add_event_listener_with_callback(
        "mousemove",
        mousemove_listener.as_ref().unchecked_ref(),
    )?;

    window
        .add_event_listener_with_callback("click", mousemove_listener.as_ref().unchecked_ref())?;

    let mousedown_listener: Closure<dyn FnMut(web_sys::MouseEvent)> =
        Closure::new(move |event: web_sys::MouseEvent| {
            if event.button() == 1 {
                event.prevent_default();
            }
        });

    window.add_event_listener_with_callback(
        "mousedown",
        mousedown_listener.as_ref().unchecked_ref(),
    )?;

    let wheel_worker_chan = worker_chan.clone();
    let wheel_listener: Closure<dyn FnMut(web_sys::WheelEvent)> =
        Closure::new(move |event: web_sys::WheelEvent| {
            use crate::message::WheelMessage;

            event.prevent_default();
            let wheel_event_data = WheelMessage::from_evt(event);

            wheel_worker_chan
                .send(WindowEvent::PointerWheel(wheel_event_data))
                .unwrap();
        });

    let wheel_options = {
        let options = AddEventListenerOptions::new();
        options.set_passive(false);
        options
    };

    window.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        wheel_listener.as_ref().unchecked_ref(),
        &wheel_options,
    )?;

    let keyboard_worker_chan = worker_chan.clone();
    let keyboard_listener: Closure<dyn FnMut(web_sys::KeyboardEvent)> =
        Closure::new(move |event: web_sys::KeyboardEvent| {
            use crate::message::KeyboardMessage;

            let keyboard_event_data = KeyboardMessage::from_evt(event);

            keyboard_worker_chan
                .send(WindowEvent::Keyboard(keyboard_event_data))
                .unwrap();
        });

    window
        .add_event_listener_with_callback("keydown", keyboard_listener.as_ref().unchecked_ref())?;

    Ok(EventListeners {
        resize_listener: Some(resize_listener),
        mousemove_listener: Some(mousemove_listener),
        mousedown_listener: Some(mousedown_listener),
        wheel_listener: Some(wheel_listener),
        keyboard_listener: Some(keyboard_listener),
    })
}

/// Runtime resources required to keep a WASM application running.
#[cfg(target_arch = "wasm32")]
pub struct WebAppRuntime {
    worker: MainWorker,
    worker_chan: Sender<WindowEvent>,
    _event_listeners: EventListeners,
}

#[cfg(target_arch = "wasm32")]
impl WebAppRuntime {
    /// Initialize the web worker, canvas ownership, and event listeners.
    pub fn new<T: crate::renderer::scene::Scene + 'static>(worker_name: &str, canvas_selector: &str) -> Result<Self, JsValue> {
        let (sender, receiver) = mpsc::channel::<WindowEvent>();

        let canvas = web::get_canvas_element(canvas_selector);
        let worker = MainWorker::spawn(worker_name, 1, move || {
            spawn_local(async move {
                MainWorker::run_render_loop::<T>(receiver).await;
            });
        })?;

        worker.transfer_ownership(&canvas);

        let event_listeners = setup_event_listeners(&sender)?;

        Ok(Self {
            worker,
            worker_chan: sender,
            _event_listeners: event_listeners,
        })
    }

    /// Access the worker channel sender for dispatching custom window events.
    pub fn sender(&self) -> &Sender<WindowEvent> {
        &self.worker_chan
    }

    /// Access the spawned worker reference.
    pub fn worker(&self) -> &MainWorker {
        &self.worker
    }
}

/// Trait for applications that rely on the renderer's default WASM setup.
#[cfg(target_arch = "wasm32")]
pub trait WebApp {
    type Scene: crate::renderer::scene::Scene + 'static;

    /// Name used for the spawned `MainWorker`.
    fn worker_name() -> &'static str {
        "main-worker"
    }

    /// CSS selector for the canvas element that will be transferred to the worker.
    fn canvas_selector() -> &'static str {
        "#canvas0"
    }

    /// Hook invoked after the runtime has been created.
    fn on_runtime_initialized(_runtime: &mut WebAppRuntime) {}

    /// Perform the default WASM initialization routine.
    fn setup_runtime() -> Result<WebAppRuntime, JsValue> {
        let mut runtime = WebAppRuntime::new::<Self::Scene>(
            Self::worker_name(),
            Self::canvas_selector(),
        )?;
        Self::on_runtime_initialized(&mut runtime);
        Ok(runtime)
    }
}

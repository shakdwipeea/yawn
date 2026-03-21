#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Sender};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use web_sys::AddEventListenerOptions;

#[cfg(target_arch = "wasm32")]
use crate::message::{MeshData, OrbitMessage, SceneCommand, WindowEvent, ZoomMessage};
#[cfg(target_arch = "wasm32")]
#[cfg(target_arch = "wasm32")]
use crate::platform::web;
#[cfg(target_arch = "wasm32")]
use crate::platform::web::worker::MainWorker;

#[cfg(target_arch = "wasm32")]
fn init_platform() {
    static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

    console_error_panic_hook::set_once();
    LOGGER_INIT.call_once(|| wasm_logger::init(wasm_logger::Config::default()));
}

/// Helper struct to store event listener closures.
#[cfg(target_arch = "wasm32")]
pub struct EventListeners {
    pub resize_listener: Option<Closure<dyn FnMut()>>,
    pub mousemove_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub mousedown_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub wheel_listener: Option<Closure<dyn FnMut(web_sys::WheelEvent)>>,
    pub keyboard_listener: Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
}

#[cfg(target_arch = "wasm32")]
impl Default for EventListeners {
    fn default() -> Self {
        Self {
            resize_listener: None,
            mousemove_listener: None,
            mousedown_listener: None,
            wheel_listener: None,
            keyboard_listener: None,
        }
    }
}

/// Setup default window event listeners that forward events to the worker thread.
#[cfg(target_arch = "wasm32")]
fn setup_event_listeners(worker_chan: &Sender<WindowEvent>) -> Result<EventListeners, JsValue> {
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

pub struct App {
    worker: MainWorker,
    sender: Sender<WindowEvent>,
    _event_listeners: EventListeners,
}

impl App {
    #[cfg(target_arch = "wasm32")]
    /// Initialize the web worker, canvas ownership, and event listeners.
    pub fn new(worker_name: &str, canvas_selector: &str) -> Result<Self, JsValue> {
        init_platform();

        let (sender, receiver) = mpsc::channel::<WindowEvent>();

        let canvas = web::get_canvas_element(canvas_selector);
        let worker = MainWorker::spawn(worker_name, 1, move || {
            spawn_local(async move {
                MainWorker::run_render_loop(receiver).await;
            });
        })?;

        worker.transfer_ownership(&canvas);

        let event_listeners = setup_event_listeners(&sender)?;

        Ok(Self {
            worker,
            sender,
            _event_listeners: event_listeners,
        })
    }

    /// Add a mesh to the scene.
    pub fn add_mesh(&self, mesh: MeshData) {
        let _ = self
            .sender
            .send(WindowEvent::SceneCommand(SceneCommand::AddMesh(mesh)));
    }

    /// Clear all meshes from the scene.
    pub fn clear_scene(&self) {
        let _ = self
            .sender
            .send(WindowEvent::SceneCommand(SceneCommand::Clear));
    }

    /// Set the camera position and look-at target.
    pub fn set_camera_look_at(&self, eye: [f32; 3], target: [f32; 3]) {
        let _ = self
            .sender
            .send(WindowEvent::SceneCommand(SceneCommand::SetCameraLookAt {
                eye,
                target,
            }));
    }

    /// Orbit the camera by the given pixel deltas.
    pub fn orbit_camera(&self, dx: f32, dy: f32) {
        let _ = self.sender.send(WindowEvent::CameraOrbit(OrbitMessage {
            delta_x: dx,
            delta_y: dy,
        }));
    }

    /// Zoom the camera by the given delta (negative = zoom in, positive = zoom out).
    pub fn zoom_camera(&self, delta: f32) {
        let _ = self
            .sender
            .send(WindowEvent::CameraZoom(ZoomMessage { delta }));
    }

    /// Access the spawned worker reference.
    pub fn worker(&self) -> &MainWorker {
        &self.worker
    }
}

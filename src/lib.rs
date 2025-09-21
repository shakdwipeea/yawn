use std::sync::mpsc::{self, Sender};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use crate::{message::WindowEvent, platform::web, platform::web::worker::MainWorker};

mod camera;
mod gltf;
mod message;
mod platform;
mod renderer;

#[cfg(target_arch = "wasm32")]
pub struct App {
    _worker: platform::web::worker::MainWorker,
    worker_chan: Sender<WindowEvent>,

    // Store closures to keep them alive
    resize_listener: Option<Closure<dyn FnMut()>>,
    mousemove_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    mousedown_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
}

impl App {
    pub async fn new() -> Result<Self, JsValue> {
        let (sender, receiver) = mpsc::channel::<WindowEvent>();

        let canvas = web::get_canvas_element("#canvas0");
        let _worker = MainWorker::spawn("main-worker", 1, move || {
            wasm_bindgen_futures::spawn_local(async move {
                MainWorker::run_render_loop(receiver).await;
            });
        })?;

        _worker.transfer_ownership(&canvas);

        let mut app = App {
            _worker,
            worker_chan: sender,
            resize_listener: None,
            mousemove_listener: None,
            mousedown_listener: None,
        };

        app.setup_event_listeners();
        Ok(app)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn setup_event_listeners(&mut self) {
        let window = web_sys::window().unwrap();
        let resize_worker_chan = self.worker_chan.clone();

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

        let _ = window
            .add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref());

        let mousemove_worker_chan = self.worker_chan.clone();
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

        let _ = window
            .add_event_listener_with_callback(
                "mousemove",
                mousemove_listener.as_ref().unchecked_ref(),
            )
            .unwrap();

        let _ = window
            .add_event_listener_with_callback("click", mousemove_listener.as_ref().unchecked_ref())
            .unwrap();

        let mousedown_listener: Closure<dyn FnMut(web_sys::MouseEvent)> =
            Closure::new(move |event: web_sys::MouseEvent| {
                if event.button() == 1 {
                    event.prevent_default();
                }
            });

        let _ = window
            .add_event_listener_with_callback(
                "mousedown",
                mousedown_listener.as_ref().unchecked_ref(),
            )
            .unwrap();

        self.resize_listener = Some(resize_listener);
        self.mousemove_listener = Some(mousemove_listener);
        self.mousedown_listener = Some(mousedown_listener);
    }
}

/// Entrypoint for the main thread
#[wasm_bindgen]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    wasm_bindgen_futures::spawn_local(async {
        let app = App::new().await.unwrap();
        // keep the app running, and prevent drops of the objects App owns
        Box::leak(Box::new(app));
    });
}

// executes the closure it is spawned with
#[wasm_bindgen]
pub fn worker_entrypoint(ptr: u32) {
    let work = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce()>) };
    (*work)();
}

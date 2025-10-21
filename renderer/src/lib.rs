use std::sync::mpsc::{self, Sender};

use wasm_bindgen::prelude::*;

use crate::{message::WindowEvent, platform::web, platform::web::worker::MainWorker};

pub mod app_setup;
pub mod camera;
pub mod gltf;
pub mod message;
pub mod platform;
pub mod renderer;
pub mod traits;

/// Worker entrypoint helper - executes the closure it is spawned with
/// Applications should export this with #[wasm_bindgen]
pub fn worker_entrypoint_impl(ptr: u32) {
    let work = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce()>) };
    (*work)();
}

/// Macro to export the worker_entrypoint function in application crates
/// 
/// Usage:
/// ```rust
/// use renderer::export_worker_entrypoint;
/// export_worker_entrypoint!();
/// ```
#[macro_export]
macro_rules! export_worker_entrypoint {
    () => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub fn worker_entrypoint(ptr: u32) {
            $crate::worker_entrypoint_impl(ptr);
        }
    };
}

#[cfg(target_arch = "wasm32")]
pub struct App {
    _worker: platform::web::worker::MainWorker,
    worker_chan: Sender<WindowEvent>,
    _event_listeners: app_setup::EventListeners,
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

        let event_listeners = app_setup::setup_event_listeners(&sender)?;

        let app = App {
            _worker,
            worker_chan: sender,
            _event_listeners: event_listeners,
        };

        Ok(app)
    }
}

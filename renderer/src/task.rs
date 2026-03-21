use std::sync::mpsc::Sender;

use crate::message::WindowEvent;

/// Run an async function that produces events, sending each result back
/// through `sender`. How the future is spawned is platform-specific and
/// fully encapsulated here.
pub fn execute_async_event(
    sender: Sender<WindowEvent>,
    f: impl std::future::Future<Output = Vec<WindowEvent>> + 'static,
) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            for event in f.await {
                sender.send(event).ok();
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let boxed = f.boxed_local();
        std::thread::spawn(move || {
            for event in futures::executor::block_on(boxed) {
                sender.send(event).ok();
            }
        });
    }
}

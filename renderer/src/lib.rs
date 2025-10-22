pub mod app_setup;
pub mod camera;
pub mod gltf;
pub mod message;
pub mod platform;
pub mod renderer;

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

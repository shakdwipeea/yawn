/// Spawn an async future on the appropriate platform executor.
///
/// On wasm32 this uses `spawn_local`; on native it spawns a thread and
/// blocks on the future with `futures::executor`.
#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

/// Native tasks hop to a background thread, so the future must be `Send`.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    std::thread::spawn(move || {
        futures::executor::block_on(future);
    });
}

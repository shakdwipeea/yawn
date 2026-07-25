#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use level_editor::LevelEditor;
    use renderer::app_runtime::WaylandAppRuntime;

    // Initialise the native logger so `log::{info,warn,error}!` calls inside
    // the renderer surface in the terminal. We default to `info` for our own
    // crates and silence wgpu/naga's chatty `info` output unless the user
    // overrides `RUST_LOG` explicitly. The WASM build initialises
    // `wasm_logger` separately in `WebAppRuntime::new`.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();

    log::info!("starting level-editor native runtime");

    let app = WaylandAppRuntime::new().expect("Failed to create Wayland runtime");
    let editor = LevelEditor::from_app(app);
    editor
        .into_app()
        .run()
        .expect("Failed to run Wayland runtime");
}

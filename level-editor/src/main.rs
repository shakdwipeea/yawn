#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use level_editor::LevelEditor;
    use renderer::app_runtime::WaylandAppRuntime;

    let app = WaylandAppRuntime::new().expect("Failed to create Wayland runtime");
    let editor = LevelEditor::from_app(app);
    editor
        .into_app()
        .run()
        .expect("Failed to run Wayland runtime");
}

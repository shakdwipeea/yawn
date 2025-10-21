use std::sync::mpsc::{self, Sender};
use wasm_bindgen::prelude::*;

use renderer::app_setup;
use renderer::message::WindowEvent;
use renderer::platform::web;
use renderer::platform::web::worker::MainWorker;
use renderer::renderer as gpu_renderer;
use renderer::renderer::scene::Scene;
use renderer::traits::SceneTrait;

/// Level editor specific scene that extends the base scene
pub struct EditorScene {
    base: Scene,
    // Add level-editor specific fields here
    // For example:
    // pub selected_object: Option<usize>,
    // pub grid_enabled: bool,
}

impl SceneTrait for EditorScene {
    fn setup(
        &mut self,
        device: &wgpu::Device,
        resources: &mut gpu_renderer::GpuResources,
        surface_format: wgpu::TextureFormat,
    ) {
        // Call base scene setup
        self.base.setup(device, resources, surface_format);

        // Add level-editor specific setup here
        // For example, load additional tools, UI elements, etc.
    }

    fn bind_group_layouts(&self) -> &[wgpu::BindGroupLayout] {
        self.base.bind_group_layouts()
    }

    fn bind_groups(&self) -> &[wgpu::BindGroup] {
        self.base.bind_groups()
    }

    fn uniform_buffers(&self) -> &[wgpu::Buffer] {
        self.base.uniform_buffers()
    }

    fn resize(&mut self, width: f64, height: f64, scale_factor: f64, queue: &wgpu::Queue) {
        self.base.resize(width, height, scale_factor, queue);
        // Add level-editor specific resize logic here
    }

    fn update(&mut self, queue: &wgpu::Queue) {
        self.base.update(queue);
        // Add level-editor specific update logic here
    }

    fn meshes(&self) -> &[gpu_renderer::Mesh] {
        self.base.meshes()
    }
}

impl EditorScene {
    pub fn new(device: &wgpu::Device, dimension: ultraviolet::Vec2) -> Self {
        EditorScene {
            base: Scene::new(device, dimension),
            // Initialize level-editor specific fields here
        }
    }
}

/// Level editor app
#[cfg(target_arch = "wasm32")]
pub struct EditorApp {
    _worker: MainWorker,
    worker_chan: Sender<WindowEvent>,
    _event_listeners: app_setup::EventListeners,
}

impl EditorApp {
    pub async fn new() -> Result<Self, JsValue> {
        let (sender, receiver) = mpsc::channel::<WindowEvent>();

        let canvas = web::get_canvas_element("#canvas0");
        let _worker = MainWorker::spawn("main-worker", 1, move || {
            wasm_bindgen_futures::spawn_local(async move {
                // Use the custom render loop with EditorScene
                MainWorker::run_render_loop(receiver).await;
            });
        })?;

        _worker.transfer_ownership(&canvas);

        let event_listeners = app_setup::setup_event_listeners(&sender)?;

        let app = EditorApp {
            _worker,
            worker_chan: sender,
            _event_listeners: event_listeners,
        };

        Ok(app)
    }
}

/// Entrypoint for the level editor
#[wasm_bindgen]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    wasm_bindgen_futures::spawn_local(async {
        let app = EditorApp::new().await.unwrap();
        // Keep the app running and prevent drops
        Box::leak(Box::new(app));
    });
}

renderer::export_worker_entrypoint!();

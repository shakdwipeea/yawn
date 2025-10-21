use wasm_bindgen::prelude::*;

use renderer::app_setup::WebApp;
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

#[cfg(target_arch = "wasm32")]
pub struct LevelEditor {
    #[allow(dead_code)]
    scene: EditorScene,
}

#[cfg(target_arch = "wasm32")]
impl WebApp for LevelEditor {
    type Scene = EditorScene;
}

/// Entrypoint for the level editor
#[wasm_bindgen]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::default());

    wasm_bindgen_futures::spawn_local(async {
        let runtime = LevelEditor::setup_runtime().unwrap();
        // Keep the runtime running and prevent drops
        Box::leak(Box::new(runtime));
    });
}

renderer::export_worker_entrypoint!();

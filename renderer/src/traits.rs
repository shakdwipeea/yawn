use wasm_bindgen::prelude::*;
use wgpu::Device;

use crate::renderer::GpuResources;

/// Trait defining the behavior of a Scene
/// Implement this trait to customize your scene setup
pub trait SceneTrait {
    /// Initialize the scene with device and resources
    fn setup(
        &mut self,
        device: &Device,
        resources: &mut GpuResources,
        surface_format: wgpu::TextureFormat,
    );

    /// Get bind group layouts for the scene
    fn bind_group_layouts(&self) -> &[wgpu::BindGroupLayout];

    /// Get bind groups for the scene
    fn bind_groups(&self) -> &[wgpu::BindGroup];

    /// Get uniform buffers for the scene
    fn uniform_buffers(&self) -> &[wgpu::Buffer];

    /// Handle window resize
    fn resize(&mut self, width: f64, height: f64, scale_factor: f64, queue: &wgpu::Queue);

    /// Update scene per frame
    fn update(&mut self, queue: &wgpu::Queue);

    /// Get meshes to render
    fn meshes(&self) -> &[crate::renderer::Mesh];
}

/// Trait defining the behavior of an App
/// Implement this trait to customize your application setup and event handling
#[cfg(target_arch = "wasm32")]
pub trait AppTrait: Sized {
    type Scene: SceneTrait;

    /// Create a new instance of the app
    fn new() -> impl std::future::Future<Output = Result<Self, JsValue>> + Send;

    /// Get a reference to the scene
    fn scene(&self) -> &Self::Scene;

    /// Get a mutable reference to the scene
    fn scene_mut(&mut self) -> &mut Self::Scene;

    /// Setup event listeners
    /// This is called during initialization and should wire up all DOM event handlers
    fn setup_event_listeners(&mut self);

    /// Get the worker channel sender
    fn worker_channel(&self) -> &std::sync::mpsc::Sender<crate::message::WindowEvent>;
}

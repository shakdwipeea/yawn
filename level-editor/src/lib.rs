use std::sync::mpsc::Receiver;
use std::{cell::RefCell, rc::Rc};
use ultraviolet::Mat4;
use wasm_bindgen::prelude::*;

use renderer::app_setup::WebApp;
use renderer::camera::Camera;
use renderer::message::WindowEvent;
use renderer::renderer as gpu_renderer;
use renderer::renderer::scene::{mesh_vertex_layout, FrameMetadata, Mesh, MeshBuilder};

/// Simple vertex format.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

pub struct EditorScene {
    uniform_buffers: [wgpu::Buffer; 2],
    bind_groups: [wgpu::BindGroup; 2],
    bind_group_layouts: [wgpu::BindGroupLayout; 2],
    frame_metadata: FrameMetadata,
    cam: Camera,
    meshes: Vec<Mesh>,
}

impl renderer::renderer::scene::Scene for EditorScene {
    fn setup(
        renderer_context: &gpu_renderer::RendererContext,
        resources: &mut gpu_renderer::GpuResources,
    ) -> Self {
        let dimension = ultraviolet::Vec2::new(
            renderer_context.surface_config.width as f32,
            renderer_context.surface_config.height as f32,
        );

        let mut frame_metadata = FrameMetadata::new(dimension);
        let camera = Camera::new(dimension.x / dimension.y);

        frame_metadata.set_camera_position(camera.position());

        let uniform_resource = frame_metadata.create_uniform_resource(&renderer_context.device);
        let camera_resource = camera.create_uniform_resource(&renderer_context.device);

        let bind_group_layouts = [
            uniform_resource.bind_group_layout,
            camera_resource.bind_group_layout,
        ];

        resources.set_bind_group_layouts(&bind_group_layouts);

        let mut scene = EditorScene {
            uniform_buffers: [uniform_resource.buffer, camera_resource.buffer],
            bind_groups: [uniform_resource.bind_group, camera_resource.bind_group],
            bind_group_layouts,
            frame_metadata,
            cam: camera,
            meshes: Vec::new(),
        };

        scene.create_default_scene(
            &renderer_context.device,
            resources,
            renderer_context.surface_config.format,
        );

        scene
    }

    fn frame_metadata_mut(&mut self) -> Option<&mut FrameMetadata> {
        Some(&mut self.frame_metadata)
    }

    fn camera_mut(&mut self) -> Option<&mut Camera> {
        Some(&mut self.cam)
    }

    fn uniform_buffers(&self) -> Option<&[wgpu::Buffer]> {
        Some(&self.uniform_buffers)
    }

    fn bind_groups(&self) -> &[wgpu::BindGroup] {
        &self.bind_groups
    }

    fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    fn handle_mouse_click(&mut self, x: f32, y: f32) {
        self.frame_metadata.mouse_click = [x, y];
    }

    fn handle_zoom(&mut self, _delta_y: f32) {
        // TODO: Implement zoom properly when Camera exposes necessary methods
    }

    fn handle_orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.cam.orbit(delta_x, delta_y);
    }

    fn clear(&mut self) {
        self.meshes.clear();
    }

    fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    fn set_camera_depth_range(&mut self, near: f32, far: f32) {
        self.cam.set_depth_range(near, far);
    }

    fn set_camera_look_at(&mut self, eye: ultraviolet::Vec3, center: ultraviolet::Vec3) {
        self.cam.look_at(eye, center);
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

impl EditorScene {
    /// Ground plane vertex data.
    const VERTICES: &[Vertex] = &[
        // First triangle of quad
        Vertex {
            pos: [-5.0, 0.0, -5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
        Vertex {
            pos: [5.0, 0.0, -5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
        Vertex {
            pos: [-5.0, 0.0, 5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
        // Second triangle of quad
        Vertex {
            pos: [5.0, 0.0, -5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
        Vertex {
            pos: [5.0, 0.0, 5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
        Vertex {
            pos: [-5.0, 0.0, 5.0],
            color: [0.2, 0.8, 0.2], // Green
        },
    ];
    // Wind the ground plane so the upward-facing side is front-facing (CCW from
    // above) to avoid being culled by the default back-face culling.
    const INDICES: &[u32] = &[0, 2, 1, 3, 5, 4];

    fn create_default_scene(
        &mut self,
        device: &wgpu::Device,
        resources: &mut gpu_renderer::GpuResources,
        surface_format: wgpu::TextureFormat,
    ) {
        let positions: Vec<[f32; 3]> = Self::VERTICES.iter().map(|v| v.pos).collect();
        // Ground plane normals point upward (Y+)
        let normals: Vec<[f32; 3]> = vec![[0.0, 1.0, 0.0]; positions.len()];
        let uvs: &[[f32; 2]] = &[
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];

        let vertex_layout = mesh_vertex_layout();

        let pipeline_index = resources.get_or_create_pipeline(
            device,
            "ground_plane",
            &vertex_layout,
            include_str!("./program.wgsl"),
            surface_format,
        );

        let scale_factor = 100.0;
        let scale_matrix = Mat4::from_scale(scale_factor);

        let mesh = MeshBuilder::default()
            .with_vertices(device, resources, &positions, &normals, uvs)
            .with_indices(device, resources, Self::INDICES)
            .with_pipeline(pipeline_index)
            .with_model_matrix(device, resources, scale_matrix)
            .build();

        self.meshes.push(mesh);
    }
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

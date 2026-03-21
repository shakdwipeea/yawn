use ultraviolet::{Mat4, Vec3};
use wasm_bindgen::prelude::*;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use renderer::app_setup::App;
use renderer::renderer as gpu_renderer;
use renderer::renderer::scene::{mesh_vertex_layout, Mesh, MeshBuilder, Scene};

/// Simple vertex format.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

/// Ground plane vertex data (double-sided: 6 vertices for top, 6 for bottom).
const VERTICES: &[Vertex] = &[
    // Top face (visible from above)
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
    // Bottom face (visible from below)
    Vertex {
        pos: [-5.0, 0.0, -5.0],
        color: [0.15, 0.6, 0.15], // Slightly darker green
    },
    Vertex {
        pos: [5.0, 0.0, -5.0],
        color: [0.15, 0.6, 0.15],
    },
    Vertex {
        pos: [-5.0, 0.0, 5.0],
        color: [0.15, 0.6, 0.15],
    },
    Vertex {
        pos: [5.0, 0.0, -5.0],
        color: [0.15, 0.6, 0.15],
    },
    Vertex {
        pos: [5.0, 0.0, 5.0],
        color: [0.15, 0.6, 0.15],
    },
    Vertex {
        pos: [-5.0, 0.0, 5.0],
        color: [0.15, 0.6, 0.15],
    },
];

// Wind the ground plane so the upward-facing side is front-facing (CCW from
// above) to avoid being culled by the default back-face culling.
// Bottom face is wound in reverse order (CW from above = CCW from below).
const INDICES: &[u32] = &[
    0, 2, 1, 3, 5, 4, // Top face
    6, 7, 8, 9, 10, 11, // Bottom face (reversed winding)
];

/// Box vertex positions (24 vertices: 4 per face for proper normals).
/// Each face has its own vertices so normals can be per-face.
const BOX_POSITIONS: &[[f32; 3]] = &[
    // Front face (Z+)
    [-0.5, -0.5, 0.5],
    [0.5, -0.5, 0.5],
    [0.5, 0.5, 0.5],
    [-0.5, 0.5, 0.5],
    // Back face (Z-)
    [0.5, -0.5, -0.5],
    [-0.5, -0.5, -0.5],
    [-0.5, 0.5, -0.5],
    [0.5, 0.5, -0.5],
    // Top face (Y+)
    [-0.5, 0.5, 0.5],
    [0.5, 0.5, 0.5],
    [0.5, 0.5, -0.5],
    [-0.5, 0.5, -0.5],
    // Bottom face (Y-)
    [-0.5, -0.5, -0.5],
    [0.5, -0.5, -0.5],
    [0.5, -0.5, 0.5],
    [-0.5, -0.5, 0.5],
    // Right face (X+)
    [0.5, -0.5, 0.5],
    [0.5, -0.5, -0.5],
    [0.5, 0.5, -0.5],
    [0.5, 0.5, 0.5],
    // Left face (X-)
    [-0.5, -0.5, -0.5],
    [-0.5, -0.5, 0.5],
    [-0.5, 0.5, 0.5],
    [-0.5, 0.5, -0.5],
];

/// Box normals (one per vertex, matching BOX_POSITIONS).
const BOX_NORMALS: &[[f32; 3]] = &[
    // Front face (Z+)
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    // Back face (Z-)
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, -1.0],
    // Top face (Y+)
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    // Bottom face (Y-)
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, -1.0, 0.0],
    // Right face (X+)
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    // Left face (X-)
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
];

/// Box indices (36 indices: 6 faces × 2 triangles × 3 vertices).
const BOX_INDICES: &[u32] = &[
    0, 1, 2, 2, 3, 0, // Front
    4, 5, 6, 6, 7, 4, // Back
    8, 9, 10, 10, 11, 8, // Top
    12, 13, 14, 14, 15, 12, // Bottom
    16, 17, 18, 18, 19, 16, // Right
    20, 21, 22, 22, 23, 20, // Left
];

/// Box UVs (one per vertex).
const BOX_UVS: &[[f32; 2]] = &[
    // Front face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
    // Back face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
    // Top face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
    // Bottom face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
    // Right face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
    // Left face
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0],
    [0.0, 0.0],
];

fn create_box_mesh(
    device: &wgpu::Device,
    resources: &mut gpu_renderer::GpuResources,
    pipeline_index: usize,
    position: Vec3,
    size: f32,
) -> Mesh {
    let model_matrix = Mat4::from_translation(position) * Mat4::from_scale(size);

    MeshBuilder::default()
        .with_vertices(device, resources, BOX_POSITIONS, BOX_NORMALS, BOX_UVS)
        .with_indices(device, resources, BOX_INDICES)
        .with_pipeline(pipeline_index)
        .with_model_matrix(device, resources, model_matrix)
        .build()
}

fn create_default_scene(
    scene: &mut Scene,
    device: &wgpu::Device,
    resources: &mut gpu_renderer::GpuResources,
    surface_format: wgpu::TextureFormat,
) {
    let positions: Vec<[f32; 3]> = VERTICES.iter().map(|v| v.pos).collect();
    // Ground plane normals: top face points up (Y+), bottom face points down (Y-)
    let normals: Vec<[f32; 3]> = vec![
        // Top face normals (upward)
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        // Bottom face normals (downward)
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let uvs: &[[f32; 2]] = &[
        // Top face UVs
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        // Bottom face UVs
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
        .with_indices(device, resources, INDICES)
        .with_pipeline(pipeline_index)
        .with_model_matrix(device, resources, scale_matrix)
        .build();

    scene.add_mesh(mesh);

    // Add random boxes
    let mut rng = SmallRng::seed_from_u64(42); // Fixed seed for reproducibility
    let num_boxes = 15;
    let box_size = 50.0; // Box size in world units

    for i in 0..num_boxes {
        // Random position within ground plane bounds
        // Ground plane is scaled by 100, so -500 to 500 in world units
        // Keep boxes well within bounds
        let x = rng.gen_range(-300.0..300.0);
        let z = rng.gen_range(-300.0..300.0);
        // Y position: half box height places bottom on ground
        // First half of boxes sit on ground, rest float at random heights
        let y = if i < num_boxes / 2 {
            box_size / 2.0 // On ground
        } else {
            box_size / 2.0 + rng.gen_range(25.0..150.0) // Floating
        };

        let position = Vec3::new(x, y, z);
        let mesh = create_box_mesh(device, resources, pipeline_index, position, box_size);
        scene.add_mesh(mesh);
    }

    // Position camera appropriately for the scaled scene (100x scale)
    scene.set_camera_look_at(
        Vec3::new(0.0, 400.0, 600.0), // Eye position: above and behind
        Vec3::new(0.0, 0.0, 0.0),     // Look at origin
    );
}

/// Handle returned from main() for controlling the application from JS.
#[wasm_bindgen]
pub struct AppHandle {
    app: App,
}

#[wasm_bindgen]
impl AppHandle {
    /// Orbit the camera by the given pixel deltas.
    pub fn orbit_camera(&self, dx: f32, dy: f32) {
        self.app.orbit_camera(dx, dy);
    }

    /// Zoom the camera by the given delta (negative = zoom in, positive = zoom out).
    pub fn zoom_camera(&self, delta: f32) {
        self.app.zoom_camera(delta);
    }
}

/// Entrypoint for the level editor - returns handle for JS interaction
#[wasm_bindgen]
pub fn start() -> AppHandle {
    AppHandle {
        app: App::new("main-worker", "#canvas0").unwrap(),
    }
}

renderer::export_worker_entrypoint!();

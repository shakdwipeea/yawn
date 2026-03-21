use ultraviolet::Mat4;
use wasm_bindgen::prelude::*;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use renderer::app_setup::App;
use renderer::message::MeshData;

const SHADER_SOURCE: &str = include_str!("./program.wgsl");

/// Ground plane vertex positions (double-sided: 6 vertices for top, 6 for bottom).
const GROUND_POSITIONS: &[[f32; 3]] = &[
    // Top face (visible from above)
    [-5.0, 0.0, -5.0],
    [5.0, 0.0, -5.0],
    [-5.0, 0.0, 5.0],
    [5.0, 0.0, -5.0],
    [5.0, 0.0, 5.0],
    [-5.0, 0.0, 5.0],
    // Bottom face (visible from below)
    [-5.0, 0.0, -5.0],
    [5.0, 0.0, -5.0],
    [-5.0, 0.0, 5.0],
    [5.0, 0.0, -5.0],
    [5.0, 0.0, 5.0],
    [-5.0, 0.0, 5.0],
];

const GROUND_NORMALS: &[[f32; 3]] = &[
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

const GROUND_UVS: &[[f32; 2]] = &[
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

// Wind the ground plane so the upward-facing side is front-facing (CCW from
// above) to avoid being culled by the default back-face culling.
// Bottom face is wound in reverse order (CW from above = CCW from below).
const GROUND_INDICES: &[u32] = &[
    0, 2, 1, 3, 5, 4, // Top face
    6, 7, 8, 9, 10, 11, // Bottom face (reversed winding)
];

/// Box vertex positions (24 vertices: 4 per face for proper normals).
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

const BOX_INDICES: &[u32] = &[
    0, 1, 2, 2, 3, 0, // Front
    4, 5, 6, 6, 7, 4, // Back
    8, 9, 10, 10, 11, 8, // Top
    12, 13, 14, 14, 15, 12, // Bottom
    16, 17, 18, 18, 19, 16, // Right
    20, 21, 22, 22, 23, 20, // Left
];

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

/// Number of boxes added to the default level-editor scene.
const DEFAULT_BOX_COUNT: usize = 15;

/// Total mesh count in the default level-editor scene.
const DEFAULT_MESH_COUNT: usize = DEFAULT_BOX_COUNT + 1;

fn mat4_to_array(m: Mat4) -> [f32; 16] {
    let s = m.as_slice();
    let mut out = [0.0f32; 16];
    out.copy_from_slice(s);
    out
}

fn make_ground_mesh(scale: f32) -> MeshData {
    MeshData {
        positions: GROUND_POSITIONS.to_vec(),
        normals: GROUND_NORMALS.to_vec(),
        uvs: GROUND_UVS.to_vec(),
        indices: GROUND_INDICES.to_vec(),
        model_matrix: mat4_to_array(Mat4::from_scale(scale)),
        shader_source: SHADER_SOURCE.to_string(),
        pipeline_key: "level_editor_lit".to_string(),
    }
}

fn make_box_mesh(position: [f32; 3], size: f32) -> MeshData {
    let model_matrix = Mat4::from_translation(position.into()) * Mat4::from_scale(size);

    MeshData {
        positions: BOX_POSITIONS.to_vec(),
        normals: BOX_NORMALS.to_vec(),
        uvs: BOX_UVS.to_vec(),
        indices: BOX_INDICES.to_vec(),
        model_matrix: mat4_to_array(model_matrix),
        shader_source: SHADER_SOURCE.to_string(),
        pipeline_key: "level_editor_lit".to_string(),
    }
}

fn setup_default_scene(app: &App) {
    // Ground plane
    app.add_mesh(make_ground_mesh(100.0));

    // Random boxes
    let mut rng = SmallRng::seed_from_u64(42);
    let box_size = 50.0;

    for i in 0..DEFAULT_BOX_COUNT {
        let x = rng.gen_range(-300.0..300.0);
        let z = rng.gen_range(-300.0..300.0);
        let y = if i < DEFAULT_BOX_COUNT / 2 {
            box_size / 2.0
        } else {
            box_size / 2.0 + rng.gen_range(25.0..150.0)
        };

        app.add_mesh(make_box_mesh([x, y, z], box_size));
    }

    // Camera
    app.set_camera_look_at([0.0, 400.0, 600.0], [0.0, 0.0, 0.0]);
}

/// Handle returned from main() for controlling the application from JS.
#[wasm_bindgen]
pub struct AppHandle {
    app: App,
}

#[wasm_bindgen]
impl AppHandle {
    /// Return the number of boxes in the default level-editor scene.
    pub fn box_count(&self) -> usize {
        DEFAULT_BOX_COUNT
    }

    /// Return the total mesh count in the default level-editor scene.
    pub fn mesh_count(&self) -> usize {
        DEFAULT_MESH_COUNT
    }
}

/// Entrypoint for the level editor - returns handle for JS interaction
#[wasm_bindgen]
pub fn start() -> AppHandle {
    let app = App::new("main-worker", "#canvas0").unwrap();
    setup_default_scene(&app);
    AppHandle { app }
}

renderer::export_worker_entrypoint!();

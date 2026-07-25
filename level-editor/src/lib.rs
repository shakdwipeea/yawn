use ultraviolet::Mat4;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use renderer::app::App;
#[cfg(target_arch = "wasm32")]
use renderer::app_runtime::WebAppRuntime;
use renderer::events::MeshData;
use renderer::gltf::LoadGltfCommand;

/// Shader shared by all procedural meshes (ground, boxes). GLTF models use
/// their own shader embedded in the glTF module.
const SHADER_SOURCE: &str = include_str!("./program.wgsl");

/// Ground plane vertex positions (double-sided: 6 vertices for top, 6 for bottom).
///
/// Duplicated for top and bottom faces so each side gets its own normal
/// direction, allowing correct lighting from both above and below.
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

/// Ground indices with deliberate winding order:
/// - Top face is wound CCW when viewed from above → front-facing for back-face culling.
/// - Bottom face is wound CW from above (= CCW from below) so it also survives culling.
const GROUND_INDICES: &[u32] = &[
    0, 2, 1, 3, 5, 4, // Top face (CCW from above)
    6, 7, 8, 9, 10, 11, // Bottom face (CCW from below)
];

/// Box vertex positions (24 vertices: 4 per face for proper normals).
///
/// Each face has its own 4 vertices (rather than sharing corners) so that
/// every vertex carries the correct face normal for flat shading.
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

/// Total mesh count: boxes + 1 ground plane. Exposed via `AppHandle` so the
/// JS side (and tests) can assert the scene was populated correctly.
const DEFAULT_MESH_COUNT: usize = DEFAULT_BOX_COUNT + 1;

/// Flatten an ultraviolet `Mat4` into `[f32; 16]` for GPU uniform upload.
fn mat4_to_array(m: Mat4) -> [f32; 16] {
    let s = m.as_slice();
    let mut out = [0.0f32; 16];
    out.copy_from_slice(s);
    out
}

/// Build a ground-plane `MeshData` from the unit-size constants, scaled up by
/// `scale` via the model matrix so the vertex data stays resolution-independent.
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

/// Build a box `MeshData` at `position` with uniform `size`. The unit cube
/// constants are scaled and translated via the model matrix so all boxes share
/// the same vertex data and only differ in their transform.
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

/// Populate the scene with a ground plane and randomly placed boxes.
///
/// A fixed seed (42) is used so the layout is deterministic across reloads,
/// which makes visual regression testing and screenshots reproducible.
pub fn setup_default_scene(app: &impl App) {
    app.add_mesh(make_ground_mesh(100.0)).unwrap();

    let mut rng = SmallRng::seed_from_u64(42);
    let box_size = 50.0;

    for i in 0..DEFAULT_BOX_COUNT {
        let x = rng.gen_range(-300.0..300.0);
        let z = rng.gen_range(-300.0..300.0);
        // First half of boxes sit on the ground; the rest are raised to
        // random heights to give the scene visual depth and test the camera
        // framing from the elevated viewpoint.
        let y = if i < DEFAULT_BOX_COUNT / 2 {
            box_size / 2.0
        } else {
            box_size / 2.0 + rng.gen_range(25.0..150.0)
        };

        app.add_mesh(make_box_mesh([x, y, z], box_size)).unwrap();
    }

    // Place the camera high and back so the full spread of boxes is visible.
    app.set_camera_look_at([0.0, 400.0, 600.0], [0.0, 0.0, 0.0])
        .unwrap();
}

pub struct LevelEditor<A: App> {
    app: A,
}

impl<A: App> LevelEditor<A> {
    /// Create a level editor around a concrete renderer runtime and populate
    /// the deterministic default scene. Runtime construction stays outside the
    /// editor so this type can be used by both native and WASM entrypoints.
    pub fn from_app(app: A) -> Self {
        setup_default_scene(&app);
        Self { app }
    }

    /// Consume the editor and return the runtime it owns.
    pub fn into_app(self) -> A {
        self.app
    }

    /// Return the number of boxes in the default level-editor scene.
    pub fn box_count(&self) -> usize {
        DEFAULT_BOX_COUNT
    }

    /// Return the total mesh count in the default level-editor scene.
    pub fn mesh_count(&self) -> usize {
        DEFAULT_MESH_COUNT
    }

    /// Load a GLTF model from a URL.
    pub fn load_gltf(&self, url: &str) {
        self.app
            .spawn_async(LoadGltfCommand::new(url, "gltf_standard").load());
    }
}

// ── WASM build ───────────────────────────────────────────────────────────────

/// Handle returned from `start()` for controlling the application from JS.
///
/// Wraps `WebAppRuntime` so that only level-editor–specific operations are
/// exposed across the WASM boundary.  The `wasm_bindgen` attribute is
/// required on both the struct *and* its `impl` block so that wasm-bindgen
/// can generate a JS class wrapper — without it on the struct, `start()`
/// couldn't return an `AppHandle` to JS.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct AppHandle {
    #[cfg(target_arch = "wasm32")]
    editor: LevelEditor<WebAppRuntime>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl AppHandle {
    /// Return the number of boxes in the default level-editor scene.
    pub fn box_count(&self) -> usize {
        self.editor.box_count()
    }

    /// Return the total mesh count in the default level-editor scene.
    pub fn mesh_count(&self) -> usize {
        self.editor.mesh_count()
    }

    /// Load a GLTF model from a URL.
    pub fn load_gltf(&self, url: &str) {
        self.editor.load_gltf(url);
    }
}

/// WASM entrypoint — called once from JS to boot the renderer and populate the
/// default scene. Returns an `AppHandle` that JS keeps alive for later calls
/// (e.g. `load_gltf`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start() -> AppHandle {
    let app = WebAppRuntime::new("main-worker", "#canvas0").unwrap();
    let editor = LevelEditor::from_app(app);
    AppHandle { editor }
}

// Re-export the web-worker entrypoint that the renderer framework requires.
// Without this the dedicated worker thread won't find its WASM init function.
// Only meaningful in the WASM build; on native the worker concept doesn't exist.
#[cfg(target_arch = "wasm32")]
renderer::export_worker_entrypoint!();

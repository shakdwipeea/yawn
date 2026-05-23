use gltf::Gltf;
use ultraviolet::{Mat4, Vec3};

use crate::renderer::{CommandContext, SyncCommand};

/// Interleaved vertex matching the GPU vertex layout.
///
/// Fields are ordered position→normal→uv to match the WGSL vertex attributes
/// without extra padding. `repr(C)` + bytemuck ensure we can cast a `&[Self]`
/// straight into a GPU buffer with no conversion.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InterleavedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Describes a single mesh as a range into the shared MeshBatch buffers.
///
/// Offsets are into the batch-wide vertex/index arrays so that multiple meshes
/// can share a single GPU upload while still being drawn individually with
/// their own model transforms.
#[derive(Debug)]
pub struct MeshDescriptor {
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,
    /// Column-major 4×4 transform, stored flat for direct GPU uniform upload.
    pub model_matrix: [f32; 16],
}

/// All parsed mesh data packed contiguously for cache-friendly access.
///
/// Vertices and indices are stored in flat arrays rather than per-mesh Vecs so
/// we can issue a single GPU buffer upload per batch. Each `MeshDescriptor`
/// records its slice into these arrays.
#[derive(Debug)]
pub struct MeshBatch {
    pub vertices: Vec<InterleavedVertex>,
    pub indices: Vec<u32>,
    pub meshes: Vec<MeshDescriptor>,
    /// Embedded WGSL source for the render pipeline that draws this batch.
    pub shader_source: String,
    /// Unique key used by `GpuResources` to cache/reuse the compiled pipeline.
    pub pipeline_key: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl ModelBounds {
    fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Grow the bounding box to include `point` (already in world space).
    fn include_point(&mut self, point: [f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(point[i]);
            self.max[i] = self.max[i].max(point[i]);
        }
    }

    /// Compute an (eye, target) pair that frames the entire model.
    ///
    /// The camera is placed slightly above and behind the centre so the default
    /// view gives a ¾-perspective rather than a flat front view. The small Y
    /// offset (5% of radius) avoids looking straight-on at ground-plane models,
    /// while the Z offset (25% of radius) keeps the model comfortably in frame.
    pub fn camera_frame(&self) -> ([f32; 3], [f32; 3]) {
        let center = [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ];

        let extent = [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ];
        // Use the bounding-sphere radius so the framing distance works
        // regardless of the model's aspect ratio.
        let radius =
            0.5 * (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt();
        // Clamp to 1.0 so tiny/degenerate models don't produce a near-zero offset.
        let radius = radius.max(1.0);

        let eye = [
            center[0],
            center[1] + radius * 0.05,
            center[2] + radius * 0.25,
        ];

        (eye, center)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("failed to fetch the model")]
    Http(#[from] reqwest::Error),

    #[error("failed to decode bytes")]
    GltfParse(#[from] gltf::Error),

    #[error("failed to load model")]
    LoadError,

    #[error("{0}")]
    Other(String),
}

/// Normalise GLTF tex-coords to `[f32; 2]`.
///
/// GLTF allows U8 and U16 quantised UVs to save file size; we expand them to
/// f32 here so the rest of the pipeline only deals with one format.
fn convert_tex_coords(tex_coords: gltf::mesh::util::ReadTexCoords<'_>) -> Vec<[f32; 2]> {
    use gltf::mesh::util::ReadTexCoords;

    match tex_coords {
        ReadTexCoords::F32(iter) => iter.collect(),
        ReadTexCoords::U16(iter) => iter
            .map(|[u, v]| [u as f32 / u16::MAX as f32, v as f32 / u16::MAX as f32])
            .collect(),
        ReadTexCoords::U8(iter) => iter
            .map(|[u, v]| [u as f32 / u8::MAX as f32, v as f32 / u8::MAX as f32])
            .collect(),
    }
}

/// Widen GLTF indices to u32 for a uniform index buffer format on the GPU.
fn convert_indices(indices: gltf::mesh::util::ReadIndices<'_>) -> Vec<u32> {
    use gltf::mesh::util::ReadIndices;

    match indices {
        ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
        ReadIndices::U16(iter) => iter.map(|i| i as u32).collect(),
        ReadIndices::U32(iter) => iter.collect(),
    }
}

/// Recursively walk the GLTF scene graph, accumulating world transforms.
///
/// Each node may carry a mesh (with multiple primitives) and child nodes.
/// We flatten the hierarchy into `batch` so the renderer can draw everything
/// with a single set of GPU buffers.
fn visit_node(
    node: gltf::Node<'_>,
    parent_transform: Mat4,
    batch: &mut MeshBatch,
    data_blob: &[u8],
    model_bounds: &mut Option<ModelBounds>,
) {
    // Accumulate the parent→child transform chain so every vertex ends up in
    // world space regardless of how deeply nested the node is.
    let local_transform = Mat4::from(node.transform().matrix());
    let world_transform = parent_transform * local_transform;

    // The normal matrix is the inverse-transpose of the model matrix. This
    // ensures non-uniform scales don't skew lighting normals.
    let normal_matrix = world_transform.inversed().transposed();

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            // The reader resolves buffer views against the embedded binary blob.
            // We only support GLB (single embedded buffer), so external URIs
            // return None and the primitive is silently skipped.
            let reader = primitive.reader(|buffer| match buffer.source() {
                gltf::buffer::Source::Bin => Some(&data_blob[..]),
                _ => None,
            });

            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(iter) => iter.collect(),
                None => Vec::new(),
            };

            // A primitive without positions is degenerate — nothing to render.
            if positions.is_empty() {
                continue;
            }

            let vertex_count = positions.len();

            // When the model has no normals we synthesise a default pointing up
            // (in world space) so the lighting shader still produces reasonable
            // output. The normal matrix is applied so the default tracks the
            // node's orientation.
            let default_normal_vec = normal_matrix.transform_vec3(Vec3::unit_y()).normalized();
            let default_normal = [
                default_normal_vec.x,
                default_normal_vec.y,
                default_normal_vec.z,
            ];

            // Transform normals into world space using the normal matrix.
            // If the normals array is shorter than positions (malformed file),
            // pad with the default to avoid panics during interleaving.
            let mut normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| {
                    iter.map(|normal| {
                        let vec = Vec3::new(normal[0], normal[1], normal[2]);
                        let transformed = normal_matrix.transform_vec3(vec).normalized();
                        [transformed.x, transformed.y, transformed.z]
                    })
                    .collect()
                })
                .unwrap_or_else(|| vec![default_normal; vertex_count]);

            if normals.len() != vertex_count {
                normals.resize(vertex_count, default_normal);
            }

            // Fall back to (0,0) UVs when the mesh has no texture coordinates,
            // keeping the vertex layout uniform for the shader.
            let mut uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(convert_tex_coords)
                .unwrap_or_else(|| vec![[0.0, 0.0]; vertex_count]);

            if uvs.len() != vertex_count {
                uvs.resize(vertex_count, [0.0, 0.0]);
            }

            // Accumulate world-space bounds across all primitives so we can
            // frame the camera after loading.
            for position in &positions {
                let vec = Vec3::new(position[0], position[1], position[2]);
                let transformed = world_transform.transform_point3(vec);
                let world_point = [transformed.x, transformed.y, transformed.z];
                if let Some(bounds) = model_bounds.as_mut() {
                    bounds.include_point(world_point);
                } else {
                    *model_bounds = Some(ModelBounds::new(world_point, world_point));
                }
            }

            // When no indices are provided, generate a trivial 0..N sequence so
            // the draw path can always use indexed drawing.
            let local_indices: Vec<u32> = reader
                .read_indices()
                .map(convert_indices)
                .unwrap_or_else(|| (0..vertex_count as u32).collect());

            if local_indices.is_empty() {
                continue;
            }

            // Snapshot offsets *before* appending so the descriptor points at
            // the correct start positions in the batch-wide arrays.
            let vertex_offset = batch.vertices.len() as u32;
            let index_offset = batch.indices.len() as u32;

            // Interleave position/normal/uv into a single vertex stream.
            for i in 0..vertex_count {
                batch.vertices.push(InterleavedVertex {
                    position: positions[i],
                    normal: normals[i],
                    uv: uvs[i],
                });
            }

            // Indices are stored with a global vertex_offset so they reference
            // the correct vertices within the batch-wide vertex array.
            for idx in &local_indices {
                batch.indices.push(vertex_offset + idx);
            }

            // Flatten the 4×4 matrix into a [f32; 16] for uniform upload.
            let model_matrix: [f32; 16] = {
                let s = world_transform.as_slice();
                let mut out = [0.0f32; 16];
                out.copy_from_slice(s);
                out
            };

            batch.meshes.push(MeshDescriptor {
                vertex_offset,
                vertex_count: vertex_count as u32,
                index_offset,
                index_count: local_indices.len() as u32,
                model_matrix,
            });
        }
    }

    // Recurse into children, propagating the accumulated world transform.
    for child in node.children() {
        visit_node(child, world_transform, batch, data_blob, model_bounds);
    }
}

const GLTF_SHADER_SOURCE: &str = include_str!("./gltf.wgsl");

/// Parse a GLB binary into a MeshBatch + optional bounds. Pure CPU, no GPU deps.
pub fn parse_gltf_from_bytes(
    glb_data: &[u8],
    pipeline_key: &str,
) -> Result<(MeshBatch, Option<ModelBounds>), ImportError> {
    let model = Gltf::from_slice(glb_data)?;
    let data_blob = model.blob.as_ref().ok_or(ImportError::LoadError)?;

    let mut batch = MeshBatch {
        vertices: Vec::new(),
        indices: Vec::new(),
        meshes: Vec::new(),
        shader_source: GLTF_SHADER_SOURCE.to_string(),
        pipeline_key: pipeline_key.to_string(),
    };

    let mut model_bounds: Option<ModelBounds> = None;

    for scene in model.scenes() {
        for node in scene.nodes() {
            visit_node(
                node,
                Mat4::identity(),
                &mut batch,
                data_blob,
                &mut model_bounds,
            );
        }
    }

    Ok((batch, model_bounds))
}

/// Command to load a GLTF model from a URL.
#[derive(Debug)]
pub struct LoadGltfCommand {
    pub url: String,
    pub pipeline_name: String,
}

impl LoadGltfCommand {
    pub fn new(url: impl Into<String>, pipeline_name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            pipeline_name: pipeline_name.into(),
        }
    }

    /// Fetch and parse a GLTF model, returning a sync command when successful.
    pub async fn load(self) -> Option<LoadGltfResult> {
        let Self { url, pipeline_name } = self;

        match Self::fetch_and_parse(&url, &pipeline_name).await {
            Ok((batch, bounds)) => {
                let result = LoadGltfResult { batch, bounds };
                Some(result)
            }
            Err(e) => {
                log::error!("Failed to load GLTF from {}: {:?}", url, e);
                None
            }
        }
    }

    async fn fetch_and_parse(
        url: &str,
        pipeline_name: &str,
    ) -> Result<(MeshBatch, Option<ModelBounds>), ImportError> {
        let glb_data = reqwest::get(url).await?.bytes().await?;
        parse_gltf_from_bytes(&glb_data, pipeline_name)
    }
}

/// Result of a GLTF load, sent back to the renderer thread for GPU upload.
#[derive(Debug)]
pub struct LoadGltfResult {
    pub batch: MeshBatch,
    pub bounds: Option<ModelBounds>,
}

impl LoadGltfResult {
    /// Upload the batch to the GPU and add meshes to the scene.
    ///
    /// Each `MeshDescriptor` becomes its own draw call so that per-mesh model
    /// matrices are preserved. We de-interleave back into separate attribute
    /// buffers because the current `MeshBuilder` expects that layout.
    pub fn apply_to_scene(
        self,
        scene: &mut crate::renderer::scene::Scene,
        resources: &mut crate::renderer::GpuResources,
        ctx: &crate::renderer::RendererContext,
    ) {
        use crate::renderer::scene::{mesh_vertex_layout, MeshBuilder};

        let device = &ctx.device;
        let surface_format = ctx.surface_config.format;

        // Re-use a single compiled pipeline for every mesh in the batch.
        let pipeline_index = resources.get_or_create_pipeline(
            device,
            &self.batch.pipeline_key,
            &mesh_vertex_layout(),
            &self.batch.shader_source,
            surface_format,
        );

        for desc in &self.batch.meshes {
            let start = desc.vertex_offset as usize;
            let end = start + desc.vertex_count as usize;
            let verts = &self.batch.vertices[start..end];

            // De-interleave back into separate attribute arrays because
            // MeshBuilder currently creates one GPU buffer per attribute.
            let positions: Vec<[f32; 3]> = verts.iter().map(|v| v.position).collect();
            let normals: Vec<[f32; 3]> = verts.iter().map(|v| v.normal).collect();
            let uvs: Vec<[f32; 2]> = verts.iter().map(|v| v.uv).collect();

            let idx_start = desc.index_offset as usize;
            let idx_end = idx_start + desc.index_count as usize;
            // Batch indices are global (offset by vertex_offset). Subtract it
            // back so each mesh's indices start from 0, matching MeshBuilder's
            // expectation of per-mesh local indices.
            let indices: Vec<u32> = self.batch.indices[idx_start..idx_end]
                .iter()
                .map(|i| i - desc.vertex_offset)
                .collect();

            // Reconstruct the ultraviolet Mat4 from the flat [f32; 16].
            let m = desc.model_matrix;
            let model_matrix = ultraviolet::Mat4::new(
                ultraviolet::Vec4::new(m[0], m[1], m[2], m[3]),
                ultraviolet::Vec4::new(m[4], m[5], m[6], m[7]),
                ultraviolet::Vec4::new(m[8], m[9], m[10], m[11]),
                ultraviolet::Vec4::new(m[12], m[13], m[14], m[15]),
            );

            let mesh = MeshBuilder::default()
                .with_vertices(device, resources, &positions, &normals, &uvs)
                .with_indices(device, resources, &indices)
                .with_pipeline(pipeline_index)
                .with_model_matrix(device, resources, model_matrix)
                .build();

            scene.meshes.push(mesh);
        }

        // Adjust the camera to frame the loaded model. We derive near/far
        // planes from the bounding sphere radius so that depth precision is
        // reasonable regardless of model scale.
        if let Some(bounds) = self.bounds {
            let (eye, target) = bounds.camera_frame();

            let extent = [
                bounds.max[0] - bounds.min[0],
                bounds.max[1] - bounds.min[1],
                bounds.max[2] - bounds.min[2],
            ];
            let radius = 0.5
                * (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt();
            let radius = radius.max(1.0);

            // Near plane at 0.1% of radius keeps close geometry visible;
            // far plane at 4× radius avoids clipping distant parts.
            let near_plane = (radius * 0.001).max(0.1);
            let far_plane = (radius * 4.0).max(near_plane + 1.0);

            // Only widen the existing depth range — never shrink it — so
            // previously loaded models aren't clipped.
            let (current_near, current_far) = scene.cam.depth_range();
            let new_near = near_plane.min(current_near);
            let new_far = far_plane.max(current_far);
            scene.cam.set_depth_range(new_near, new_far);
            scene.cam.look_at(Vec3::from(eye), Vec3::from(target));
        }
    }
}

impl SyncCommand for LoadGltfResult {
    fn run(self: Box<Self>, cx: &mut CommandContext<'_>) {
        (*self).apply_to_scene(cx.scene, cx.resources, cx.context);
    }
}

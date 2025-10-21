use gltf::Gltf;
use ultraviolet::{Mat4, Vec3};
use wgpu::TextureFormat;

use crate::renderer::scene::{mesh_vertex_layout, MeshBuilder};

#[derive(Clone, Copy, Debug)]
pub struct ModelBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl ModelBounds {
    fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    fn include_point(&mut self, point: [f32; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(point[i]);
            self.max[i] = self.max[i].max(point[i]);
        }
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

fn convert_indices(indices: gltf::mesh::util::ReadIndices<'_>) -> Vec<u32> {
    use gltf::mesh::util::ReadIndices;

    match indices {
        ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
        ReadIndices::U16(iter) => iter.map(|i| i as u32).collect(),
        ReadIndices::U32(iter) => iter.collect(),
    }
}

fn visit_node<'a>(
    node: gltf::Node<'a>,
    parent_transform: Mat4,
    device: &wgpu::Device,
    resources: &mut crate::renderer::GpuResources,
    meshes: &mut Vec<crate::renderer::scene::Mesh>,
    data_blob: &[u8],
    pipeline_index: usize,
    model_bounds: &mut Option<ModelBounds>,
) {
    let local_transform = Mat4::from(node.transform().matrix());
    let world_transform = parent_transform * local_transform;
    let normal_matrix = world_transform.inversed().transposed();

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| match buffer.source() {
                gltf::buffer::Source::Bin => Some(&data_blob[..]),
                _ => None,
            });

            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(iter) => iter.collect(),
                None => Vec::new(),
            };

            if positions.is_empty() {
                continue;
            }

            let vertex_count = positions.len();

            let default_normal_vec = normal_matrix.transform_vec3(Vec3::unit_y()).normalized();
            let default_normal = [
                default_normal_vec.x,
                default_normal_vec.y,
                default_normal_vec.z,
            ];

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

            let mut uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(convert_tex_coords)
                .unwrap_or_else(|| vec![[0.0, 0.0]; vertex_count]);

            if uvs.len() != vertex_count {
                uvs.resize(vertex_count, [0.0, 0.0]);
            }

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

            let indices: Vec<u32> = reader
                .read_indices()
                .map(convert_indices)
                .unwrap_or_else(|| (0..vertex_count as u32).collect());

            if indices.is_empty() {
                continue;
            }

            let mesh = MeshBuilder::new()
                .with_vertices(device, resources, &positions, &normals, &uvs)
                .with_indices(device, resources, &indices)
                .with_pipeline(pipeline_index)
                .with_model_matrix(device, resources, world_transform)
                .build();

            meshes.push(mesh);
        }
    }

    for child in node.children() {
        visit_node(
            child,
            world_transform,
            device,
            resources,
            meshes,
            data_blob,
            pipeline_index,
            model_bounds,
        );
    }
}

pub async fn load_gltf_model(
    device: &wgpu::Device,
    resources: &mut crate::renderer::GpuResources,
    meshes: &mut Vec<crate::renderer::scene::Mesh>,
    surface_format: TextureFormat,
) -> Result<Option<ModelBounds>, ImportError> {
    let glb_data = reqwest::get("http://localhost:8080/themanor.glb")
        .await?
        .bytes()
        .await?;

    let model = Gltf::from_slice(&glb_data)?;
    let data_blob = model.blob.as_ref().ok_or(ImportError::LoadError)?;

    let vertex_layout = mesh_vertex_layout();

    let pipeline_index = resources.get_or_create_pipeline(
        device,
        "gltf_standard",
        &vertex_layout,
        include_str!("./gltf.wgsl"),
        surface_format,
    );

    let mut model_bounds: Option<ModelBounds> = None;

    for scene in model.scenes() {
        for node in scene.nodes() {
            visit_node(
                node,
                Mat4::identity(),
                device,
                resources,
                meshes,
                data_blob,
                pipeline_index,
                &mut model_bounds,
            );
        }
    }

    Ok(model_bounds)
}

use gltf::Gltf;
use log::info;
use wgpu::{util::DeviceExt, BufferUsages, PipelineCompilationOptions, TextureFormat};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("failed to fetch the model")]
    Http(#[from] reqwest::Error),

    #[error("failed to decode bytes")]
    GltfParse(#[from] gltf::Error),

    #[error("failed to load model")]
    LoadError,
}

struct GltfVertex {
    pos: [f32; 3],
    normals: [f32; 3],
    uv: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct AttributeIndex {
    pub offset: usize,
    pub length: usize,
}

impl GltfVertex {
    fn non_interleaved_layout() -> [wgpu::VertexBufferLayout<'static>; 3] {
        [
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                }],
            },
        ]
    }
}

pub struct NonInterleavedRenderInfo {
    pub vertex_buffers: Vec<wgpu::Buffer>,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub index_count: u32,
    pub index_format: wgpu::IndexFormat,
}

impl NonInterleavedRenderInfo {
    pub fn new(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        surface_format: TextureFormat,
        data_blob: Vec<u8>,
        attribute_slices: Vec<AttributeIndex>,
        indices: AttributeIndex,
        index_format: wgpu::IndexFormat,
    ) -> Result<Self, ImportError> {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./gltf.wgsl").into()),
        });

        let vertex_buffers = attribute_slices
            .iter()
            .enumerate()
            .map(|(i, slice)| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("gltf vertex buffer {}", i)),
                    contents: &data_blob[slice.offset..slice.offset + slice.length],
                    usage: BufferUsages::VERTEX,
                })
            })
            .collect();

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gltf index buffer"),
            contents: &data_blob.as_slice()[indices.offset..indices.offset + indices.length],
            usage: BufferUsages::INDEX,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gltf render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &GltfVertex::non_interleaved_layout(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let index_size = match index_format {
            wgpu::IndexFormat::Uint16 => 2,
            wgpu::IndexFormat::Uint32 => 4,
        };

        Ok(Self {
            vertex_buffers,
            index_buffer,
            pipeline,
            index_count: (indices.length / index_size) as u32,
            index_format,
        })
    }
}

pub async fn load_gltf_model(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    surface_format: TextureFormat,
) -> Result<NonInterleavedRenderInfo, ImportError> {
    let glb_data = reqwest::get("http://localhost:8080/cube.glb")
        .await?
        .bytes()
        .await?;

    let model = Gltf::from_slice(&glb_data)?;

    let data_blob = model.blob.as_ref().ok_or(ImportError::LoadError)?.clone();

    let primitives = model
        .scenes()
        .flat_map(|scene| scene.nodes())
        .filter_map(|node| node.mesh())
        .flat_map(|m| m.primitives());

    let mut attribute_slices = vec![
        AttributeIndex {
            offset: 0,
            length: 0
        };
        3
    ];

    for primitive in primitives.clone() {
        for (semantic, accessor) in primitive.attributes() {
            if let Some(view) = accessor.view() {
                let slice = AttributeIndex {
                    offset: view.offset(),
                    length: view.length(),
                };

                match semantic {
                    gltf::Semantic::Positions => attribute_slices[0] = slice,
                    gltf::Semantic::Normals => attribute_slices[1] = slice,
                    gltf::Semantic::TexCoords(0) => attribute_slices[2] = slice,
                    _ => {}
                }
            }
        }
    }

    let (indices, index_format) = primitives
        .clone()
        .filter_map(|primitive| primitive.indices())
        .filter_map(|indices| {
            indices.view().map(|view| {
                let format = match indices.data_type() {
                    gltf::accessor::DataType::U16 => wgpu::IndexFormat::Uint16,
                    gltf::accessor::DataType::U32 => wgpu::IndexFormat::Uint32,
                    _ => wgpu::IndexFormat::Uint16, // Default fallback
                };
                (
                    AttributeIndex {
                        offset: view.offset(),
                        length: view.length(),
                    },
                    format,
                )
            })
        })
        .last()
        .ok_or(ImportError::LoadError)?;

    info!("slices are {:?} {:?}", attribute_slices, indices);

    NonInterleavedRenderInfo::new(
        &device,
        &pipeline_layout,
        surface_format,
        data_blob,
        attribute_slices,
        indices,
        index_format,
    )
}

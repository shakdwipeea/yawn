use ultraviolet::Mat4;
use wgpu::util::DeviceExt;

use crate::{
    camera::Camera,
    renderer::{BufferIndex, GpuResources, Index, ModelMatrix, Normal, Position, UV},
};

pub struct UniformResource {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

/// Simple uniform data.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
pub struct FrameMetadata {
    pub mouse_move: [f32; 2],
    pub mouse_click: [f32; 2],
    pub resolution: [f32; 2],
    time: f32,
    _padding0: f32,
    pub camera_position: [f32; 4],
}

impl FrameMetadata {
    pub fn new(dimension: ultraviolet::Vec2) -> Self {
        FrameMetadata {
            resolution: dimension.into(),
            mouse_move: [std::f32::MIN, std::f32::MIN],
            mouse_click: [std::f32::MIN, std::f32::MIN],
            _padding0: 0.0,
            camera_position: [0.0, 0.0, 0.0, 1.0],
            ..Default::default()
        }
    }

    pub fn set_camera_position(&mut self, position: ultraviolet::Vec3) {
        self.camera_position = [position.x, position.y, position.z, 1.0];
    }

    pub fn update_dimension(&mut self, dimension: ultraviolet::Vec2) {
        self.resolution = dimension.into();
    }

    pub fn create_uniform_resource(self, device: &wgpu::Device) -> UniformResource {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frame metadata uniform buffer"),
            contents: bytemuck::cast_slice(&[self][..]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        UniformResource {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }
}

pub struct Mesh {
    pub pipeline_index: usize,
    pub position_buffer_index: BufferIndex<Position>,
    pub normal_buffer_index: BufferIndex<Normal>,
    pub uv_buffer_index: BufferIndex<UV>,
    pub model_buffer_index: BufferIndex<ModelMatrix>,
    pub index_buffer_index: BufferIndex<Index>,
    pub index_format: wgpu::IndexFormat,
    pub index_count: u32,
    pub instance_count: u32,
}

type VertexBufferSet = (BufferIndex<Position>, BufferIndex<Normal>, BufferIndex<UV>);
type IndexBufferInfo = (BufferIndex<Index>, u32, wgpu::IndexFormat);

pub fn mesh_vertex_layout() -> [wgpu::VertexBufferLayout<'static>; 4] {
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
        wgpu::VertexBufferLayout {
            array_stride: 64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        },
    ]
}

pub struct MeshBuilder<I, V, P, M> {
    indices: I,
    vertices: V,
    pipeline: P,
    model_matrix: M,
    instance_count: u32,
}

impl MeshBuilder<(), (), (), ()> {
    pub fn new() -> Self {
        Self {
            indices: (),
            vertices: (),
            pipeline: (),
            model_matrix: (),
            instance_count: 1,
        }
    }
}

impl<P, M> MeshBuilder<(), (), P, M> {
    pub fn with_vertices(
        self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        uvs: &[[f32; 2]],
    ) -> MeshBuilder<(), VertexBufferSet, P, M> {
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let normal_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Normals"),
            contents: bytemuck::cast_slice(normals),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let uv_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh UVs"),
            contents: bytemuck::cast_slice(uvs),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let position_buffer_index = resources.add_position_buffer(position_buffer);
        let normal_buffer_index = resources.add_normal_buffer(normal_buffer);
        let uv_buffer_index = resources.add_uv_buffer(uv_buffer);

        MeshBuilder {
            vertices: (position_buffer_index, normal_buffer_index, uv_buffer_index),
            indices: self.indices,
            pipeline: self.pipeline,
            model_matrix: self.model_matrix,
            instance_count: self.instance_count,
        }
    }
}

impl<V, P, M> MeshBuilder<(), V, P, M> {
    pub fn with_indices(
        self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        indices: &[u32],
    ) -> MeshBuilder<IndexBufferInfo, V, P, M> {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let index_buffer_index = resources.add_index_buffer(index_buffer);

        MeshBuilder {
            indices: (
                index_buffer_index,
                indices.len() as u32,
                wgpu::IndexFormat::Uint32,
            ),
            vertices: self.vertices,
            pipeline: self.pipeline,
            model_matrix: self.model_matrix,
            instance_count: self.instance_count,
        }
    }
}

impl<I, V, M> MeshBuilder<I, V, (), M> {
    pub fn with_pipeline(self, pipeline_index: usize) -> MeshBuilder<I, V, usize, M> {
        MeshBuilder {
            pipeline: pipeline_index,
            indices: self.indices,
            vertices: self.vertices,
            model_matrix: self.model_matrix,
            instance_count: self.instance_count,
        }
    }
}

impl<I, V, P> MeshBuilder<I, V, P, ()> {
    pub fn with_model_matrix(
        self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        matrix_columns: Mat4,
    ) -> MeshBuilder<I, V, P, BufferIndex<ModelMatrix>> {
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Model Matrix"),
            contents: bytemuck::cast_slice(matrix_columns.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let model_buffer_index = resources.add_model_matrix_buffer(model_buffer);

        MeshBuilder {
            indices: self.indices,
            vertices: self.vertices,
            pipeline: self.pipeline,
            model_matrix: model_buffer_index,
            instance_count: self.instance_count,
        }
    }
}

impl MeshBuilder<IndexBufferInfo, VertexBufferSet, usize, BufferIndex<ModelMatrix>> {
    pub fn build(self) -> Mesh {
        Mesh {
            pipeline_index: self.pipeline,
            position_buffer_index: (self.vertices).0,
            normal_buffer_index: (self.vertices).1,
            uv_buffer_index: (self.vertices).2,
            model_buffer_index: self.model_matrix,
            index_buffer_index: (self.indices).0,
            index_count: (self.indices).1,
            index_format: (self.indices).2,
            instance_count: self.instance_count,
        }
    }
}

pub struct SceneBuilder<F, C> {
    frame_metadata: F,
    camera: C,
    meshes: Vec<Mesh>,
}

impl SceneBuilder<(), ()> {
    pub fn new() -> Self {
        Self {
            frame_metadata: (),
            camera: (),
            meshes: Vec::new(),
        }
    }

    pub fn with_dimension(
        self,
        dimension: ultraviolet::Vec2,
    ) -> SceneBuilder<FrameMetadata, Camera> {
        let SceneBuilder { meshes, .. } = self;

        SceneBuilder {
            frame_metadata: FrameMetadata::new(dimension),
            camera: Camera::new(dimension.x / dimension.y),
            meshes,
        }
    }
}

impl<C> SceneBuilder<(), C> {
    pub fn with_frame_metadata(
        self,
        frame_metadata: FrameMetadata,
    ) -> SceneBuilder<FrameMetadata, C> {
        let SceneBuilder { camera, meshes, .. } = self;

        SceneBuilder {
            frame_metadata,
            camera,
            meshes,
        }
    }
}

impl<F> SceneBuilder<F, ()> {
    pub fn with_camera(self, camera: Camera) -> SceneBuilder<F, Camera> {
        let SceneBuilder {
            frame_metadata,
            meshes,
            ..
        } = self;

        SceneBuilder {
            frame_metadata,
            camera,
            meshes,
        }
    }
}

impl<F, C> SceneBuilder<F, C> {
    pub fn with_mesh(mut self, mesh: Mesh) -> Self {
        self.meshes.push(mesh);
        self
    }

    pub fn with_meshes<I>(mut self, meshes: I) -> Self
    where
        I: IntoIterator<Item = Mesh>,
    {
        self.meshes.extend(meshes);
        self
    }
}

impl SceneBuilder<FrameMetadata, Camera> {
    pub fn build(self, device: &wgpu::Device) -> Scene {
        let SceneBuilder {
            mut frame_metadata,
            camera,
            meshes,
        } = self;

        frame_metadata.set_camera_position(camera.position());

        let uniform_resource = frame_metadata.create_uniform_resource(device);
        let camera_resource = camera.create_uniform_resource(device);

        Scene {
            uniform_buffers: [uniform_resource.buffer, camera_resource.buffer],
            bind_groups: [uniform_resource.bind_group, camera_resource.bind_group],
            bind_group_layout: [
                uniform_resource.bind_group_layout,
                camera_resource.bind_group_layout,
            ],
            frame_metadata,
            cam: camera,
            meshes,
        }
    }
}

/// Simple vertex format.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

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

pub struct Scene {
    pub uniform_buffers: [wgpu::Buffer; 2],
    pub bind_groups: [wgpu::BindGroup; 2],
    pub bind_group_layout: [wgpu::BindGroupLayout; 2],
    pub frame_metadata: FrameMetadata,
    pub cam: Camera,
    pub meshes: Vec<Mesh>,
}

impl Scene {
    pub fn builder() -> SceneBuilder<(), ()> {
        SceneBuilder::new()
    }

    pub fn new(device: &wgpu::Device, dimension: ultraviolet::Vec2) -> Self {
        Scene::builder().with_dimension(dimension).build(device)
    }

    pub fn setup(
        &mut self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        surface_format: wgpu::TextureFormat,
    ) {
        self.create_default_scene(device, resources, surface_format);
    }

    

    pub fn resize(&mut self, width: f64, height: f64, _scale_factor: f64, queue: &wgpu::Queue) {
        if height.abs() <= f64::EPSILON {
            return;
        }

        let dimension = ultraviolet::Vec2::new(width as f32, height as f32);
        self.frame_metadata.update_dimension(dimension);
        self.cam.update_aspect_ratio(width as f32 / height as f32);

        queue.write_buffer(
            &self.uniform_buffers[0],
            0,
            bytemuck::cast_slice(&[self.frame_metadata][..]),
        );
        queue.write_buffer(
            &self.uniform_buffers[1],
            0,
            bytemuck::cast_slice(&[self.cam.view_proj]),
        );
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        let time = (js_sys::Date::now() as f32) * 0.001;
        self.frame_metadata.time = time;
        self.frame_metadata.set_camera_position(self.cam.position());

        queue.write_buffer(
            &self.uniform_buffers[0],
            0,
            bytemuck::cast_slice(&[self.frame_metadata][..]),
        );

        queue.write_buffer(
            &self.uniform_buffers[1],
            0,
            bytemuck::cast_slice(&[self.cam.view_proj]),
        );
    }

    

    pub fn create_default_scene(
        &mut self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        surface_format: wgpu::TextureFormat,
    ) {
        let positions: Vec<[f32; 3]> = VERTICES.iter().map(|v| v.pos).collect();
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
            include_str!("../gltf.wgsl"),
            surface_format,
        );

        let scale_factor = 100.0;
        let scale_matrix = Mat4::from_scale(scale_factor);

        let mesh = MeshBuilder::new()
            .with_vertices(device, resources, &positions, &normals, uvs)
            .with_indices(device, resources, INDICES)
            .with_pipeline(pipeline_index)
            .with_model_matrix(device, resources, scale_matrix)
            .build();

        self.meshes.push(mesh);
    }
}

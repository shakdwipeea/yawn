use wgpu::util::DeviceExt;

use crate::{
    camera::Camera,
    renderer::{BufferIndex, GpuResources, Index, Normal, Position, UV},
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
    pub index_buffer_index: BufferIndex<Index>,
    pub index_format: wgpu::IndexFormat,
    pub index_count: u32,
    pub instance_count: u32,
}

type VertexBufferSet = (BufferIndex<Position>, BufferIndex<Normal>, BufferIndex<UV>);
type IndexBufferInfo = (BufferIndex<Index>, u32, wgpu::IndexFormat);

pub fn mesh_vertex_layout() -> [wgpu::VertexBufferLayout<'static>; 3] {
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

pub struct MeshBuilder<I, V, P> {
    indices: I,
    vertices: V,
    pipeline: P,
    instance_count: u32,
}

impl MeshBuilder<(), (), ()> {
    pub fn new() -> Self {
        Self {
            indices: (),
            vertices: (),
            pipeline: (),
            instance_count: 1,
        }
    }
}

impl<P> MeshBuilder<(), (), P> {
    pub fn with_vertices(
        self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        uvs: &[[f32; 2]],
    ) -> MeshBuilder<(), VertexBufferSet, P> {
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
            instance_count: self.instance_count,
        }
    }
}

impl<V, P> MeshBuilder<(), V, P> {
    pub fn with_indices(
        self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        indices: &[u32],
    ) -> MeshBuilder<IndexBufferInfo, V, P> {
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
            instance_count: self.instance_count,
        }
    }
}

impl<I, V> MeshBuilder<I, V, ()> {
    pub fn with_pipeline(self, pipeline_index: usize) -> MeshBuilder<I, V, usize> {
        MeshBuilder {
            pipeline: pipeline_index,
            indices: self.indices,
            vertices: self.vertices,
            instance_count: self.instance_count,
        }
    }
}

impl MeshBuilder<IndexBufferInfo, VertexBufferSet, usize> {
    pub fn build(self) -> Mesh {
        Mesh {
            pipeline_index: self.pipeline,
            position_buffer_index: (self.vertices).0,
            normal_buffer_index: (self.vertices).1,
            uv_buffer_index: (self.vertices).2,
            index_buffer_index: (self.indices).0,
            index_count: (self.indices).1,
            index_format: (self.indices).2,
            instance_count: self.instance_count,
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

/// Triangle vertex data.
const VERTICES: &[Vertex] = &[
    Vertex {
        pos: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 1.0], // Magenta
    },
    Vertex {
        pos: [-0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0], // Blue
    },
    Vertex {
        pos: [0.5, -0.5, 0.0],
        color: [1.0, 1.0, 0.0], // Yellow
    },
];
const INDICES: &[u32] = &[0, 1, 2];

pub struct Scene {
    pub uniform_buffers: [wgpu::Buffer; 2],
    pub bind_groups: [wgpu::BindGroup; 2],
    pub bind_group_layout: [wgpu::BindGroupLayout; 2],
    pub frame_metadata: FrameMetadata,
    pub cam: Camera,
    pub meshes: Vec<Mesh>,
}

impl Scene {
    pub fn new(device: &wgpu::Device, dimension: ultraviolet::Vec2) -> Self {
        let cam = Camera::new(dimension.x / dimension.y);
        let mut frame_metadata = FrameMetadata::new(dimension);
        frame_metadata.set_camera_position(cam.position());

        let uniform_resource = frame_metadata.create_uniform_resource(device);
        let camera_resource = cam.create_uniform_resource(device);

        Scene {
            uniform_buffers: [uniform_resource.buffer, camera_resource.buffer],
            bind_groups: [uniform_resource.bind_group, camera_resource.bind_group],
            bind_group_layout: [
                uniform_resource.bind_group_layout,
                camera_resource.bind_group_layout,
            ],
            frame_metadata,
            cam,
            meshes: Vec::new(),
        }
    }

    pub fn create_default_triangle(
        &mut self,
        device: &wgpu::Device,
        resources: &mut GpuResources,
        surface_format: wgpu::TextureFormat,
    ) {
        let positions: Vec<[f32; 3]> = VERTICES.iter().map(|v| v.pos).collect();
        // Colors ride through the "normal" slot because the render path always binds
        // three vertex buffers (position, normal, uv) for every mesh.
        // todo clean that shit up
        let colors: Vec<[f32; 3]> = VERTICES.iter().map(|v| v.color).collect();
        let uvs: &[[f32; 2]] = &[[0.0, 0.0], [0.0, 1.0], [1.0, 0.0]];

        let vertex_layout = mesh_vertex_layout();

        let pipeline_index = resources.get_or_create_pipeline(
            device,
            "triangle_colored",
            &vertex_layout,
            include_str!("../example.wgsl"),
            surface_format,
        );

        let mesh = MeshBuilder::new()
            .with_vertices(device, resources, &positions, &colors, uvs)
            .with_indices(device, resources, INDICES)
            .with_pipeline(pipeline_index)
            .build();

        self.meshes.push(mesh);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, time: f32) {
        self.frame_metadata.time = time * 0.001;
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
}

use std::f32::consts::PI;

use wgpu::util::DeviceExt;

use crate::renderer::scene::UniformResource;

const MIN_DISTANCE: f32 = 0.1;
const MAX_PITCH: f32 = PI / 2.0 - 0.01;
const ORBIT_SENSITIVITY: f32 = 0.005;

#[repr(C)]
pub struct Camera {
    // Hot data - cached computed matrix (64 bytes, 1 cache line)
    pub view_proj: [[f32; 4]; 4],

    // Warm data - frequently accessed vectors (36 bytes)
    position: ultraviolet::Vec3,
    target: ultraviolet::Vec3,
    up: ultraviolet::Vec3,

    // Cold data - projection parameters (16 bytes)
    fov: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,

    // Spherical coordinates for orbit camera behaviour
    yaw: f32,
    pitch: f32,
    distance: f32,

    // Dirty flag for lazy evaluation
    dirty: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        let mut camera = Camera {
            view_proj: [[0.0; 4]; 4],
            position: ultraviolet::Vec3::new(0.0, 1.5, 0.0),
            target: ultraviolet::Vec3::zero(),
            up: ultraviolet::Vec3::unit_y(),
            fov: PI / 3.0,
            aspect_ratio,
            z_near: 0.1,
            z_far: 100000.0,
            yaw: 0.0,
            pitch: 0.0,
            distance: 1.0,
            dirty: true,
        };

        camera.update_spherical_from_position();
        camera.compute_view_proj_mat();

        camera
    }

    pub fn compute_view_proj_mat(&mut self) {
        let view = ultraviolet::Mat4::look_at(self.position, self.target, self.up);
        let proj = ultraviolet::projection::rh_yup::perspective_wgpu_dx(
            self.fov,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );
        self.view_proj = (proj * view).into();
        self.dirty = false;
    }

    pub fn look_at(&mut self, position: ultraviolet::Vec3, target: ultraviolet::Vec3) {
        self.position = position;
        self.target = target;
        self.up = ultraviolet::Vec3::unit_y();
        self.update_spherical_from_position();
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn set_depth_range(&mut self, z_near: f32, z_far: f32) {
        self.z_near = z_near;
        self.z_far = z_far.max(z_near + f32::EPSILON);
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn position(&self) -> ultraviolet::Vec3 {
        self.position
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * ORBIT_SENSITIVITY;
        self.pitch -= delta_y * ORBIT_SENSITIVITY;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        self.update_position_from_spherical();
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn create_uniform_resource(&self, device: &wgpu::Device) -> UniformResource {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "camera uniform buffer".into(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[self.view_proj]),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 1,
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
                binding: 1,
                resource: buffer.as_entire_binding(),
            }],
        });

        UniformResource {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

impl Camera {
    fn update_spherical_from_position(&mut self) {
        let offset = self.position - self.target;
        let distance = (offset.x * offset.x + offset.y * offset.y + offset.z * offset.z).sqrt();
        self.distance = distance.max(MIN_DISTANCE);

        // Avoid invalid values when the camera is extremely close to the target
        let inv_distance = if self.distance.abs() < f32::EPSILON {
            0.0
        } else {
            offset.y / self.distance
        };

        self.pitch = inv_distance.clamp(-1.0, 1.0).asin();
        self.yaw = offset.x.atan2(offset.z);
    }

    fn update_position_from_spherical(&mut self) {
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        let sin_yaw = self.yaw.sin();
        let cos_yaw = self.yaw.cos();

        let offset = ultraviolet::Vec3::new(
            self.distance * cos_pitch * sin_yaw,
            self.distance * sin_pitch,
            self.distance * cos_pitch * cos_yaw,
        );

        self.position = self.target + offset;
        self.up = ultraviolet::Vec3::unit_y();
    }
}

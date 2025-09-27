use std::f32::consts::PI;

use ultraviolet::{projection, Bivec3, Mat4, Rotor3, Vec3};
use wgpu::util::DeviceExt;

use crate::renderer::scene::UniformResource;

const MIN_DISTANCE: f32 = 0.1;
const MAX_PITCH: f32 = PI / 2.0 - 0.01;
const ORBIT_SENSITIVITY: f32 = 0.0005;

#[repr(C)]
pub struct Camera {
    // Hot data - cached computed matrix (64 bytes, 1 cache line)
    pub view_proj: [[f32; 4]; 4],

    // Warm data - frequently accessed vectors (36 bytes)
    position: Vec3,
    target: Vec3,
    up: Vec3,

    // Cold data - projection parameters (16 bytes)
    fov: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,

    // Rotor orientation + spherical coordinates for orbit camera behaviour
    rotor: Rotor3,
    distance: f32,
    yaw: f32,
    pitch: f32,

    // Dirty flag for lazy evaluation
    dirty: bool,
}

struct OrthonormalBasis {
    right: Vec3,
    up: Vec3,
    forward: Vec3,
}

impl OrthonormalBasis {
    pub fn new(right: Vec3, up: Vec3, forward: Vec3) -> Self {
        Self { right, up, forward }
    }

    pub fn from_camera(camera: &Camera) -> Self {
        let mut forward_offset = camera.target - camera.position;
        if forward_offset.mag_sq() <= f32::EPSILON {
            forward_offset = -Vec3::unit_z();
        }

        let forward = forward_offset.normalized();

        let mut right = forward.cross(camera.up);

        // Check if right vector is near zero (forward and up are parallel)
        if right.mag_sq() < 1e-10 {
            // Try alternate axes to find a valid right vector
            let alternate_axes = [Vec3::unit_y(), Vec3::unit_x()];
            for axis in alternate_axes.iter() {
                right = forward.cross(*axis);
                if right.mag_sq() >= 1e-10 {
                    break;
                }
            }
        }

        right = right.normalized();
        let up = right.cross(forward).normalized();

        Self::new(right, up, forward)
    }
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
            position: Vec3::new(0.0, 1.5, 0.0),
            target: Vec3::zero(),
            up: Vec3::unit_y(),
            fov: PI / 3.0,
            aspect_ratio,
            z_near: 0.1,
            z_far: 100000.0,
            rotor: Rotor3::identity(),
            distance: 1.0,
            yaw: 0.0,
            pitch: 0.0,
            dirty: true,
        };

        camera.compute_rotor();
        camera.compute_view_proj_mat();

        camera
    }

    pub fn compute_view_proj_mat(&mut self) {
        let view = Mat4::look_at(self.position, self.target, self.up);
        let proj = projection::rh_yup::perspective_wgpu_dx(
            self.fov,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        );
        self.view_proj = (proj * view).into();
        self.dirty = false;
    }

    pub fn look_at(&mut self, position: Vec3, target: Vec3) {
        self.position = position;
        self.target = target;
        self.up = Vec3::unit_y();
        self.compute_rotor();
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn set_depth_range(&mut self, z_near: f32, z_far: f32) {
        self.z_near = z_near;
        self.z_far = z_far.max(z_near + f32::EPSILON);
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let yaw_theta = delta_x * ORBIT_SENSITIVITY;
        let yaw_rotor =
            Rotor3::from_angle_plane(yaw_theta, Bivec3::from_normalized_axis(Vec3::unit_y()));

        let basis = OrthonormalBasis::from_camera(self);

        let desired_pitch = (self.pitch + delta_y * ORBIT_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
        let applied_pitch = desired_pitch - self.pitch;

        let pitch_rotor =
            Rotor3::from_angle_plane(applied_pitch, Bivec3::from_normalized_axis(basis.right));

        let orbit_rotor = (yaw_rotor * pitch_rotor).normalized();

        self.rotor = (orbit_rotor * self.rotor).normalized();

        let mut offset = self.position - self.target;
        if offset.mag_sq() <= f32::EPSILON {
            offset = Vec3::unit_z() * self.distance.max(MIN_DISTANCE);
        }

        orbit_rotor.rotate_vec(&mut offset);
        self.distance = offset.mag().max(MIN_DISTANCE);
        self.position = offset + self.target;

        self.yaw += yaw_theta;
        self.pitch = desired_pitch;

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

    fn compute_rotor(&mut self) {
        let offset = self.position - self.target;
        let distance = (offset.x * offset.x + offset.y * offset.y + offset.z * offset.z).sqrt();
        self.distance = distance.max(MIN_DISTANCE);

        // to compute the initial rotor we will do two rotations
        // these will orient the camera to the new coordinates
        //

        // but first we need the orthonormal basis for the current camera
        let basis = OrthonormalBasis::from_camera(self);

        // first rotation
        // this is the swing to make position face the target
        let camera_local_up = Vec3::unit_z();
        let swing_rotor = Rotor3::from_rotation_between(camera_local_up, -basis.forward);

        // now we need a twist rotor which aligns the camera up
        let mut up_after_swing = self.up.clone();
        swing_rotor.rotate_vec(&mut up_after_swing);

        // to rotate a vector by a rotor we need
        // - a bivector (represents the axis of rotation)
        // - angle of rotation
        let twist_axis = (-basis.forward).normalized();
        let twist_plane = Bivec3::from_normalized_axis(twist_axis);

        // Calculate twist angle between the up vectors:
        //            u1 × uc ⋅ (-f)
        // θ = atan2( ————————————— , u1 ⋅ uc )
        //              ‖u1 × uc‖
        //
        // Where:
        //   u1 = up vector after swing rotation
        //   uc = camera's current up vector
        //   f = forward vector (twist axis)
        let theta = up_after_swing
            .cross(self.up)
            .dot(twist_axis)
            .atan2(up_after_swing.dot(self.up));

        let twist_rotor = Rotor3::from_angle_plane(theta, twist_plane);

        self.rotor = (swing_rotor * twist_rotor).normalized();
    }
}

use std::f32::consts::PI;

use ultraviolet::{projection, Bivec3, Mat4, Rotor3, Vec3};
use wgpu::util::DeviceExt;

use crate::{message::WheelMessage, renderer::scene::UniformResource};

const MIN_DISTANCE: f32 = 0.1;
const MAX_PITCH: f32 = PI / 2.0 - 0.01;
const ORBIT_SENSITIVITY: f32 = 0.005;
const ZOOM_SENSITIVITY: f32 = 0.002;

#[repr(C)]
pub struct Camera {
    // Hot data - cached computed matrix (64 bytes, 1 cache line)
    pub view_proj: [[f32; 4]; 4],

    // Derived position (computed from yaw/pitch/distance + target)
    position: Vec3,
    // Orbit pivot point
    target: Vec3,

    // Cold data - projection parameters (16 bytes)
    fov: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,

    // Spherical coordinates for orbit (world-up preserving)
    yaw: f32,   // Rotation around world Y axis (radians)
    pitch: f32, // Tilt up/down from horizon (radians), clamped to avoid poles
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
            position: Vec3::zero(),
            target: Vec3::zero(),
            fov: PI / 3.0,
            aspect_ratio,
            z_near: 0.1,
            z_far: 100000.0,
            yaw: 0.0,
            pitch: 0.0,
            distance: 3.0,
            dirty: true,
        };

        camera.update_position_from_spherical();
        camera.compute_view_proj_mat();

        camera
    }

    /// Compute the orbit rotor from current yaw and pitch.
    /// Yaw rotates around world Y, then pitch tilts around the local right axis.
    fn orbit_rotor(&self) -> Rotor3 {
        let yaw_rotor =
            Rotor3::from_angle_plane(self.yaw, Bivec3::from_normalized_axis(Vec3::unit_y()));

        // Right axis after yaw rotation
        let mut right = Vec3::unit_x();
        yaw_rotor.rotate_vec(&mut right);

        let pitch_rotor = Rotor3::from_angle_plane(self.pitch, Bivec3::from_normalized_axis(right));

        // Compose: first yaw, then pitch
        (pitch_rotor * yaw_rotor).normalized()
    }

    /// Recompute position from spherical coordinates (yaw, pitch, distance) around target.
    fn update_position_from_spherical(&mut self) {
        let rotor = self.orbit_rotor();

        // Start with camera behind target along +Z axis
        let mut offset = Vec3::new(0.0, 0.0, self.distance);
        rotor.rotate_vec(&mut offset);

        self.position = self.target + offset;
    }

    pub fn compute_view_proj_mat(&mut self) {
        // World-up is always Y for the view matrix
        let view = Mat4::look_at(self.position, self.target, Vec3::unit_y());
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
        self.target = target;

        // Compute spherical coordinates from the given position
        let offset = position - target;
        self.distance = offset.mag().max(MIN_DISTANCE);

        // Extract yaw and pitch from offset direction
        let dir = offset / self.distance;

        // Pitch: angle from horizontal plane (asin of y component)
        self.pitch = dir.y.clamp(-1.0, 1.0).asin().clamp(-MAX_PITCH, MAX_PITCH);

        // Yaw: angle around Y axis from +Z axis
        self.yaw = dir.x.atan2(dir.z);

        self.update_position_from_spherical();
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn depth_range(&self) -> (f32, f32) {
        (self.z_near, self.z_far)
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

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        // Skip tiny movements to reduce unnecessary computations
        if delta_x.abs() < 0.001 && delta_y.abs() < 0.001 {
            return;
        }

        // Update yaw (horizontal rotation around world Y)
        // Negative because dragging right should rotate camera to the left (view rotates right)
        self.yaw -= delta_x * ORBIT_SENSITIVITY;

        // Update pitch (vertical tilt), clamped to avoid poles
        // Negative because dragging down should tilt camera up
        self.pitch -= delta_y * ORBIT_SENSITIVITY;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        self.update_position_from_spherical();
        self.dirty = true;
        self.compute_view_proj_mat();
    }

    pub fn zoom(&mut self, msg: &WheelMessage) {
        let mut delta = msg.delta_y as f32;

        // Match browser delta modes so the wheel delta is always roughly pixels.
        match msg.delta_mode {
            1 => delta *= 16.0,
            2 => delta *= 800.0,
            _ => {}
        }

        // Scrolling up should zoom in (reduce distance).
        delta = -delta;

        if delta.abs() <= f32::EPSILON {
            return;
        }

        // Scale dolly movement by current distance for consistent perceived zoom speed
        let dolly_amount = delta * ZOOM_SENSITIVITY * self.distance;

        // Move position toward/away from target by adjusting distance
        self.distance = (self.distance - dolly_amount).max(MIN_DISTANCE);

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
            label: Some("Camera bind group layout"),
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
            label: Some("Camera bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
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

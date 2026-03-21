use std::{collections::HashMap, marker::PhantomData, sync::mpsc::Sender};

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

use futures::channel::oneshot;
use log::info;
use ultraviolet::Vec4;

use crate::gltf::{load_gltf_model, ImportError, ModelBounds};

use crate::{
    message::{
        AsyncWindowEvent, MeshData, MouseMessage, ResizeMessage, SceneCommand, SyncWindowEvent,
        WheelMessage, WindowEvent,
    },
    renderer::surface::{SurfaceContext, WindowDimension},
};

pub mod scene;
pub mod surface;

// Re-export commonly used types
pub use scene::Mesh;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct GpuResources {
    // Core resources
    buffers: Vec<wgpu::Buffer>,
    pipelines: Vec<wgpu::RenderPipeline>,
    textures: Vec<wgpu::Texture>,

    // Layout management
    pipeline_layouts: Vec<wgpu::PipelineLayout>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,

    // Simple name-based pipeline lookup
    pipeline_registry: HashMap<String, usize>,

    // Shader modules cache
    shader_modules: HashMap<String, wgpu::ShaderModule>,
}

impl GpuResources {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            pipelines: Vec::new(),
            textures: Vec::new(),
            pipeline_layouts: Vec::new(),
            bind_group_layouts: Vec::new(),
            pipeline_registry: HashMap::new(),
            shader_modules: HashMap::new(),
        }
    }

    pub fn add_position_buffer(&mut self, buffer: wgpu::Buffer) -> BufferIndex<Position> {
        let index = self.buffers.len() as u32;
        self.buffers.push(buffer);
        BufferIndex {
            index,
            _buffer_type: PhantomData,
        }
    }

    pub fn add_normal_buffer(&mut self, buffer: wgpu::Buffer) -> BufferIndex<Normal> {
        let index = self.buffers.len() as u32;
        self.buffers.push(buffer);
        BufferIndex {
            index,
            _buffer_type: PhantomData,
        }
    }

    pub fn add_uv_buffer(&mut self, buffer: wgpu::Buffer) -> BufferIndex<UV> {
        let index = self.buffers.len() as u32;
        self.buffers.push(buffer);
        BufferIndex {
            index,
            _buffer_type: PhantomData,
        }
    }

    pub fn add_index_buffer(&mut self, buffer: wgpu::Buffer) -> BufferIndex<Index> {
        let index = self.buffers.len() as u32;
        self.buffers.push(buffer);
        BufferIndex {
            index,
            _buffer_type: PhantomData,
        }
    }

    pub fn add_model_matrix_buffer(&mut self, buffer: wgpu::Buffer) -> BufferIndex<ModelMatrix> {
        let index = self.buffers.len() as u32;
        self.buffers.push(buffer);
        BufferIndex {
            index,
            _buffer_type: PhantomData,
        }
    }

    #[inline(always)]
    pub fn get_buffer<T>(&self, id: &BufferIndex<T>) -> &wgpu::Buffer {
        &self.buffers[id.index as usize]
    }

    pub fn create_pipeline(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        vertex_layout: &[wgpu::VertexBufferLayout],
        shader_source: &str,
        surface_format: wgpu::TextureFormat,
    ) -> Result<usize, String> {
        if self.pipeline_registry.contains_key(name) {
            return Err(format!("Pipeline '{}' already exists", name));
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let layout = self.get_or_create_pipeline_layout(device, name);

        // Determine entry points based on pipeline name
        let (vertex_entry, fragment_entry) = match name {
            "triangle_colored" => ("v_main", "f_main"),
            _ => ("vs_main", "fs_main"),
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some(vertex_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: vertex_layout,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(fragment_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let index = self.pipelines.len();
        self.pipelines.push(pipeline);
        self.pipeline_registry.insert(name.to_string(), index);

        Ok(index)
    }

    pub fn get_pipeline(&self, name: &str) -> Option<usize> {
        self.pipeline_registry.get(name).copied()
    }

    pub fn get_or_create_pipeline(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        vertex_layout: &[wgpu::VertexBufferLayout],
        shader_source: &str,
        surface_format: wgpu::TextureFormat,
    ) -> usize {
        if let Some(index) = self.get_pipeline(name) {
            return index;
        }

        self.create_pipeline(device, name, vertex_layout, shader_source, surface_format)
            .expect(&format!("Failed to create pipeline '{}'", name))
    }

    pub fn get_pipeline_by_index(&self, index: usize) -> &wgpu::RenderPipeline {
        &self.pipelines[index]
    }

    pub fn set_bind_group_layouts(&mut self, layouts: &[wgpu::BindGroupLayout; 2]) {
        self.bind_group_layouts = layouts.to_vec();
    }

    fn get_or_create_pipeline_layout(
        &mut self,
        device: &wgpu::Device,
        label: &str,
    ) -> wgpu::PipelineLayout {
        if self.pipeline_layouts.is_empty() {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &self.bind_group_layouts.iter().collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });
            self.pipeline_layouts.push(layout);
        }
        self.pipeline_layouts[0].clone()
    }
}

impl Default for GpuResources {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferIndex<T> {
    pub index: u32,
    _buffer_type: PhantomData<T>,
}

impl<T> BufferIndex<T> {
    pub fn new(index: u32) -> Self {
        Self {
            index,
            _buffer_type: PhantomData,
        }
    }
}

// Kinds of buffers supported
pub struct Position;
pub struct Normal;
pub struct UV;
pub struct Index;
pub struct ModelMatrix;

pub struct RendererContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
}

pub struct Renderer {
    surface_size: WindowDimension,
    context: RendererContext,
    resources: GpuResources,
    scene: scene::Scene,
    sender: Sender<WindowEvent>,
}

impl Renderer {
    fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

    fn recreate_depth_texture(&mut self) {
        let (texture, view) =
            Self::create_depth_texture(&self.context.device, &self.context.surface_config);
        self.context.depth_texture = texture;
        self.context.depth_view = view;
    }

    pub async fn new(surface_context: SurfaceContext, sender: Sender<WindowEvent>) -> Self {
        let SurfaceContext { target, size } = surface_context;
        let id = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&id);
        let surface = instance.create_surface(target).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .unwrap();

        info!("Adapter info: {:?}", adapter.get_info());
        info!("Adapter features: {:?}", adapter.features());
        info!("Adapter limits: {:?}", adapter.limits());

        let descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::default(),
        };

        let (device, queue) = adapter.request_device(&descriptor).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        info!(
            "suface size: {} x {}",
            surface_config.width, surface_config.height
        );
        surface.configure(&device, &surface_config);

        let (depth_texture, depth_view) = Self::create_depth_texture(&device, &surface_config);

        let mut resources = GpuResources::new();
        let context = RendererContext {
            surface,
            device,
            queue,
            surface_config,
            depth_texture,
            depth_view,
        };

        let scene = scene::Scene::new(&context, &mut resources);

        Self {
            surface_size: size,
            context,
            scene,
            resources,
            sender,
        }
    }

    pub fn render(&mut self, time: f32) {
        self.scene.update(&self.context, &mut self.resources, time);

        let surface_texture = self.context.surface.get_current_texture().unwrap();
        let texture_view = surface_texture.texture.create_view(&Default::default());
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render command encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.context.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            for (i, bind_group) in self.scene.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, bind_group, &[]);
            }

            for mesh in self.scene.meshes.iter() {
                render_pass.set_pipeline(self.resources.get_pipeline_by_index(mesh.pipeline_index));

                render_pass.set_vertex_buffer(
                    0,
                    self.resources
                        .get_buffer(&mesh.position_buffer_index)
                        .slice(..),
                );
                render_pass.set_vertex_buffer(
                    1,
                    self.resources
                        .get_buffer(&mesh.normal_buffer_index)
                        .slice(..),
                );
                render_pass.set_vertex_buffer(
                    2,
                    self.resources.get_buffer(&mesh.uv_buffer_index).slice(..),
                );
                render_pass.set_vertex_buffer(
                    3,
                    self.resources
                        .get_buffer(&mesh.model_buffer_index)
                        .slice(..),
                );

                render_pass.set_index_buffer(
                    self.resources
                        .get_buffer(&mesh.index_buffer_index)
                        .slice(..),
                    mesh.index_format,
                );

                render_pass.draw_indexed(0..mesh.index_count, 0, 0..mesh.instance_count);
            }
        }
        self.context.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub async fn read_pixel_from_texture(&self, x: u32, y: u32) -> Vec4 {
        let width = self.context.depth_texture.width();
        let height = self.context.depth_texture.height();

        if width == 0 || height == 0 {
            log::warn!("Depth texture has zero extent ({} x {})", width, height);
            return Vec4::zero();
        }

        // Validate coordinates
        if x >= width || y >= height {
            log::warn!(
                "Pixel coordinates ({}, {}) out of bounds for texture size {}x{}",
                x,
                y,
                width,
                height
            );
            return Vec4::zero();
        }

        let pixel_size = std::mem::size_of::<f32>() as u32;
        let unpadded_row_bytes = width * pixel_size;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = if unpadded_row_bytes % align == 0 {
            unpadded_row_bytes
        } else {
            (unpadded_row_bytes / align + 1) * align
        };
        let buffer_size = padded_row_bytes as u64 * height as u64;
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("depth pixel read buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy just the single pixel
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("copy depth pixel to buffer"),
                });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.context.depth_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and read the pixel
        let slice = buffer.slice(..);
        let (tx, rx) = oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        // Poll the device to process the mapping

        rx.await.unwrap().unwrap();
        let depth_value = {
            let data = slice.get_mapped_range();
            let row_pitch = padded_row_bytes as usize;
            let byte_offset = y as usize * row_pitch + x as usize * pixel_size as usize;
            let mut depth_bytes = [0u8; 4];
            depth_bytes.copy_from_slice(&data[byte_offset..byte_offset + 4]);
            f32::from_le_bytes(depth_bytes)
        };
        buffer.unmap();

        Vec4::new(depth_value, 0.0, 0.0, 0.0)
    }

    pub fn tick(
        renderer: &Rc<RefCell<Renderer>>,
        events_chan: &std::sync::mpsc::Receiver<WindowEvent>,
        time: f32,
    ) {
        if let Ok(mut r) = renderer.try_borrow_mut() {
            let mut events = Vec::new();
            while let Ok(event) = events_chan.try_recv() {
                events.push(event);
            }

            let mut coalesced_move: Option<MouseMessage> = None;

            for event in events {
                match event {
                    WindowEvent::Sync(SyncWindowEvent::PointerMove(msg)) => {
                        if let Some(ref mut prev) = coalesced_move {
                            prev.movement_x += msg.movement_x;
                            prev.movement_y += msg.movement_y;
                            prev.client_x = msg.client_x;
                            prev.client_y = msg.client_y;
                            prev.offset_x = msg.offset_x;
                            prev.offset_y = msg.offset_y;
                            prev.buttons = msg.buttons;
                        } else {
                            coalesced_move = Some(msg);
                        }
                    }
                    other => r.handle_event(renderer, other),
                }
            }

            if let Some(msg) = coalesced_move {
                let evt = WindowEvent::Sync(SyncWindowEvent::PointerMove(msg));
                r.handle_event(renderer, evt);
            }

            r.render(time);
        }
    }

    fn handle_event(&mut self, renderer: &Rc<RefCell<Renderer>>, event: WindowEvent) {
        match event {
            WindowEvent::Sync(sync_event) => {
                match sync_event {
                    SyncWindowEvent::PointerMove(msg) => {
                        self.mouse_move(msg);
                    }
                    SyncWindowEvent::Resize(msg) => {
                        self.resize(msg);
                    }
                    SyncWindowEvent::PointerClick(msg) => {
                        let x = (msg.offset_x * msg.scale_factor) as f32;
                        let y = (msg.offset_y * msg.scale_factor) as f32;
                        self.scene.frame_metadata.mouse_click = [x, y];
                        log::info!("clicked");
                    }
                    SyncWindowEvent::PointerWheel(msg) => {
                        let wheel_msg = WheelMessage {
                            scale_factor: 1.0,
                            delta_x: 0.0,
                            delta_y: msg.delta_y as f64,
                            delta_z: 0.0,
                            delta_mode: 1, // DOM_DELTA_LINE
                            client_x: 0.0,
                            client_y: 0.0,
                        };
                        self.scene.cam.zoom(&wheel_msg);
                    }
                    SyncWindowEvent::Keyboard(msg) => {
                        log::info!("Key event received: {:?}", msg);

                        if msg.key == "l" || msg.key == "L" {
                            Self::handle_async_event(renderer, AsyncWindowEvent::LoadGltf);
                        }
                    }
                    SyncWindowEvent::CameraOrbit(msg) => {
                        self.scene.cam.orbit(msg.delta_x, msg.delta_y);
                    }
                    SyncWindowEvent::CameraZoom(msg) => {
                        let wheel_msg = WheelMessage {
                            scale_factor: 1.0,
                            delta_x: 0.0,
                            delta_y: msg.delta as f64,
                            delta_z: 0.0,
                            delta_mode: 0,
                            client_x: 0.0,
                            client_y: 0.0,
                        };
                        self.scene.cam.zoom(&wheel_msg);
                    }
                    SyncWindowEvent::SceneCommand(cmd) => {
                        self.apply_scene_command(cmd);
                    }
                }
            }
            WindowEvent::Async(async_event) => {
                Self::handle_async_event(renderer, async_event);
            }
        }
    }

    fn apply_scene_command(&mut self, cmd: SceneCommand) {
        match cmd {
            SceneCommand::Clear => self.scene.clear_meshes(),
            SceneCommand::AddMesh(mesh_data) => {
                self.add_mesh(mesh_data);
            }
            SceneCommand::SetCameraLookAt { eye, target } => {
                self.scene.set_camera_look_at(eye, target);
            }
        }
    }

    fn add_mesh(&mut self, upload: MeshData) {
        let device = &self.context.device;
        let surface_format = self.context.surface_config.format;
        let (scene, resources) = (&mut self.scene, &mut self.resources);

        let pipeline_index = resources.get_or_create_pipeline(
            device,
            &upload.pipeline_key,
            &scene::mesh_vertex_layout(),
            &upload.shader_source,
            surface_format,
        );

        let m = upload.model_matrix;
        let model_matrix = ultraviolet::Mat4::new(
            ultraviolet::Vec4::new(m[0], m[1], m[2], m[3]),
            ultraviolet::Vec4::new(m[4], m[5], m[6], m[7]),
            ultraviolet::Vec4::new(m[8], m[9], m[10], m[11]),
            ultraviolet::Vec4::new(m[12], m[13], m[14], m[15]),
        );
        let mesh = scene::MeshBuilder::default()
            .with_vertices(
                device,
                resources,
                &upload.positions,
                &upload.normals,
                &upload.uvs,
            )
            .with_indices(device, resources, &upload.indices)
            .with_pipeline(pipeline_index)
            .with_model_matrix(device, resources, model_matrix)
            .build();

        scene.meshes.push(mesh);
    }

    fn resize(&mut self, msg: ResizeMessage) {
        let new_width = (msg.width * msg.scale_factor) as u32;
        let new_height = (msg.height * msg.scale_factor) as u32;
        if new_width != self.surface_size.width || new_height != self.surface_size.height {
            self.surface_size = WindowDimension::new(new_width, new_height);
            self.context.surface_config.width = new_width;
            self.context.surface_config.height = new_height;
            self.context
                .surface
                .configure(&self.context.device, &self.context.surface_config);
            self.recreate_depth_texture();

            self.scene.resize(
                new_width as f64,
                new_height as f64,
                msg.scale_factor,
                &self.context.queue,
            );

            info!(
                "Resized: ({}, {}), scale: {}",
                new_width, new_height, msg.scale_factor
            );
        }
    }

    pub fn mouse_move(&mut self, msg: MouseMessage) {
        if (msg.buttons & 0x04) != 0 {
            let delta_x = (msg.movement_x * msg.scale_factor) as f32;
            let delta_y = (msg.movement_y * msg.scale_factor) as f32;
            self.scene.cam.orbit(delta_x, delta_y);
        }
    }

    fn handle_async_event(renderer: &Rc<RefCell<Renderer>>, event: AsyncWindowEvent) {
        let sender = renderer.borrow().sender.clone();
        match event {
            AsyncWindowEvent::LoadGltf => {
                let r = renderer.clone();
                crate::task::execute_async_event(sender, async move {
                    if let Err(e) = Renderer::load_assets_async(r).await {
                        log::error!("Failed to load file: {:?}", e);
                    }
                    vec![]
                });
            }
        }
    }

    // currently this replaces everything, will need more sophisticated mechanisms later
    pub async fn load_assets_async(renderer: Rc<RefCell<Renderer>>) -> Result<(), ImportError> {
        let (device, surface_format) = {
            let r = renderer.borrow();
            (r.context.device.clone(), r.context.surface_config.format)
        };

        let mut meshes = Vec::new();

        // Don't clear the scene - add to existing content
        let mut r = renderer.borrow_mut();

        let bounds =
            load_gltf_model(&device, &mut r.resources, &mut meshes, surface_format).await?;

        for mesh in meshes {
            r.scene.meshes.push(mesh);
        }

        // Optional: Adjust camera to frame the newly added model
        if let Some(ModelBounds { min, max }) = bounds {
            let center = ultraviolet::Vec3::new(
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            );

            let extent = ultraviolet::Vec3::new(max[0] - min[0], max[1] - min[1], max[2] - min[2]);
            let radius =
                0.5 * (extent.x * extent.x + extent.y * extent.y + extent.z * extent.z).sqrt();
            let radius = radius.max(1.0);

            // set the camera position after load, so we are not disoriented
            let eye_offset = ultraviolet::Vec3::new(0.0, radius * 0.05, radius * 0.25);

            // Keep the near plane proportional to the model size to avoid
            // extreme depth ranges when loading very large assets
            let near_plane = (radius * 0.001).max(0.1);

            // The far plane must be far enough to cover the entire model.
            // Using a fixed upper clamp caused large models to be clipped
            // completely; relying on the model radius instead.
            let far_plane = (radius * 4.0).max(near_plane + 1.0);

            // Only expand the depth range, never shrink it. This prevents
            // loading a small model from clipping objects already in the scene.
            let (current_near, current_far) = r.scene.cam.depth_range();
            let new_near = near_plane.min(current_near);
            let new_far = far_plane.max(current_far);
            r.scene.cam.set_depth_range(new_near, new_far);
            r.scene.cam.look_at(center + eye_offset, center);
        }

        Ok(())
    }
}

impl<T> From<BufferIndex<T>> for u32 {
    fn from(value: BufferIndex<T>) -> Self {
        value.index
    }
}

use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc, sync::mpsc::Receiver};

use log::info;
use wasm_bindgen::{prelude::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::DedicatedWorkerGlobalScope;

use crate::{
    gltf::{load_gltf_model, ImportError, ModelBounds},
    message::{MouseMessage, ResizeMessage, WindowEvent},
    renderer::scene::Scene,
};

pub mod scene;

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

pub struct Renderer {
    canvas: web_sys::OffscreenCanvas,
    events_chan: Receiver<WindowEvent>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    scene: Scene,
    resources: GpuResources,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

    fn recreate_depth_texture(&mut self) {
        let (texture, view) = Self::create_depth_texture(&self.device, &self.surface_config);
        self.depth_texture = texture;
        self.depth_view = view;
    }

    pub async fn new(canvas: web_sys::OffscreenCanvas, events_chan: Receiver<WindowEvent>) -> Self {
        let id = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&id);
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone()))
            .unwrap();
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
            width: canvas.clone().width(),
            height: canvas.clone().height(),
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

        let mut scene = Scene::new(
            &device,
            ultraviolet::Vec2::new(canvas.width() as f32, canvas.height() as f32),
        );

        resources.set_bind_group_layouts(&scene.bind_group_layout);
        scene.create_default_triangle(&device, &mut resources, surface_config.format);

        Self {
            canvas,
            events_chan,
            surface,
            device,
            queue,
            surface_config,
            scene,
            resources,
            depth_texture,
            depth_view,
        }
    }

    fn render(&mut self, time: f32) {
        self.scene.update(&self.queue, time);

        let surface_texture = self.surface.get_current_texture().unwrap();
        let texture_view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = self
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
                    view: &self.depth_view,
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

            for mesh in &self.scene.meshes {
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

                render_pass.set_index_buffer(
                    self.resources
                        .get_buffer(&mesh.index_buffer_index)
                        .slice(..),
                    mesh.index_format,
                );

                render_pass.draw_indexed(0..mesh.index_count, 0, 0..mesh.instance_count);
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub async fn handle_event(renderer: Rc<RefCell<Self>>, event: WindowEvent) {
        match event {
            WindowEvent::PointerMove(msg) => {
                renderer.borrow_mut().mouse_move(msg);
            }
            WindowEvent::Resize(msg) => {
                renderer.borrow_mut().resize(msg);
            }
            WindowEvent::PointerClick(msg) => {
                {
                    let mut r = renderer.borrow_mut();
                    let x = (msg.offset_x * msg.scale_factor) as f32;
                    let y = (msg.offset_y * msg.scale_factor) as f32;
                    r.scene.frame_metadata.mouse_click = [x, y];
                    log::info!("clicked");
                }
                if let Err(e) = Self::load_assets_async(renderer.clone()).await {
                    log::error!("failed to load gltf: {e}");
                }
            }
            WindowEvent::PointerWheel(msg) => {
                let mut r = renderer.borrow_mut();
                r.scene.cam.zoom(&msg);
            }
        }
    }

    pub fn run_render_loop(renderer: Rc<RefCell<Renderer>>) {
        let render_frame: Closure<dyn FnMut(f32)> = Closure::new(move |time: f32| {
            {
                let event = { renderer.borrow_mut().events_chan.try_recv() };

                if let Ok(event) = event {
                    let renderer_clone = renderer.clone();
                    spawn_local(async move {
                        Self::handle_event(renderer_clone, event).await;
                    });
                }
            }

            {
                let mut r = renderer.borrow_mut();
                r.render(time);
            }

            Self::run_render_loop(renderer.clone());
        });

        let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

        global
            .request_animation_frame(render_frame.as_ref().unchecked_ref())
            .unwrap();

        render_frame.forget();
    }

    fn resize(&mut self, msg: ResizeMessage) {
        let new_width = (msg.width * msg.scale_factor) as u32;
        let new_height = (msg.height * msg.scale_factor) as u32;
        if new_width != self.canvas.width() || new_height != self.canvas.height() {
            self.surface_config.width = new_width;
            self.surface_config.height = new_height;
            self.surface.configure(&self.device, &self.surface_config);
            self.recreate_depth_texture();

            self.scene.frame_metadata.resolution = [new_width as f32, new_height as f32];

            info!(
                "Resized: ({}, {}), scale: {}",
                new_width, new_height, msg.scale_factor
            );
        }
    }

    pub fn mouse_move(&mut self, msg: MouseMessage) {
        let x = (msg.offset_x * msg.scale_factor) as f32;
        let y = (msg.offset_y * msg.scale_factor) as f32;
        self.scene.frame_metadata.mouse_move = [x, y];

        if (msg.buttons & 0x04) != 0 {
            let delta_x = (msg.movement_x * msg.scale_factor) as f32;
            let delta_y = (msg.movement_y * msg.scale_factor) as f32;
            self.scene.cam.orbit(delta_x, delta_y);
        }
    }

    // currently this replaces everything, will need more sophisticated mechanisms later
    pub async fn load_assets_async(renderer: Rc<RefCell<Renderer>>) -> Result<(), ImportError> {
        let (device, surface_format, bind_group_layout) = {
            let r = renderer.borrow();
            (
                r.device.clone(),
                r.surface_config.format,
                r.scene.bind_group_layout.clone(),
            )
        };

        let mut meshes = Vec::new();

        let mut original_resources = {
            let mut r = renderer.borrow_mut();
            r.scene.meshes.clear();
            std::mem::take(&mut r.resources)
        };

        original_resources.set_bind_group_layouts(&bind_group_layout);

        let bounds = load_gltf_model(
            &device,
            &mut original_resources,
            &mut meshes,
            surface_format,
        )
        .await?;

        {
            let mut r = renderer.borrow_mut();
            r.resources = original_resources;
            r.scene.meshes = meshes;

            if let Some(ModelBounds { min, max }) = bounds {
                let center = ultraviolet::Vec3::new(
                    (min[0] + max[0]) * 0.5,
                    (min[1] + max[1]) * 0.5,
                    (min[2] + max[2]) * 0.5,
                );

                let extent =
                    ultraviolet::Vec3::new(max[0] - min[0], max[1] - min[1], max[2] - min[2]);
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
                r.scene.cam.set_depth_range(near_plane, far_plane);
                r.scene.cam.look_at(center + eye_offset, center);
            }
        }

        Ok(())
    }
}

impl<T> From<BufferIndex<T>> for u32 {
    fn from(value: BufferIndex<T>) -> Self {
        value.index
    }
}

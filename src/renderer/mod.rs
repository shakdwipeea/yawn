use std::{cell::RefCell, rc::Rc, sync::mpsc::Receiver};

use log::{error, info};
use wasm_bindgen::{prelude::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::DedicatedWorkerGlobalScope;
use wgpu::util::DeviceExt;

use crate::{
    gltf::{load_gltf_model, ImportError},
    message::{MouseMessage, ResizeMessage, WindowEvent},
};

/// Drawing relative data.
/// Note that this belongs to main worker.
pub struct Renderer {
    canvas: web_sys::OffscreenCanvas,
    events_chan: Receiver<WindowEvent>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    vertex_buffers: Vec<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
    index_num: u32,
    index_format: wgpu::IndexFormat,
    uniform_data: UniformData,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    pipeline_layout: wgpu::PipelineLayout,
}

impl Renderer {
    pub async fn new(canvas: web_sys::OffscreenCanvas, events_chan: Receiver<WindowEvent>) -> Self {
        let id = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        };

        // wgpu instance
        let instance = wgpu::Instance::new(&id);
        // wgpu surface
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone()))
            .unwrap();
        // wgpu adapter
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

        // wgpu device and queue
        let descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::default(),
        };

        let (device, queue) = adapter.request_device(&descriptor).await.unwrap();
        info!("after");
        // wgpu surface configuration
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
        // wgpu vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // wgpu index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        // wgpu uniform buffer
        let uniform_data = UniformData {
            resolution: [canvas.width() as f32, canvas.height() as f32],
            mouse_move: [std::f32::MIN, std::f32::MIN],
            mouse_click: [std::f32::MIN, std::f32::MIN],
            ..Default::default()
        };
        let (uniform_buffer, uniform_layout, uniform_bind_group) =
            Renderer::create_uniform_buffer(&device, bytemuck::cast_slice(&[uniform_data][..]));
        // wgpu shader module
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../example.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        // wgpu render pipeline
        let render_pipeline = Renderer::create_render_pipeline(
            &device,
            &pipeline_layout,
            &shader_module,
            &surface_config,
        );

        Self {
            canvas,
            events_chan,
            surface,
            device,
            queue,
            surface_config,
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            index_num: INDICES.len() as u32,
            index_format: wgpu::IndexFormat::Uint32,
            uniform_data,
            uniform_buffer,
            uniform_bind_group,
            render_pipeline,
            pipeline_layout,
        }
    }

    fn create_uniform_buffer(
        device: &wgpu::Device,
        contents: &[u8],
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform buffer"),
            contents,
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
        (buffer, bind_group_layout, bind_group)
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader_module: &wgpu::ShaderModule,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: shader_module,
                entry_point: Some("v_main"),
                buffers: &[Vertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: shader_module,
                entry_point: Some("f_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
    }

    fn render(&mut self, time: f32) {
        // Write uniform data to its buffer
        self.uniform_data.time = time * 0.001;
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform_data][..]),
        );

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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);

            for (i, buffer) in self.vertex_buffers.iter().enumerate() {
                render_pass.set_vertex_buffer(i as u32, buffer.slice(..));
            }

            render_pass.set_index_buffer(self.index_buffer.slice(..), self.index_format);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.draw_indexed(0..self.index_num, 0, 0..1);
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
                // Update uniform data synchronously
                {
                    let mut r = renderer.borrow_mut();
                    let x = (msg.offset_x * msg.scale_factor) as f32;
                    let y = (msg.offset_y * msg.scale_factor) as f32;
                    r.uniform_data.mouse_click = [x, y];
                    log::info!("clicked");
                }
                // Then async work without holding borrow
                if let Err(e) = Self::load_assets_async(renderer.clone()).await {
                    log::error!("failed to load gltf: {e}");
                }
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

    pub async fn load_assets(&mut self) -> Result<(), ImportError> {
        let render_info = load_gltf_model(
            &self.device,
            &self.pipeline_layout,
            self.surface_config.format,
        )
        .await?;

        self.vertex_buffers = render_info.vertex_buffers;
        self.index_buffer = render_info.index_buffer;
        self.render_pipeline = render_info.pipeline;
        self.index_num = render_info.index_count;
        self.index_format = render_info.index_format;

        Ok(())
    }

    fn resize(&mut self, msg: ResizeMessage) {
        let new_width = (msg.width * msg.scale_factor) as u32;
        let new_height = (msg.height * msg.scale_factor) as u32;
        if new_width != self.canvas.width() || new_height != self.canvas.height() {
            self.surface_config.width = new_width;
            self.surface_config.height = new_height;
            self.surface.configure(&self.device, &self.surface_config);

            // Update uniform data
            self.uniform_data.resolution = [new_width as f32, new_height as f32];

            info!(
                "Resized: ({}, {}), scale: {}",
                new_width, new_height, msg.scale_factor
            );
        }
    }

    pub fn mouse_move(&mut self, msg: MouseMessage) {
        // Update uniform data
        let x = (msg.offset_x * msg.scale_factor) as f32;
        let y = (msg.offset_y * msg.scale_factor) as f32;
        self.uniform_data.mouse_move = [x, y];
    }

    pub fn mouse_click(&mut self, msg: MouseMessage) {
        info!("clicked");
        // Update uniform data
        let x = (msg.offset_x * msg.scale_factor) as f32;
        let y = (msg.offset_y * msg.scale_factor) as f32;
        self.uniform_data.mouse_click = [x, y];
    }

    pub async fn load_assets_async(renderer: Rc<RefCell<Renderer>>) -> Result<(), ImportError> {
        // Take a short immutable borrow to clone what we need for async work
        let (device, pipeline_layout, surface_format) = {
            let r = renderer.borrow();
            (
                r.device.clone(),
                r.pipeline_layout.clone(),
                r.surface_config.format,
            )
        };

        // Perform async load without holding any RefCell borrow
        let render_info = load_gltf_model(&device, &pipeline_layout, surface_format).await?;

        // Install results with a short mutable borrow
        {
            let mut r = renderer.borrow_mut();
            r.vertex_buffers = render_info.vertex_buffers;
            r.index_buffer = render_info.index_buffer;
            r.render_pipeline = render_info.pipeline;
            r.index_num = render_info.index_count;
            r.index_format = render_info.index_format;
        }

        Ok(())
    }
}

/// Simple vertex format.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    // pos
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    // color
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Vertex example.
const VERTICES: &[Vertex] = &[
    Vertex {
        pos: [0.0, 0.5, 0.0],   // Top-left
        color: [1.0, 0.0, 1.0], // Magenta
    },
    Vertex {
        pos: [-0.5, -0.5, 0.0], // Bottom-left
        color: [0.0, 0.0, 1.0], // Blue
    },
    Vertex {
        pos: [0.5, -0.5, 0.0],  // Top-right
        color: [1.0, 1.0, 0.0], // Yellow
    },
];

const INDICES: &[u32] = &[0, 1, 2]; // CCW, quad

/// Simple uniform data.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Default)]
struct UniformData {
    mouse_move: [f32; 2],
    mouse_click: [f32; 2],
    resolution: [f32; 2],
    time: f32,
    _padding: f32,
}

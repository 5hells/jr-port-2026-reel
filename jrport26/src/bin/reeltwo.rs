use bytemuck::{Pod, Zeroable};
use commons::{HAXOR_FONT, bdf_to_curves, load_bdf};
use lazy_static::lazy_static;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    monitor,
    window::{self, Window, WindowAttributes},
};

struct Quote {
    text: &'static str,
    color: [f32; 4], // Changed to 4
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    params: [f32; 4], // [time, res_x, res_y, unused]
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    width: u32,
    height: u32,
    particles: Vec<Particle>,
    last_frame: Instant,
    time: f32,

    uniform_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    pipeline: Option<wgpu::RenderPipeline>,

    current_idx: usize,
    pts_a: Vec<(f32, f32)>,
    pts_b: Vec<(f32, f32)>,
}

impl App {
    fn new(width: u32, height: u32) -> Self {
        let mut particles = Vec::new();
        for _ in 0..3000 {
            particles.push(Particle {
                x: width as f32 / 2.0,
                y: height as f32 / 2.0,
                vx: 0.0,
                vy: 0.0,
            });
        }
        Self {
            window: None,
            pixels: None,
            width,
            height,
            particles,
            last_frame: Instant::now(),
            time: 0.0,
            uniform_buffer: None,
            bind_group: None,
            pipeline: None,
            current_idx: usize::MAX,
            pts_a: Vec::new(),
            pts_b: Vec::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Reel One")
                        .with_inner_size(LogicalSize::new(self.width, self.height))
                        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None))),
                )
                .unwrap(),
        );

        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        let mut pixels = Pixels::new(size.width, size.height, surface_texture).unwrap();

        let device = pixels.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("reel2.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms"),
            contents: bytemuck::bytes_of(&Globals {
                params: [self.time, self.width as f32, self.height as f32, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: pixels.render_texture_format(),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        self.uniform_buffer = Some(uniform_buffer);
        self.bind_group = Some(bind_group);
        self.pipeline = Some(pipeline);
        self.pixels = Some(pixels);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event,
                device_id,
                is_synthetic,
            } => {
                if let Key::Named(NamedKey::Escape) = event.logical_key {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                let dt = self.last_frame.elapsed().as_secs_f32();
                self.last_frame = Instant::now();
                self.time += dt;

                if let Some(pixels) = &mut self.pixels {
                    pixels
                        .queue()
                        .write_buffer(self.uniform_buffer.as_ref().unwrap(), 0, bytemuck::bytes_of(&Globals {
                            params: [self.time, self.width as f32, self.height as f32, 0.0],
                        }));

                    pixels
                        .render_with(|encoder, target, context| {
                            context.scaling_renderer.render(encoder, target);
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: target,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Load,
                                            store: wgpu::StoreOp::Store,
                                        },
                                        depth_slice: None,
                                    })],
                                    ..Default::default()
                                });
                            rpass.set_pipeline(self.pipeline.as_ref().unwrap());
                            rpass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
                            rpass.draw(0..3, 0..1);
                            Ok(())
                        })
                        .unwrap();
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

static MONITOR: &str = "eDP-1";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let d = display_info::DisplayInfo::from_name(MONITOR).unwrap();
    let mut app = App::new(d.width, d.height);
    event_loop.run_app(&mut app)?;
    Ok(())
}

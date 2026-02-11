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

lazy_static! {
    static ref HAXOR_CURVES: commons::Curves = bdf_to_curves(&load_bdf(HAXOR_FONT).unwrap());

    static ref QUOTES: Vec<Quote> = vec![
        Quote { text: "I miss you.", color: [0.4, 0.1, 0.2, 1.0] }, // Red/Purple
        Quote { text: "That is, if you miss me back.", color: [0.1, 0.3, 0.2, 1.0] }, // Teal
        Quote { text: "If you had cared for me, and I cared for you.", color: [0.3, 0.3, 0.1, 1.0] }, // Gold
        Quote { text: "But neither of us really did feel appreciation.", color: [0.2, 0.1, 0.4, 1.0] }, // Deep Violet
        Quote { text: "We instead felt mutual sadness in each-other.", color: [0.05, 0.05, 0.05, 1.0] }, // Faded Grey
        Quote { text: "I wish I truly did appreciate you.", color: [0.1, 0.2, 0.3, 1.0] }, // Blue
        Quote { text: "...and had helped you understand that you are perfect.", color: [0.2, 0.2, 0.2, 1.0] }, // Dark Grey
        Quote { text: "Because it's true. You are.", color: [0.3, 0.1, 0.1, 1.0] }, // Deep Red
        Quote { text: "And it was my fault I didn't bake it into your head.", color: [0.1, 0.3, 0.3, 1.0] }, // Cyan
        Quote { text: "Because I really do know you'll inevitably get to where you want to be.", color: [0.2, 0.2, 0.1, 1.0] }, // Olive
        Quote { text: "Where you need to be.", color: [0.1, 0.2, 0.1, 1.0] }, // Green
        Quote { text: "Somewhere you can feel safe.", color: [0.2, 0.1, 0.2, 1.0] }, // Purple
        Quote { text: "But that isn't here.", color: [0.1, 0.1, 0.3, 1.0] }, // Deep Blue
        Quote { text: "My fault.", color: [0.3, 0.3, 0.3, 1.0] }, // Light Grey
        Quote { text: "Stephen Hellings, 2025-2026 Jr. Portfolio", color: [0.1, 0.1, 0.1, 1.0] }, // Very Dark Grey
    ];
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    params: [f32; 4],      // [time, transition, res_x, res_y]
    color_a_old: [f32; 4], // The previous color A
    color_a_new: [f32; 4], // The target color A
    color_b_old: [f32; 4], // The previous color B
    color_b_new: [f32; 4], // The target color B
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

fn get_text_points(text: &str, width: u32, height: u32) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let scale = 3.0;
    let mut x_cursor = 50.0;
    let y_center = height as f32 / 2.0;

    for c in text.chars() {
        let codepoint = (c as u32).to_string();
        if let Some((_, dwidth, paths)) = HAXOR_CURVES.iter().find(|(enc, _, _)| enc == &codepoint)
        {
            for path in paths {
                for pt in path {
                    points.push((x_cursor + pt.0 * scale, y_center + pt.1 * scale));
                }
            }
            x_cursor += *dwidth as f32 * scale;
        }
    }
    points
}

fn expo_in_out(t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        t
    } else if t < 0.5 {
        0.5 * 2.0f32.powf(20.0 * t - 10.0)
    } else {
        -0.5 * 2.0f32.powf(-20.0 * t + 10.0) + 1.0
    }
}

static MONITOR: &str = "eDP-1";

fn render_particles(
    frame: &mut [u8],
    particles: &mut [Particle],
    pts_a: &[(f32, f32)],
    pts_b: &[(f32, f32)],
    w: u32,
    h: u32,
    time: f32,
    dt: f32,
) {
    let interval = 4.0;
    let t = (time % interval) / interval;

    let lerp_t = expo_in_out(t);

    let len_a = pts_a.len().max(1);
    let len_b = pts_b.len().max(1);

    let offset_y = 200.0;
    let offset_x = 0.0;

    for (i, p) in particles.iter_mut().enumerate() {
        let target_a = pts_a[i % len_a];
        let target_b = pts_b[i % len_b];

        let dest_x = target_a.0 * (1.0 - lerp_t) + target_b.0 * lerp_t;
        let dest_y = target_a.1 * (1.0 - lerp_t) + target_b.1 * lerp_t;

        let offset_dest_y = dest_y + offset_y;
        let offset_dest_x = dest_x + offset_x;

        let follow_speed = 12.0;
        p.x += (offset_dest_x - p.x) * (1.0 - (-follow_speed * dt).exp());
        p.y += (offset_dest_y - p.y) * (1.0 - (-follow_speed * dt).exp());

        let px = p.x as i32;
        let py = p.y as i32;
        if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
            let idx = ((py * w as i32 + px) * 4) as usize;
            if idx + 3 < frame.len() {
                frame[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
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
            source: wgpu::ShaderSource::Wgsl(include_str!("ps3.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms"),
            contents: bytemuck::bytes_of(&Globals {
                params: [0.0, 0.0, self.width as f32, self.height as f32],
                color_a_old: [0.0, 0.0, 0.0, 1.0],
                color_a_new: [0.0, 0.0, 0.0, 1.0],
                color_b_old: [0.0, 0.0, 0.0, 1.0],
                color_b_new: [0.0, 0.0, 0.0, 1.0],
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
                        alpha: wgpu::BlendComponent::OVER
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
                    let frame = pixels.frame_mut();

                    for byte in frame.iter_mut() {
                        *byte = (*byte as f32 * 0.75) as u8;
                    }

                    let interval = 4.0;
                    let idx = (self.time / interval) as usize % QUOTES.len();

                    if idx != self.current_idx {
                        self.current_idx = idx;

                        self.pts_a = get_text_points(QUOTES[idx].text, self.width, self.height);

                        self.pts_b = get_text_points(
                            QUOTES[(idx + 1) % QUOTES.len()].text,
                            self.width,
                            self.height,
                        );
                    }

                    let dt = self.last_frame.elapsed().as_secs_f32();

                    render_particles(
                        frame,
                        &mut self.particles,
                        &self.pts_a,
                        &self.pts_b,
                        self.width,
                        self.height,
                        self.time,
                        dt,
                    );

                    let t = (self.time % 4.0) / 4.0;
                    let smooth_t = t * t * (3.0 - 2.0 * t);

                    let idx = (self.time / 4.0) as usize % QUOTES.len();
                    let next_idx = (idx + 1) % QUOTES.len();

                    let color_a = QUOTES[idx].color;
                    let color_b = QUOTES[next_idx].color;

                    pixels.queue().write_buffer(
                        self.uniform_buffer.as_ref().unwrap(),
                        0,
                        bytemuck::bytes_of(&Globals {
                            params: [self.time, smooth_t, self.width as f32, self.height as f32],
                            color_a_old: if idx == 0 {
                                [0.0, 0.0, 0.0, 1.0]
                            } else {
                                QUOTES[(idx - 1) % QUOTES.len()].color
                            },
                            color_a_new: color_a,
                            color_b_old: if next_idx == 0 {
                                [0.0, 0.0, 0.0, 1.0]
                            } else {
                                QUOTES[(next_idx - 1) % QUOTES.len()].color
                            },
                            color_b_new: color_b,
                        }),
                    );

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let d = display_info::DisplayInfo::from_name(MONITOR).unwrap();
    let mut app = App::new(d.width, d.height);
    event_loop.run_app(&mut app)?;
    Ok(())
}

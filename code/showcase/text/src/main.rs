mod font;

use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
    path::Path,
};

use framework::{Display, resources::load_string, winit::keyboard::KeyCode};
use glam::{vec2, vec4};

use crate::font::{BitmapFont, FontBinder, TextPipeline};

struct TextDemo {
    sans_font: BitmapFont,
    medieval_font: BitmapFont,
    text_pipeline: TextPipeline,
    font_index: u32,
    sans_binding: font::FontBinding,
    medieval_binding: font::FontBinding,
    sans_text: font::TextBuffer,
    medieval_text: font::TextBuffer,
    camera: framework::Camera,
    camera_controller: framework::CameraController,
    camera_uniforms: framework::CameraUniform,
    camera_bind_group: wgpu::BindGroup,
    lmb_presssed: bool,
    projection: framework::Projection,
}

impl TextDemo {
    fn current_font(&self) -> &font::FontBinding {
        match self.font_index {
            0 => &self.sans_binding,
            _ => &self.medieval_binding,
        }
    }

    fn cycle_font(&mut self) {
        self.font_index += 1;
        if self.font_index >= 2 {
            self.font_index = 0;
        }
    }
}

impl std::fmt::Debug for TextDemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextDemo").finish()
    }
}

impl framework::Demo for TextDemo {
    async fn init(display: &Display, res_dir: &Path) -> anyhow::Result<Self> {
        // TODO: replace this with 2D camera
        let camera = framework::Camera::new(glam::vec3(10.0, 10.0, 10.0), -2.37, -0.5);
        let camera_controller = framework::CameraController::new(1.0, 0.01);
        let projection = framework::Projection::new(
            display.config.width,
            display.config.height,
            PI * 0.25,
            0.1,
            100.0,
        );
        let lmb_presssed = false;

        let mut camera_uniforms = framework::CameraUniform::new(&display.device);
        camera_uniforms.update_view_proj(&camera, &projection);

        let camera_layout =
            display
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
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

        let camera_bind_group = display
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("uniforms_bind_group"),
                layout: &camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniforms.buffer.as_entire_binding(),
                }],
            });

        let dialog_dir = res_dir.join("dialog");
        let dialog = load_string(dialog_dir.join("text-demo.txt")).await?;

        let fonts_dir = res_dir.join("fonts");
        let chars = HashSet::from_iter(dialog.chars());
        let padding = 4;
        let sans_font = BitmapFont::load(
            &display.device,
            &display.queue,
            padding,
            fonts_dir.join("Open_Sans/OpenSans-VariableFont_wdth,wght.ttf"),
            &chars,
        )
        .await?;
        let medieval_font = BitmapFont::load(
            &display.device,
            &display.queue,
            padding,
            fonts_dir.join("MedievalSharp/MedievalSharp-Regular.ttf"),
            &chars,
        )
        .await?;

        let font_sampler = display.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font_sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let font_binder = FontBinder::new(&display.device);
        let sans_binding = font_binder.bind(&display.device, &sans_font, &font_sampler);
        let medieval_binding = font_binder.bind(&display.device, &medieval_font, &font_sampler);

        let text_pipeline = TextPipeline::new(
            &display.device,
            display.config.format,
            &font_binder,
            &camera_layout,
        );

        let sans_text = text_pipeline.buffer_text(
            &display.device,
            &sans_font,
            &font_binder,
            &font_sampler,
            &dialog,
            vec2(10.0, 10.0),
            vec4(0.8, 0.9, 0.7, 1.0),
        );
        let medieval_text = text_pipeline.buffer_text(
            &display.device,
            &medieval_font,
            &font_binder,
            &font_sampler,
            &dialog,
            vec2(10.0, 10.0),
            vec4(0.8, 0.9, 0.7, 1.0),
        );

        Ok(TextDemo {
            sans_font,
            medieval_font,
            text_pipeline,
            font_index: 0,
            sans_binding,
            medieval_binding,
            sans_text,
            medieval_text,
            projection,
            camera,
            camera_controller,
            camera_uniforms,
            camera_bind_group,
            lmb_presssed,
        })
    }

    fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        self.camera_controller.process_keyboard(key, pressed);
        // match (key, pressed) {
        //     (KeyCode::Space, true) => self.cycle_font(),
        //     _ => {}
        // }
    }

    fn handle_mouse_button(&mut self, button: u32, pressed: bool) {
        if button == 0 {
            self.lmb_presssed = pressed;
        }
    }

    fn handle_mouse_move(&mut self, dx: f64, dy: f64) {
        self.camera_controller.process_mouse(dx, dy);
    }

    fn resize(&mut self, display: &Display) {
        self.projection.resize(display.width(), display.height());
    }

    fn update(&mut self, display: &Display, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
    }

    fn render(&mut self, display: &mut Display) {
        let frame = match display.surface().get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                display.configure();
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                display.configure();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => panic!("Surface lost"),
        };

        let view = frame.texture.create_view(&Default::default());

        let mut encoder = display.device.create_command_encoder(&Default::default());

        self.camera_uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.camera_uniforms
            .update_buffer(&display.device, &mut encoder);

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.text_pipeline
                .draw_text(&self.sans_text, &self.camera_bind_group, &mut pass);

            self.text_pipeline
                .debug_glyph_texture(&self.current_font(), &mut pass);
        }

        display.queue.submit([encoder.finish()]);
        frame.present();
    }
}

fn main() {
    framework::run::<TextDemo>().unwrap()
}

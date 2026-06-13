mod font;

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use framework::{Display, resources::load_string, winit::keyboard::KeyCode};

use crate::font::{BitmapFont, FontBinder, TextPipeline};

struct TextDemo {
    sans_font: BitmapFont,
    medieval_font: BitmapFont,
    text_pipeline: TextPipeline,
    font_index: u32,
    sans_binding: font::FontBinding,
    medieval_binding: font::FontBinding,
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

        let text_pipeline = TextPipeline::new(&display.device, display.config.format, &font_binder);

        Ok(TextDemo {
            sans_font,
            medieval_font,
            text_pipeline,
            font_index: 0,
            sans_binding,
            medieval_binding,
        })
    }

    fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        match (key, pressed) {
            (KeyCode::Space, true) => self.cycle_font(),
            _ => {}
        }
    }

    fn resize(&mut self, _display: &Display) {}

    fn update(&mut self, _display: &Display, _dt: std::time::Duration) {}

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
                .debug_glyph_texture(&self.current_font(), &mut pass);
        }

        display.queue.submit([encoder.finish()]);
        frame.present();
    }
}

fn main() {
    framework::run::<TextDemo>().unwrap()
}

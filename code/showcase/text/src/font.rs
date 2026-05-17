use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use ab_glyph::Font as _;
use framework::resources::load_binary;
use glam::{Vec2, vec2};
use wgpu::BlendState;

pub struct BitmapFont {
    glyphs: HashMap<char, Glyph>,
    texture: wgpu::TextureView,
}

impl BitmapFont {
    pub async fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: impl AsRef<Path>,
        chars: &HashSet<char>,
    ) -> anyhow::Result<Self> {
        let font_bytes = load_binary(path.as_ref()).await?;
        let font = ab_glyph::FontRef::try_from_slice(&font_bytes)?;

        // Figure out texture size
        let glyph_size = 64;
        let glyphs_per_row = chars.len().isqrt().next_power_of_two() as u32;
        let size = glyphs_per_row * glyph_size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{}", path.as_ref().display())),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let glyph_patch_size = glyph_size * glyph_size;
        let mut glyphs = HashMap::new();
        let mut x = 0;
        let mut y = 0;

        for c in chars.iter().copied() {
            let glyph = font.glyph_id(c).with_scale(glyph_size as f32);
            if let Some(outline) = font.outline_glyph(glyph) {
                let mut coverage = vec![0u8; glyph_patch_size as _];
                outline.draw(|x, y, c| {
                    coverage[(x + y * glyph_size) as usize] = (255.0 * c) as u8;
                });

                let bytes_per_row = coverage.len() as u32 / glyph_size;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfoBase {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x, y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &coverage,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(glyph_size),
                    },
                    wgpu::Extent3d {
                        width: glyph_size,
                        height: glyph_size,
                        depth_or_array_layers: 1,
                    },
                );

                let min = vec2(x as _, y as _);
                let max = min + vec2(glyph_size as _, glyph_size as _);
                let min_uv = min / texture.size().width as f32;
                let max_uv = max / texture.size().height as f32;

                glyphs.insert(
                    c,
                    Glyph {
                        min,
                        max,
                        min_uv,
                        max_uv,
                    },
                );

                x += glyph_size;

                // Maybe have the texture atlas be layered
                if x >= texture.size().width {
                    x = 0;
                    y += glyph_size;
                }
            }
        }

        Ok(Self {
            glyphs,
            texture: texture.create_view(&Default::default()),
        })
    }
}

pub struct Glyph {
    min: Vec2,
    max: Vec2,
    min_uv: Vec2,
    max_uv: Vec2,
}

pub struct FontBinder {
    layout: wgpu::BindGroupLayout,
}

impl FontBinder {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FontBinder"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        Self { layout }
    }

    pub fn bind(&self, device: &wgpu::Device, font: &BitmapFont, sampler: &wgpu::Sampler) -> FontBinding {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&font.texture),
                },
            ],
        });

        FontBinding { bind_group }
    }
}

pub struct FontBinding {
    bind_group: wgpu::BindGroup,
}

pub struct TextPipeline {
    debug: wgpu::RenderPipeline,
    render_format: wgpu::TextureFormat,
}

impl TextPipeline {
    pub fn new(
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
        font_binder: &FontBinder,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&font_binder.layout)],
            immediate_size: 0,
        });

        let debug = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TextPipeline::debug"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_glyph"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            debug,
            render_format,
        }
    }

    pub fn buffer_text(&self, device: &wgpu::Device, font: &BitmapFont, text: &str) -> TextBuffer {
        todo!()
    }

    pub fn debug_glyph_texture(&self, font: &FontBinding, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.debug);
        pass.set_bind_group(0, &font.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub struct TextBuffer {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

use std::collections::HashMap;

use anyhow::Result;

use crate::TextureId;
use crate::texture::Texture;

/// A loaded font with pre-rasterized glyph atlas.
pub struct Font {
    /// GPU texture containing all rasterized glyphs.
    pub atlas_texture_id: TextureId,
    /// Mapping from char → glyph info (UV rect in atlas).
    glyphs: HashMap<char, GlyphInfo>,
    /// Font size in pixels.
    pub size: f32,
    /// Line height.
    pub line_height: f32,
}

struct GlyphInfo {
    /// UV coordinates in atlas: [u, v, u_width, v_height]
    uv: [f32; 4],
    /// Pixel size of this glyph.
    width: f32,
    height: f32,
    /// Horizontal advance after this glyph.
    advance: f32,
    /// Y offset from baseline.
    y_offset: f32,
}

/// Characters to pre-rasterize into the atlas.
const CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 !@#$%^&*()-=+[]{}|;':\",./<>?_~`";

impl Font {
    /// Load a font from TTF/OTF bytes and rasterize a glyph atlas.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        font_bytes: &[u8],
        font_size: f32,
        texture_id: TextureId,
    ) -> Result<(Self, Texture)> {
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;

        // Rasterize all glyphs to get their sizes
        let mut rasterized: Vec<(char, fontdue::Metrics, Vec<u8>)> = Vec::new();
        for ch in CHARSET.chars() {
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            rasterized.push((ch, metrics, bitmap));
        }

        // Pack glyphs into atlas (simple row packing)
        let padding = 2;
        let atlas_width = 512u32;
        let mut atlas_height = 128u32;
        let mut row_x = padding as u32;
        let mut row_y = padding as u32;
        let mut row_height = 0u32;

        // First pass: calculate positions
        let mut positions: Vec<(u32, u32)> = Vec::new();
        for (_, metrics, _) in &rasterized {
            let gw = metrics.width as u32;
            let gh = metrics.height as u32;

            if row_x + gw + padding as u32 > atlas_width {
                row_y += row_height + padding as u32;
                row_x = padding as u32;
                row_height = 0;
            }

            positions.push((row_x, row_y));
            row_x += gw + padding as u32;
            row_height = row_height.max(gh);
        }
        atlas_height = atlas_height.max(row_y + row_height + padding as u32);
        // Round up to power of 2
        atlas_height = atlas_height.next_power_of_two();

        // Second pass: blit glyphs into RGBA atlas
        let mut atlas_data = vec![0u8; (atlas_width * atlas_height * 4) as usize];
        let mut glyphs = HashMap::new();

        let line_height = font_size * 1.2;

        for (i, (ch, metrics, bitmap)) in rasterized.iter().enumerate() {
            let (px, py) = positions[i];
            let gw = metrics.width;
            let gh = metrics.height;

            // Blit glyph bitmap (alpha channel) into RGBA atlas
            for gy in 0..gh {
                for gx in 0..gw {
                    let alpha = bitmap[gy * gw + gx];
                    let atlas_idx =
                        ((py + gy as u32) * atlas_width + (px + gx as u32)) as usize * 4;
                    atlas_data[atlas_idx] = 255; // R
                    atlas_data[atlas_idx + 1] = 255; // G
                    atlas_data[atlas_idx + 2] = 255; // B
                    atlas_data[atlas_idx + 3] = alpha; // A
                }
            }

            // Store UV info
            let u = px as f32 / atlas_width as f32;
            let v = py as f32 / atlas_height as f32;
            let u_w = gw as f32 / atlas_width as f32;
            let v_h = gh as f32 / atlas_height as f32;

            glyphs.insert(
                *ch,
                GlyphInfo {
                    uv: [u, v, u_w, v_h],
                    width: gw as f32,
                    height: gh as f32,
                    advance: metrics.advance_width,
                    y_offset: metrics.ymin as f32,
                },
            );
        }

        // Create GPU texture from atlas
        let size = wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font_atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas_width),
                rows_per_image: Some(atlas_height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("font_atlas_bind_group"),
        });

        let gpu_texture = Texture {
            texture,
            view,
            bind_group,
            width: atlas_width,
            height: atlas_height,
        };

        let font_obj = Font {
            atlas_texture_id: texture_id,
            glyphs,
            size: font_size,
            line_height,
        };

        Ok((font_obj, gpu_texture))
    }

    /// Measure the width of a text string in pixels.
    pub fn text_width(&self, text: &str) -> f32 {
        let mut width = 0.0f32;
        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                width += glyph.advance;
            }
        }
        width
    }

    /// Generate draw commands for text at position (x, y).
    /// Returns a list of (texture_id, src_rect, dst_rect) for each glyph.
    pub fn layout_text(&self, text: &str, x: f32, y: f32) -> Vec<(TextureId, [f32; 4], [f32; 4])> {
        let mut result = Vec::new();
        let mut cursor_x = x;

        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                if glyph.width > 0.0 && glyph.height > 0.0 {
                    let dst_y = y + self.size - glyph.height - glyph.y_offset;
                    result.push((
                        self.atlas_texture_id,
                        glyph.uv,
                        [cursor_x, dst_y, glyph.width, glyph.height],
                    ));
                }
                cursor_x += glyph.advance;
            }
        }

        result
    }
}

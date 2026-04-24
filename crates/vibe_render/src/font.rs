use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::TextureId;
use crate::texture::Texture;

/// Maximum atlas dimension. Grows as `256 → 512 → 1024 → 2048 → 4096`.
/// 4096×4096 RGBA = 64 MiB, enough for ~16k 32px CJK glyphs at single size.
const MAX_ATLAS_SIZE: u32 = 4096;
const INITIAL_ATLAS_SIZE: u32 = 256;
const GLYPH_PADDING: u32 = 2;

/// A loaded font that lazily rasterizes glyphs into a dynamically growing atlas.
///
/// Unlike a static prebaked atlas, this supports arbitrary characters (including
/// full CJK) by rasterizing on first use via [`Font::prepare_text`]. Glyphs not
/// yet prepared render as blank space (with cursor advancing the fallback width
/// so layout stays consistent with [`Font::text_width`]).
pub struct Font {
    /// Owned fontdue object — used for on-demand rasterization.
    font: fontdue::Font,
    /// GPU texture handle for this font's atlas (managed by AssetManager).
    pub atlas_texture_id: TextureId,
    /// Glyph cache (codepoint → atlas position + metrics).
    glyphs: HashMap<char, GlyphInfo>,
    /// Atlas dimensions (square, power of 2).
    atlas_size: u32,
    /// CPU-side atlas data (RGBA8). Kept around so we can re-upload after grow.
    atlas_data: Vec<u8>,
    /// Shelf packer state.
    packer: ShelfPacker,
    /// Tracks whether `atlas_data` has unsynced changes to upload.
    dirty: bool,
    /// Font size in pixels.
    pub size: f32,
    /// Line height for newline spacing.
    pub line_height: f32,
}

#[derive(Debug, Clone, Copy)]
struct GlyphInfo {
    /// UV rect in atlas: [u, v, u_w, v_h] (normalized 0..1).
    uv: [f32; 4],
    /// Pixel size of the rasterized glyph bitmap.
    width: f32,
    height: f32,
    /// Horizontal advance after this glyph (typographic).
    advance: f32,
    /// Y offset from baseline (fontdue convention).
    y_offset: f32,
}

/// Outcome of a [`Font::prepare_text`] call.
pub enum PrepareOutcome {
    /// Nothing changed — every requested glyph was already cached.
    NoChange,
    /// Existing atlas texture was updated in place via `queue.write_texture`.
    /// Caller does not need to swap textures.
    AtlasUpdated,
    /// Atlas was grown to a larger size; a brand-new GPU texture was created.
    /// Caller must replace the old texture in the asset registry.
    AtlasResized(Texture),
}

/// Simple shelf packer: glyphs are placed left-to-right; when a row fills,
/// move to the next row whose y = sum of previous row heights.
struct ShelfPacker {
    cursor_x: u32,
    cursor_y: u32,
    current_row_height: u32,
}

impl ShelfPacker {
    fn new() -> Self {
        Self {
            cursor_x: GLYPH_PADDING,
            cursor_y: GLYPH_PADDING,
            current_row_height: 0,
        }
    }

    /// Try to pack a `w × h` rectangle. Returns `Some((x, y))` on success,
    /// `None` if it doesn't fit in `atlas_size × atlas_size`.
    fn pack(&mut self, w: u32, h: u32, atlas_size: u32) -> Option<(u32, u32)> {
        if w == 0 || h == 0 {
            // Zero-area glyph (e.g. space) — return current cursor without advancing.
            return Some((self.cursor_x, self.cursor_y));
        }

        // Wrap to next row if doesn't fit horizontally.
        if self.cursor_x + w + GLYPH_PADDING > atlas_size {
            self.cursor_y += self.current_row_height + GLYPH_PADDING;
            self.cursor_x = GLYPH_PADDING;
            self.current_row_height = 0;
        }

        // Doesn't fit vertically either → packer full.
        if self.cursor_y + h + GLYPH_PADDING > atlas_size {
            return None;
        }

        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + GLYPH_PADDING;
        self.current_row_height = self.current_row_height.max(h);
        Some(pos)
    }

    /// Reset packer state (used when atlas grows; we re-pack everything from scratch).
    fn reset(&mut self) {
        self.cursor_x = GLYPH_PADDING;
        self.cursor_y = GLYPH_PADDING;
        self.current_row_height = 0;
    }
}

impl Font {
    /// Load a font from TTF/OTF bytes. The atlas starts empty;
    /// glyphs are rasterized on demand via [`Font::prepare_text`].
    ///
    /// Returns the [`Font`] alongside its initial empty GPU atlas texture,
    /// which the caller must register into the asset registry under
    /// `texture_id`.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        font_bytes: &[u8],
        font_size: f32,
        texture_id: TextureId,
    ) -> Result<(Self, Texture)> {
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|e| anyhow!("Failed to parse font: {}", e))?;

        let atlas_size = INITIAL_ATLAS_SIZE;
        let atlas_data = vec![0u8; (atlas_size * atlas_size * 4) as usize];
        let texture =
            create_atlas_texture(device, queue, bind_group_layout, atlas_size, &atlas_data);

        let line_height = font_size * 1.2;

        let mut font_obj = Font {
            font,
            atlas_texture_id: texture_id,
            glyphs: HashMap::new(),
            atlas_size,
            atlas_data,
            packer: ShelfPacker::new(),
            dirty: false,
            size: font_size,
            line_height,
        };

        // Pre-rasterize printable ASCII (0x20..=0x7E). This keeps the
        // "I just loaded a font and want to draw English text" path working
        // without forcing every game to call `prepare_text` for ASCII.
        // Pre-baked ASCII fits comfortably in the 256×256 initial atlas at
        // typical UI font sizes (~14–32px); CJK still arrives lazily later.
        font_obj.warm_ascii();
        // Upload the warmed atlas into the GPU texture we just created
        // so that glyphs are visible from frame 1 without needing a
        // subsequent `prepare_text` call for ASCII.
        if font_obj.dirty {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &font_obj.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * font_obj.atlas_size),
                    rows_per_image: Some(font_obj.atlas_size),
                },
                wgpu::Extent3d {
                    width: font_obj.atlas_size,
                    height: font_obj.atlas_size,
                    depth_or_array_layers: 1,
                },
            );
            font_obj.dirty = false;
        }

        Ok((font_obj, texture))
    }

    /// Pre-rasterize the printable ASCII range into the atlas. Used once at
    /// load time so English/digit/punctuation text needs no `prepare_text`
    /// call from the game.
    fn warm_ascii(&mut self) {
        for code in 0x20u32..=0x7Eu32 {
            // SAFETY: 0x20..=0x7E are all valid scalar values.
            let ch = char::from_u32(code).expect("ASCII range is valid");
            let (metrics, bitmap) = self.font.rasterize(ch, self.size);
            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            // ASCII at typical UI sizes always fits in the initial 256×256
            // atlas; if a user picks an absurdly huge font size we just skip
            // the overflowing glyphs (they'll fall back to blank-with-advance,
            // matching CJK behaviour).
            if let Some((px, py)) = self.packer.pack(gw, gh, self.atlas_size) {
                self.blit_glyph(px, py, gw, gh, &bitmap);
                self.glyphs.insert(
                    ch,
                    GlyphInfo {
                        uv: [
                            px as f32 / self.atlas_size as f32,
                            py as f32 / self.atlas_size as f32,
                            gw as f32 / self.atlas_size as f32,
                            gh as f32 / self.atlas_size as f32,
                        ],
                        width: gw as f32,
                        height: gh as f32,
                        advance: metrics.advance_width,
                        y_offset: metrics.ymin as f32,
                    },
                );
                self.dirty = true;
            }
        }
    }

    /// Ensure all characters in `text` have rasterized glyphs in the atlas,
    /// uploading any new pixel data to the existing GPU texture.
    ///
    /// Must be called before [`Font::layout_text`] / [`Font::text_width`]
    /// for any newly-encountered characters. Idempotent for cached chars.
    ///
    /// On atlas overflow the atlas is doubled in size and a fresh GPU texture
    /// is allocated; the new texture is returned via
    /// [`PrepareOutcome::AtlasResized`] and the caller must install it under
    /// `self.atlas_texture_id` in the asset registry.
    pub fn prepare_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        atlas_texture: &wgpu::Texture,
        text: &str,
    ) -> PrepareOutcome {
        // Collect distinct unseen chars first so we don't hash-lookup repeatedly.
        let mut new_chars: Vec<char> = text
            .chars()
            .filter(|ch| !self.glyphs.contains_key(ch))
            .collect();
        if new_chars.is_empty() {
            return PrepareOutcome::NoChange;
        }
        new_chars.sort_unstable();
        new_chars.dedup();

        let mut atlas_grew = false;

        for ch in new_chars {
            // Try to rasterize + pack; on packer overflow, grow atlas and retry.
            loop {
                let (metrics, bitmap) = self.font.rasterize(ch, self.size);
                let gw = metrics.width as u32;
                let gh = metrics.height as u32;

                match self.packer.pack(gw, gh, self.atlas_size) {
                    Some((px, py)) => {
                        self.blit_glyph(px, py, gw, gh, &bitmap);
                        self.glyphs.insert(
                            ch,
                            GlyphInfo {
                                uv: [
                                    px as f32 / self.atlas_size as f32,
                                    py as f32 / self.atlas_size as f32,
                                    gw as f32 / self.atlas_size as f32,
                                    gh as f32 / self.atlas_size as f32,
                                ],
                                width: gw as f32,
                                height: gh as f32,
                                advance: metrics.advance_width,
                                y_offset: metrics.ymin as f32,
                            },
                        );
                        self.dirty = true;
                        break;
                    }
                    None => {
                        if !self.grow_atlas() {
                            tracing::warn!(
                                "Font atlas reached maximum size {}×{}; \
                                glyph '{}' (U+{:04X}) skipped",
                                MAX_ATLAS_SIZE,
                                MAX_ATLAS_SIZE,
                                ch,
                                ch as u32
                            );
                            break;
                        }
                        atlas_grew = true;
                    }
                }
            }
        }

        if atlas_grew {
            // Allocate a new GPU texture of the new size; caller installs it.
            let new_texture = create_atlas_texture(
                device,
                queue,
                bind_group_layout,
                self.atlas_size,
                &self.atlas_data,
            );
            self.dirty = false;
            PrepareOutcome::AtlasResized(new_texture)
        } else if self.dirty {
            // In-place upload of the (possibly partially-modified) atlas.
            // We re-upload the whole CPU buffer for simplicity; the atlas is
            // CJK-sparse so this happens rarely after warmup.
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.atlas_size),
                    rows_per_image: Some(self.atlas_size),
                },
                wgpu::Extent3d {
                    width: self.atlas_size,
                    height: self.atlas_size,
                    depth_or_array_layers: 1,
                },
            );
            self.dirty = false;
            PrepareOutcome::AtlasUpdated
        } else {
            PrepareOutcome::NoChange
        }
    }

    fn blit_glyph(&mut self, px: u32, py: u32, gw: u32, gh: u32, bitmap: &[u8]) {
        for gy in 0..gh {
            for gx in 0..gw {
                let alpha = bitmap[(gy * gw + gx) as usize];
                let atlas_idx = (((py + gy) * self.atlas_size) + (px + gx)) as usize * 4;
                self.atlas_data[atlas_idx] = 255;
                self.atlas_data[atlas_idx + 1] = 255;
                self.atlas_data[atlas_idx + 2] = 255;
                self.atlas_data[atlas_idx + 3] = alpha;
            }
        }
    }

    /// Double the atlas size (up to `MAX_ATLAS_SIZE`) and re-pack all existing glyphs.
    /// Returns `false` if already at maximum.
    fn grow_atlas(&mut self) -> bool {
        let new_size = self.atlas_size * 2;
        if new_size > MAX_ATLAS_SIZE {
            return false;
        }

        self.atlas_data = vec![0u8; (new_size * new_size * 4) as usize];
        self.atlas_size = new_size;

        // Re-pack and re-blit every cached glyph by re-rasterizing.
        // This is simpler than memcpy from the old atlas (which would require
        // tracking each glyph's pixel rect) and just as correct — fontdue
        // rasterization is deterministic.
        let cached: Vec<char> = self.glyphs.keys().copied().collect();
        self.glyphs.clear();
        self.packer.reset();

        for ch in cached {
            let (metrics, bitmap) = self.font.rasterize(ch, self.size);
            let gw = metrics.width as u32;
            let gh = metrics.height as u32;
            if let Some((px, py)) = self.packer.pack(gw, gh, self.atlas_size) {
                self.blit_glyph(px, py, gw, gh, &bitmap);
                self.glyphs.insert(
                    ch,
                    GlyphInfo {
                        uv: [
                            px as f32 / self.atlas_size as f32,
                            py as f32 / self.atlas_size as f32,
                            gw as f32 / self.atlas_size as f32,
                            gh as f32 / self.atlas_size as f32,
                        ],
                        width: gw as f32,
                        height: gh as f32,
                        advance: metrics.advance_width,
                        y_offset: metrics.ymin as f32,
                    },
                );
            }
        }
        true
    }

    /// Measure the rendered width of `text`. Characters not yet in the atlas
    /// fall back to half the font size as advance — same fallback used by
    /// [`Font::layout_text`], so the values agree.
    pub fn text_width(&self, text: &str) -> f32 {
        let mut width = 0.0f32;
        let fallback_advance = self.size * 0.5;
        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                width += glyph.advance;
            } else {
                width += fallback_advance;
            }
        }
        width
    }

    /// Generate per-glyph draw data for `text` at position `(x, y)`.
    /// Glyphs not yet cached are rendered as blank space (cursor still advances
    /// by the fallback width so layout stays consistent with `text_width`).
    /// Call [`Font::prepare_text`] beforehand to populate the atlas.
    pub fn layout_text(&self, text: &str, x: f32, y: f32) -> Vec<(TextureId, [f32; 4], [f32; 4])> {
        let mut result = Vec::new();
        let mut cursor_x = x;
        let fallback_advance = self.size * 0.5;

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
            } else {
                cursor_x += fallback_advance;
            }
        }

        result
    }

    /// Whether a character is already cached in the atlas (for tests).
    pub fn has_glyph(&self, ch: char) -> bool {
        self.glyphs.contains_key(&ch)
    }

    /// Current atlas dimension in pixels (square, power of 2).
    pub fn atlas_size(&self) -> u32 {
        self.atlas_size
    }
}

/// Allocate a fresh `size × size` RGBA8 atlas texture with the given pixel data.
fn create_atlas_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    size: u32,
    data: &[u8],
) -> Texture {
    let extent = wgpu::Extent3d {
        width: size,
        height: size,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("font_atlas"),
        size: extent,
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
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * size),
            rows_per_image: Some(size),
        },
        extent,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Linear filtering avoids the blocky look on CJK glyphs at non-1:1 scales.
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
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

    Texture {
        texture,
        view,
        bind_group,
        width: size,
        height: size,
    }
}

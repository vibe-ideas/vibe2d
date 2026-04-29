use crate::Color;
use vibe_render::{Font, Renderer, TextureId};

/// The render target for the current frame. Users draw to this.
pub struct Screen<'a> {
    renderer: &'a mut Renderer,
    pub virtual_width: f32,
    pub virtual_height: f32,
    /// Engine-provided antialiased filled-circle texture ID, set up
    /// once at startup by `GameBridge` and forwarded here every frame.
    /// `None` only in test/headless paths that build a Screen by hand.
    builtin_circle_filled: Option<TextureId>,
    /// Engine-provided antialiased ring texture ID. Same lifecycle as
    /// `builtin_circle_filled`.
    builtin_circle_ring: Option<TextureId>,
}

impl<'a> Screen<'a> {
    pub fn new(renderer: &'a mut Renderer, virtual_width: f32, virtual_height: f32) -> Self {
        Self {
            renderer,
            virtual_width,
            virtual_height,
            builtin_circle_filled: None,
            builtin_circle_ring: None,
        }
    }

    /// Wire up the built-in circle / ring texture IDs. Called by
    /// `GameBridge` right after constructing the Screen each frame; not
    /// intended to be called from game code (the IDs come from
    /// engine-managed textures registered in `on_init`).
    pub fn set_builtin_circle_textures(
        &mut self,
        filled: Option<TextureId>,
        ring: Option<TextureId>,
    ) {
        self.builtin_circle_filled = filled;
        self.builtin_circle_ring = ring;
    }

    /// Draw a filled, antialiased circle centered at `(cx, cy)` with
    /// the given `radius` (in virtual pixels) and color.
    ///
    /// Implementation: blits the engine's procedural 256² filled-circle
    /// texture (`vibe_render::builtin::CIRCLE_FILLED`) to the bounding square. Because
    /// the texture has alpha-AA on its perimeter, the result composites
    /// cleanly over any background. Internally this is a single sprite
    /// — repeated calls batch into one GPU draw call.
    ///
    /// No-op if the engine couldn't initialize its built-in circle
    /// textures (which would only happen if `Game::new` was called
    /// before `on_init` finished — i.e. it shouldn't happen in normal
    /// game code paths).
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let Some(tex) = self.builtin_circle_filled else {
            return;
        };
        let d = radius * 2.0;
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id: tex,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [cx - radius, cy - radius, d, d],
            color: color.to_array(),
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw an antialiased circle outline (ring) centered at `(cx, cy)`
    /// with the given `radius` (in virtual pixels) and color.
    ///
    /// The ring's stroke thickness is fixed (proportional to `radius`,
    /// see the engine's `vibe_render::builtin::CIRCLE_RING` texture initialization for
    /// the exact ratio). For wildly different stroke widths, generate a
    /// custom ring texture via `Renderer::create_ring_texture` and use
    /// the regular `draw_sprite_tinted` API.
    ///
    /// No-op if the engine couldn't initialize its built-in circle
    /// textures.
    pub fn draw_circle_outline(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let Some(tex) = self.builtin_circle_ring else {
            return;
        };
        let d = radius * 2.0;
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id: tex,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [cx - radius, cy - radius, d, d],
            color: color.to_array(),
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw a sprite at position (x, y) using the full texture.
    pub fn draw_sprite(&mut self, texture_id: TextureId, x: f32, y: f32, width: f32, height: f32) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw a sprite flipped vertically (used for upside-down pipes, etc.).
    pub fn draw_sprite_flipped(
        &mut self,
        texture_id: TextureId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: true,
        });
    }

    /// Draw a sprite flipped horizontally (used for left-facing characters, etc.).
    pub fn draw_sprite_flipped_h(
        &mut self,
        texture_id: TextureId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: true,
            flip_y: false,
        });
    }

    /// Draw a sprite flipped on both axes.
    pub fn draw_sprite_flipped_both(
        &mut self,
        texture_id: TextureId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: true,
            flip_y: true,
        });
    }

    /// Draw a sub-region of a sprite (for sprite sheets, scrolling textures, etc.).
    pub fn draw_sprite_region(
        &mut self,
        texture_id: TextureId,
        src_rect: [f32; 4],
        dst_rect: [f32; 4],
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect,
            dst_rect,
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw a sub-region of a sprite with flip control.
    pub fn draw_sprite_region_flipped(
        &mut self,
        texture_id: TextureId,
        src_rect: [f32; 4],
        dst_rect: [f32; 4],
        flip_x: bool,
        flip_y: bool,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect,
            dst_rect,
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x,
            flip_y,
        });
    }

    /// Draw text using a loaded font at position (x, y).
    pub fn draw_text(&mut self, font: &Font, text: &str, x: f32, y: f32) {
        for (tex_id, src_rect, dst_rect) in font.layout_text(text, x, y) {
            self.renderer.draw_sprite(vibe_render::DrawCommand {
                texture_id: tex_id,
                src_rect,
                dst_rect,
                color: [1.0, 1.0, 1.0, 1.0],
                flip_x: false,
                flip_y: false,
            });
        }
    }

    /// Draw text centered horizontally at the given y position.
    pub fn draw_text_centered(&mut self, font: &Font, text: &str, y: f32) {
        let text_w = font.text_width(text);
        let x = (self.virtual_width - text_w) / 2.0;
        self.draw_text(font, text, x, y);
    }

    /// Draw a sprite with color tinting (color is multiplied with texture color).
    pub fn draw_sprite_tinted(
        &mut self,
        texture_id: TextureId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: color.to_array(),
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw a sub-region of a sprite with color tinting.
    pub fn draw_sprite_region_tinted(
        &mut self,
        texture_id: TextureId,
        src_rect: [f32; 4],
        dst_rect: [f32; 4],
        color: Color,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect,
            dst_rect,
            color: color.to_array(),
            flip_x: false,
            flip_y: false,
        });
    }

    /// Draw a sub-region of a sprite with flip control and color tinting.
    pub fn draw_sprite_region_flipped_tinted(
        &mut self,
        texture_id: TextureId,
        src_rect: [f32; 4],
        dst_rect: [f32; 4],
        flip_x: bool,
        flip_y: bool,
        color: Color,
    ) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect,
            dst_rect,
            color: color.to_array(),
            flip_x,
            flip_y,
        });
    }
}

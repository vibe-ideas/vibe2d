use vibe_render::{Font, Renderer, TextureId};

/// The render target for the current frame. Users draw to this.
pub struct Screen<'a> {
    renderer: &'a mut Renderer,
    pub virtual_width: f32,
    pub virtual_height: f32,
}

impl<'a> Screen<'a> {
    pub fn new(renderer: &'a mut Renderer, virtual_width: f32, virtual_height: f32) -> Self {
        Self {
            renderer,
            virtual_width,
            virtual_height,
        }
    }

    /// Draw a sprite at position (x, y) using the full texture.
    pub fn draw_sprite(&mut self, texture_id: TextureId, x: f32, y: f32, width: f32, height: f32) {
        self.renderer.draw_sprite(vibe_render::DrawCommand {
            texture_id,
            src_rect: [0.0, 0.0, 1.0, 1.0],
            dst_rect: [x, y, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
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
            flip_y: false,
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
}

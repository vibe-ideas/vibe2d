mod font;
mod procedural;
mod renderer;
mod texture;

pub use font::{Font, PrepareOutcome};
pub use procedural::{build_filled_circle_pixels, build_ring_pixels};
pub use renderer::{DrawCommand, Renderer};
pub use texture::{Texture, TextureId};

/// Names under which the engine registers its runtime-generated
/// "atom" textures into [`vibe_asset::AssetManager`]. These are the
/// canonical labels — *do not* hard-code the same string elsewhere;
/// always import from here.
///
/// See [`crate::Renderer::create_white_pixel_texture`] /
/// [`crate::Renderer::create_filled_circle_texture`] /
/// [`crate::Renderer::create_ring_texture`] for the texture
/// definitions, and `vibe_asset::AssetManager::builtin_white` /
/// `builtin_circle_filled` / `builtin_circle_ring` for the
/// recommended way to look them up by `TextureId` from game code.
///
/// Game-defined texture names must avoid this namespace (the
/// `__vibe_` prefix is reserved for engine-internal textures).
pub mod builtin {
    /// 1×1 white pixel. Used by the UI system to draw colored
    /// rectangles via tinting; reusable by any game that needs a
    /// solid-color rect.
    pub const WHITE: &str = "__vibe_ui_white";

    /// 256² antialiased filled disc. Backs `Screen::draw_circle`.
    pub const CIRCLE_FILLED: &str = "__vibe_circle_filled";

    /// 256² antialiased hollow ring. Backs `Screen::draw_circle_outline`.
    pub const CIRCLE_RING: &str = "__vibe_circle_ring";
}

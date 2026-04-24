use vibe_asset::AssetManager;
use vibe_audio::AudioEngine;
use vibe_ui::UiState;

/// The engine context passed to user game code.
pub struct Context {
    pub assets: AssetManager,
    pub audio: AudioEngine,
    pub ui_state: UiState,
    pub virtual_width: f32,
    pub virtual_height: f32,

    /// Queued `(font_name, text)` pairs for the engine to lazily upload
    /// glyph atlases for. Populated by [`Context::prepare_text`] during
    /// `update` / `update_ui`, drained by `GameBridge` at the start of
    /// `on_render` (where the [`vibe_render::Renderer`] is available).
    ///
    /// Public to the engine crate for the take/swap dance, but games
    /// should always go through [`Context::prepare_text`].
    pub pending_text_prep: Vec<(String, String)>,
}

impl Context {
    /// Queue a font + text pair for lazy glyph atlas preparation.
    ///
    /// All characters in `text` will be rasterized into the named font's
    /// GPU atlas before this frame is rendered, ensuring CJK and other
    /// non-ASCII glyphs show up correctly. Idempotent — calling this every
    /// frame with the same text is cheap (no-op once cached).
    ///
    /// Call this in `update()` / `update_ui()` for any text you intend to
    /// draw later this frame, including text inside scroll lists, the
    /// current `TextInput` buffer, and any in-flight IME preedit.
    ///
    /// The actual GPU work happens later in the frame (right before
    /// rendering) so the renderer borrow doesn't have to leak into
    /// `update`. As a consequence, **layout measurements taken in the
    /// same frame a character is first prepared use a fallback advance
    /// width** (half the font size); the next frame, after the glyph is
    /// cached, measurement is exact.
    pub fn prepare_text(&mut self, font_name: &str, text: &str) {
        if text.is_empty() {
            return;
        }
        self.pending_text_prep
            .push((font_name.to_string(), text.to_string()));
    }
}

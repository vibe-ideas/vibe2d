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
}

use vibe_asset::AssetManager;
use vibe_audio::AudioEngine;

/// The engine context passed to user game code.
pub struct Context {
    pub assets: AssetManager,
    pub audio: AudioEngine,
    pub virtual_width: f32,
    pub virtual_height: f32,
}

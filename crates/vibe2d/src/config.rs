use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GameConfig {
    pub meta: Option<MetaConfig>,
    pub window: WindowConfig,
    pub virtual_resolution: Option<VirtualResolutionConfig>,
    pub assets: Option<AssetsConfig>,
    pub physics: Option<PhysicsConfig>,
    pub input: Option<InputConfig>,
    pub debug: Option<DebugConfig>,
    pub constants: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct MetaConfig {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub resizable: Option<bool>,
    pub vsync: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct VirtualResolutionConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct AssetsConfig {
    pub textures: Option<HashMap<String, String>>,
    pub fonts: Option<HashMap<String, String>>,
    pub audio: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct PhysicsConfig {
    pub gravity: Option<f32>,
    pub iterations: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub actions: HashMap<String, vibe_input::ActionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DebugConfig {
    pub vdp: Option<VdpConfig>,
    pub physics_overlay: Option<bool>,
    pub fps_counter: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct VdpConfig {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
}

impl GameConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_constant_f32(&self, key: &str) -> Option<f32> {
        self.constants
            .as_ref()?
            .get(key)?
            .as_f64()
            .map(|v| v as f32)
    }
}

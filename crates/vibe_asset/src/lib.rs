use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use vibe_render::{Font, Renderer, Texture, TextureId};

/// Manages loaded game assets (textures, fonts, audio).
#[derive(Default)]
pub struct AssetManager {
    textures: Vec<Texture>,
    texture_names: HashMap<String, TextureId>,
    fonts: HashMap<String, Font>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load textures defined in the config.
    pub fn load_textures(
        &mut self,
        renderer: &Renderer,
        base_path: &Path,
        texture_configs: &HashMap<String, String>,
    ) -> Result<()> {
        for (name, rel_path) in texture_configs {
            let full_path = base_path.join(rel_path);
            let bytes = std::fs::read(&full_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to load texture '{}' from {:?}: {}",
                    name,
                    full_path,
                    e
                )
            })?;

            let texture = Texture::from_bytes(
                &renderer.device,
                &renderer.queue,
                &renderer.texture_bind_group_layout,
                &bytes,
                name,
            )?;

            let id = TextureId(self.textures.len());
            self.textures.push(texture);
            self.texture_names.insert(name.clone(), id);
        }
        Ok(())
    }

    /// Get a texture ID by its config name.
    pub fn texture_id(&self, name: &str) -> Option<TextureId> {
        self.texture_names.get(name).copied()
    }

    /// Get a texture reference by ID.
    pub fn texture(&self, id: TextureId) -> &Texture {
        &self.textures[id.0]
    }

    /// Get the (width, height) of a texture in pixels.
    pub fn texture_size(&self, id: TextureId) -> (u32, u32) {
        let t = &self.textures[id.0];
        (t.width, t.height)
    }

    /// Get all textures as a slice (for rendering).
    pub fn all_textures(&self) -> Vec<&Texture> {
        self.textures.iter().collect()
    }

    /// Load fonts defined in the config. Each entry maps name → "path:size".
    pub fn load_fonts(
        &mut self,
        renderer: &Renderer,
        base_path: &Path,
        font_configs: &HashMap<String, String>,
    ) -> Result<()> {
        for (name, config_str) in font_configs {
            // Format: "path/to/font.ttf:32" (path:size)
            let (rel_path, size_str) = config_str.rsplit_once(':').ok_or_else(|| {
                anyhow::anyhow!("Font config '{}' must be 'path:size'", config_str)
            })?;

            let font_size: f32 = size_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid font size '{}' for '{}'", size_str, name))?;

            let full_path = base_path.join(rel_path);
            let bytes = std::fs::read(&full_path).map_err(|e| {
                anyhow::anyhow!("Failed to load font '{}' from {:?}: {}", name, full_path, e)
            })?;

            let texture_id = TextureId(self.textures.len());
            let (font, atlas_texture) = Font::from_bytes(
                &renderer.device,
                &renderer.queue,
                &renderer.texture_bind_group_layout,
                &bytes,
                font_size,
                texture_id,
            )?;

            self.textures.push(atlas_texture);
            self.fonts.insert(name.clone(), font);
            tracing::info!("Loaded font '{}' at {}px", name, font_size);
        }
        Ok(())
    }

    /// Get a font by name.
    pub fn font(&self, name: &str) -> Option<&Font> {
        self.fonts.get(name)
    }

    /// Register a runtime-created texture with the given name.
    /// Returns the assigned TextureId.
    pub fn register_texture(&mut self, name: &str, texture: Texture) -> TextureId {
        let id = TextureId(self.textures.len());
        self.textures.push(texture);
        self.texture_names.insert(name.to_string(), id);
        id
    }
}

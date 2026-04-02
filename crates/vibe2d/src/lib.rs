mod config;
mod context;
mod game;
mod screen;

pub mod prelude {
    pub use crate::config::GameConfig;
    pub use crate::context::Context;
    pub use crate::game::Game;
    pub use crate::screen::Screen;
    pub use crate::{run, Color};
    pub use glam::Vec2;
    pub use vibe_input::InputState;
    pub use vibe_render::TextureId;
}

pub use config::GameConfig;
pub use context::Context;
pub use game::Game;
pub use screen::Screen;

/// RGBA color.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Main entry point. Loads config from YAML and starts the game loop.
pub fn run<G: Game + 'static>(config_path: &str) {
    tracing_subscriber::fmt::init();

    let config = GameConfig::load(config_path).expect("Failed to load game config");

    let virtual_width = config.virtual_resolution.as_ref().map_or(
        config.window.width as f32,
        |vr| vr.width as f32,
    );
    let virtual_height = config.virtual_resolution.as_ref().map_or(
        config.window.height as f32,
        |vr| vr.height as f32,
    );

    let platform_config = vibe_platform::PlatformConfig {
        window_width: config.window.width,
        window_height: config.window.height,
        window_title: config.window.title.clone(),
        vsync: config.window.vsync.unwrap_or(true),
        virtual_width,
        virtual_height,
    };

    let mut input_state = vibe_input::InputState::new();
    if let Some(ref input_cfg) = config.input {
        input_state.load_actions(&input_cfg.actions);
    }

    // Start VDP server if configured
    let vdp_channel = if config
        .debug
        .as_ref()
        .and_then(|d| d.vdp.as_ref())
        .and_then(|v| v.enabled)
        .unwrap_or(false)
    {
        let port = config
            .debug
            .as_ref()
            .and_then(|d| d.vdp.as_ref())
            .and_then(|v| v.port)
            .unwrap_or(9229);

        let (game_channel, server_channel) = vibe_debug::create_channel();
        if let Err(e) = vibe_debug::VdpServer::start(port, server_channel) {
            tracing::error!("Failed to start VDP server: {}", e);
            None
        } else {
            Some(game_channel)
        }
    } else {
        None
    };

    let bridge = GameBridge::<G> {
        game: None,
        assets: vibe_asset::AssetManager::new(),
        audio: vibe_audio::AudioEngine::new(),
        vdp: vdp_channel,
        config,
        base_path: std::path::PathBuf::from(
            std::path::Path::new(config_path)
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        ),
        virtual_width,
        virtual_height,
        pending_screenshot: None,
    };

    vibe_platform::run_desktop(platform_config, bridge, input_state)
        .expect("Game loop failed");
}

use std::path::PathBuf;

/// Bridges the user's Game implementation to the platform callbacks.
struct GameBridge<G: Game> {
    game: Option<G>,
    assets: vibe_asset::AssetManager,
    audio: vibe_audio::AudioEngine,
    vdp: Option<vibe_debug::VdpChannel>,
    config: GameConfig,
    base_path: PathBuf,
    virtual_width: f32,
    virtual_height: f32,
    pending_screenshot: Option<PathBuf>,
}

impl<G: Game> vibe_platform::PlatformCallbacks for GameBridge<G> {
    fn on_init(&mut self, renderer: &vibe_render::Renderer) {
        if let Some(ref tex_configs) = self.config.assets.as_ref().and_then(|a| a.textures.as_ref()) {
            if let Err(e) = self.assets.load_textures(renderer, &self.base_path, tex_configs) {
                tracing::error!("Failed to load textures: {}", e);
            }
        }

        if let Some(ref font_configs) = self.config.assets.as_ref().and_then(|a| a.fonts.as_ref()) {
            if let Err(e) = self.assets.load_fonts(renderer, &self.base_path, font_configs) {
                tracing::error!("Failed to load fonts: {}", e);
            }
        }

        if let Some(ref audio_configs) = self.config.assets.as_ref().and_then(|a| a.audio.as_ref()) {
            if let Err(e) = self.audio.load_sounds(&self.base_path, audio_configs) {
                tracing::error!("Failed to load audio: {}", e);
            }
        }

        let mut ctx = Context {
            assets: std::mem::take(&mut self.assets),
            audio: std::mem::take(&mut self.audio),
            virtual_width: self.virtual_width,
            virtual_height: self.virtual_height,
        };

        self.game = Some(G::new(&mut ctx));

        self.assets = ctx.assets;
        self.audio = ctx.audio;
    }

    fn on_input_event(&mut self, _input: &mut vibe_input::InputState) {}

    fn on_update(&mut self, dt: f32, input: &vibe_input::InputState) {
        self.process_vdp_requests();

        if let Some(game) = &mut self.game {
            let mut ctx = Context {
                assets: std::mem::take(&mut self.assets),
                audio: std::mem::take(&mut self.audio),
                virtual_width: self.virtual_width,
                virtual_height: self.virtual_height,
            };
            game.update(&mut ctx, dt, input);
            self.assets = ctx.assets;
            self.audio = ctx.audio;
        }
    }

    fn on_render(&mut self, renderer: &mut vibe_render::Renderer) {
        if let Some(path) = self.pending_screenshot.take() {
            renderer.request_screenshot(path);
        }

        if let Some(game) = &mut self.game {
            let ctx = Context {
                assets: std::mem::take(&mut self.assets),
                audio: std::mem::take(&mut self.audio),
                virtual_width: self.virtual_width,
                virtual_height: self.virtual_height,
            };
            let mut screen = Screen::new(renderer, self.virtual_width, self.virtual_height);
            game.draw(&ctx, &mut screen);
            self.assets = ctx.assets;
            self.audio = ctx.audio;
        }
    }

    fn clear_color(&self) -> [f32; 4] {
        if let Some(game) = &self.game {
            game.clear_color().to_array()
        } else {
            [0.0, 0.0, 0.0, 1.0]
        }
    }

    fn get_textures(&self) -> Vec<&vibe_render::Texture> {
        self.assets.all_textures()
    }
}

impl<G: Game> GameBridge<G> {
    fn process_vdp_requests(&mut self) {
        let vdp = match &self.vdp {
            Some(v) => v,
            None => return,
        };

        // Collect all pending requests first to avoid borrow conflict
        let requests: Vec<_> = std::iter::from_fn(|| vdp.receiver.try_recv().ok()).collect();

        for req in requests {
            let response = self.handle_vdp_request(&req);
            if let Some(vdp) = &self.vdp {
                let _ = vdp.sender.send(response);
            }
        }
    }

    fn handle_vdp_request(&mut self, req: &vibe_debug::VdpRequest) -> vibe_debug::VdpResponse {
        match req.method.as_str() {
            "engine.info" => {
                vibe_debug::VdpResponse::success(
                    req.id.clone(),
                    serde_json::json!({
                        "engine": "vibe2d",
                        "version": env!("CARGO_PKG_VERSION"),
                        "virtual_width": self.virtual_width,
                        "virtual_height": self.virtual_height,
                    }),
                )
            }
            "game.inspect" => {
                if let Some(game) = &self.game {
                    vibe_debug::VdpResponse::success(req.id.clone(), game.inspect())
                } else {
                    vibe_debug::VdpResponse::error(req.id.clone(), -32000, "Game not initialized")
                }
            }
            "game.screenshot" => {
                let path = req.params.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("screenshot.png")
                    .to_string();
                self.pending_screenshot = Some(PathBuf::from(&path));
                vibe_debug::VdpResponse::success(
                    req.id.clone(),
                    serde_json::json!({ "path": path, "status": "queued" }),
                )
            }
            _ => {
                // Try game-specific handler
                if let Some(game) = &mut self.game {
                    match game.handle_vdp(&req.method, &req.params) {
                        Ok(result) => vibe_debug::VdpResponse::success(req.id.clone(), result),
                        Err(msg) => vibe_debug::VdpResponse::error(req.id.clone(), -32000, msg),
                    }
                } else {
                    vibe_debug::VdpResponse::method_not_found(req.id.clone(), &req.method)
                }
            }
        }
    }
}

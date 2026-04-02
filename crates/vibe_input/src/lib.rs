use std::collections::HashMap;
use serde::Deserialize;
use winit::keyboard::KeyCode;

/// Tracks keyboard state: which keys are currently down, just pressed, or just released.
pub struct InputState {
    pressed: HashMap<KeyCode, bool>,
    just_pressed: HashMap<KeyCode, bool>,
    just_released: HashMap<KeyCode, bool>,
    actions: HashMap<String, Vec<KeyCode>>,
}

/// Input action mapping from game.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct ActionConfig {
    pub keys: Vec<String>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashMap::new(),
            just_pressed: HashMap::new(),
            just_released: HashMap::new(),
            actions: HashMap::new(),
        }
    }

    /// Load action mappings from config.
    pub fn load_actions(&mut self, actions: &HashMap<String, ActionConfig>) {
        for (name, config) in actions {
            let keycodes: Vec<KeyCode> = config
                .keys
                .iter()
                .filter_map(|s| string_to_keycode(s))
                .collect();
            self.actions.insert(name.clone(), keycodes);
        }
    }

    /// Called at the start of each frame to clear per-frame state.
    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Called when a key is pressed.
    pub fn on_key_pressed(&mut self, key: KeyCode) {
        if !self.pressed.get(&key).copied().unwrap_or(false) {
            self.just_pressed.insert(key, true);
        }
        self.pressed.insert(key, true);
    }

    /// Called when a key is released.
    pub fn on_key_released(&mut self, key: KeyCode) {
        self.pressed.insert(key, false);
        self.just_released.insert(key, true);
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed.get(&key).copied().unwrap_or(false)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.get(&key).copied().unwrap_or(false)
    }

    /// Check if an action (defined in game.yaml) was just pressed this frame.
    pub fn is_action_just_pressed(&self, action: &str) -> bool {
        if let Some(keys) = self.actions.get(action) {
            keys.iter().any(|k| self.is_key_just_pressed(*k))
        } else {
            false
        }
    }

    /// Check if an action is currently held down.
    pub fn is_action_pressed(&self, action: &str) -> bool {
        if let Some(keys) = self.actions.get(action) {
            keys.iter().any(|k| self.is_key_pressed(*k))
        } else {
            false
        }
    }
}

fn string_to_keycode(s: &str) -> Option<KeyCode> {
    match s {
        "Space" => Some(KeyCode::Space),
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Up" => Some(KeyCode::ArrowUp),
        "Down" => Some(KeyCode::ArrowDown),
        "Left" => Some(KeyCode::ArrowLeft),
        "Right" => Some(KeyCode::ArrowRight),
        "A" => Some(KeyCode::KeyA),
        "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC),
        "D" => Some(KeyCode::KeyD),
        "W" => Some(KeyCode::KeyW),
        "S" => Some(KeyCode::KeyS),
        _ => None,
    }
}

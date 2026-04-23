use std::collections::HashMap;
use serde::Deserialize;
pub use winit::keyboard::KeyCode;

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Tracks keyboard and mouse state per frame.
pub struct InputState {
    // ── Keyboard ──
    pressed: HashMap<KeyCode, bool>,
    just_pressed: HashMap<KeyCode, bool>,
    just_released: HashMap<KeyCode, bool>,
    actions: HashMap<String, Vec<KeyCode>>,

    // ── Mouse ──
    mouse_x: f32,
    mouse_y: f32,
    mouse_pressed: HashMap<MouseButton, bool>,
    mouse_just_pressed: HashMap<MouseButton, bool>,
    mouse_just_released: HashMap<MouseButton, bool>,
    mouse_actions: HashMap<String, Vec<MouseButton>>,

    // ── Character input (for UI text input) ──
    chars_received: Vec<char>,

    // ── Mouse scroll ──
    scroll_delta: f32,
    scroll_delta_x: f32,
}

/// Input action mapping from game.yaml
#[derive(Debug, Clone, Deserialize)]
pub struct ActionConfig {
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub mouse_buttons: Vec<String>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashMap::new(),
            just_pressed: HashMap::new(),
            just_released: HashMap::new(),
            actions: HashMap::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_pressed: HashMap::new(),
            mouse_just_pressed: HashMap::new(),
            mouse_just_released: HashMap::new(),
            mouse_actions: HashMap::new(),
            chars_received: Vec::new(),
            scroll_delta: 0.0,
            scroll_delta_x: 0.0,
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
            if !keycodes.is_empty() {
                self.actions.insert(name.clone(), keycodes);
            }

            let buttons: Vec<MouseButton> = config
                .mouse_buttons
                .iter()
                .filter_map(|s| string_to_mouse_button(s))
                .collect();
            if !buttons.is_empty() {
                self.mouse_actions.insert(name.clone(), buttons);
            }
        }
    }

    /// Called at the start of each frame to clear per-frame state.
    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.chars_received.clear();
        self.scroll_delta = 0.0;
        self.scroll_delta_x = 0.0;
    }

    // ── Keyboard events ──

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

    // ── Mouse events ──

    /// Called when the mouse cursor moves (coordinates in virtual resolution).
    pub fn on_mouse_moved(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    /// Called when a mouse button is pressed.
    pub fn on_mouse_button_pressed(&mut self, button: MouseButton) {
        if !self.mouse_pressed.get(&button).copied().unwrap_or(false) {
            self.mouse_just_pressed.insert(button, true);
        }
        self.mouse_pressed.insert(button, true);
    }

    /// Called when a mouse button is released.
    pub fn on_mouse_button_released(&mut self, button: MouseButton) {
        self.mouse_pressed.insert(button, false);
        self.mouse_just_released.insert(button, true);
    }

    /// Get the current mouse position in virtual coordinates.
    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.get(&button).copied().unwrap_or(false)
    }

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.get(&button).copied().unwrap_or(false)
    }

    // ── Action queries (keyboard + mouse) ──

    /// Check if an action (defined in game.yaml) was just pressed this frame.
    pub fn is_action_just_pressed(&self, action: &str) -> bool {
        let key_match = self.actions.get(action)
            .is_some_and(|keys| keys.iter().any(|k| self.is_key_just_pressed(*k)));
        let mouse_match = self.mouse_actions.get(action)
            .is_some_and(|btns| btns.iter().any(|b| self.is_mouse_button_just_pressed(*b)));
        key_match || mouse_match
    }

    // ── Character input ──

    /// Characters received this frame (for text input widgets).
    pub fn chars_this_frame(&self) -> &[char] {
        &self.chars_received
    }

    /// Called by the platform layer when a printable character is received.
    pub fn on_char_received(&mut self, ch: char) {
        self.chars_received.push(ch);
    }

    // ── Mouse scroll ──

    /// Vertical mouse scroll wheel delta this frame (positive = scroll up).
    pub fn mouse_scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    /// Horizontal mouse scroll wheel delta this frame (positive = scroll right).
    pub fn mouse_scroll_delta_x(&self) -> f32 {
        self.scroll_delta_x
    }

    /// Called by the platform layer when a scroll event is received.
    pub fn on_mouse_scroll(&mut self, delta_x: f32, delta_y: f32) {
        self.scroll_delta += delta_y;
        self.scroll_delta_x += delta_x;
    }

    /// Check if an action is currently held down.
    pub fn is_action_pressed(&self, action: &str) -> bool {
        let key_match = self.actions.get(action)
            .is_some_and(|keys| keys.iter().any(|k| self.is_key_pressed(*k)));
        let mouse_match = self.mouse_actions.get(action)
            .is_some_and(|btns| btns.iter().any(|b| self.is_mouse_button_pressed(*b)));
        key_match || mouse_match
    }
}

pub fn string_to_keycode(s: &str) -> Option<KeyCode> {
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
        "E" => Some(KeyCode::KeyE),
        "F" => Some(KeyCode::KeyF),
        "W" => Some(KeyCode::KeyW),
        "S" => Some(KeyCode::KeyS),
        "X" => Some(KeyCode::KeyX),
        "Z" => Some(KeyCode::KeyZ),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        _ => None,
    }
}

pub fn string_to_mouse_button(s: &str) -> Option<MouseButton> {
    match s {
        "Left" => Some(MouseButton::Left),
        "Right" => Some(MouseButton::Right),
        "Middle" => Some(MouseButton::Middle),
        _ => None,
    }
}

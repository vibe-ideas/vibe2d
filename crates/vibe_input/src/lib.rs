use serde::Deserialize;
use std::collections::HashMap;
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

    // ── IME (Input Method Editor) ──
    /// Text committed by the IME this frame (the result of finalizing a
    /// composition, e.g. selecting a Chinese candidate). Multi-character
    /// strings are inserted as one atomic unit, unlike `chars_received`.
    ime_commit: String,
    /// In-flight composition text being edited by the IME, with the cursor
    /// position (byte offset within the preedit). `None` when no IME
    /// composition is active. The preedit must be rendered as a hint above
    /// the focused widget but **not** appended to the widget's text buffer.
    ime_preedit: Option<ImePreedit>,

    // ── Mouse scroll ──
    scroll_delta: f32,
    scroll_delta_x: f32,
}

/// In-progress IME composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImePreedit {
    /// The text currently being composed (e.g. "ni hao" / "你好" candidates).
    pub text: String,
    /// Caret byte offset inside `text`. `None` when the cursor is hidden.
    pub cursor_byte: Option<usize>,
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
            ime_commit: String::new(),
            ime_preedit: None,
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
    ///
    /// Note: `ime_preedit` persists across frames — it represents IME
    /// composition state, which is cleared by the platform layer via
    /// `clear_ime_preedit()` when the IME explicitly ends/cancels.
    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.chars_received.clear();
        self.ime_commit.clear();
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
        self.mouse_just_pressed
            .get(&button)
            .copied()
            .unwrap_or(false)
    }

    // ── Action queries (keyboard + mouse) ──

    /// Check if an action (defined in game.yaml) was just pressed this frame.
    pub fn is_action_just_pressed(&self, action: &str) -> bool {
        let key_match = self
            .actions
            .get(action)
            .is_some_and(|keys| keys.iter().any(|k| self.is_key_just_pressed(*k)));
        let mouse_match = self
            .mouse_actions
            .get(action)
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

    // ── IME ──

    /// Text committed by the IME this frame, if any (e.g. a finalized Chinese word).
    /// Empty when no commit happened this frame.
    pub fn ime_commit(&self) -> &str {
        &self.ime_commit
    }

    /// Current in-progress IME composition, if any.
    /// Returns `None` when no IME composition is active.
    pub fn ime_preedit(&self) -> Option<&ImePreedit> {
        self.ime_preedit.as_ref()
    }

    /// Called by the platform layer when the IME commits a composition.
    /// Multiple commits within the same frame are concatenated (rare in practice).
    pub fn on_ime_commit(&mut self, text: &str) {
        self.ime_commit.push_str(text);
        // A commit always ends the composition.
        self.ime_preedit = None;
    }

    /// Called by the platform layer for IME preedit updates.
    /// Pass an empty `text` to clear the preedit.
    pub fn on_ime_preedit(&mut self, text: String, cursor_byte: Option<usize>) {
        if text.is_empty() {
            self.ime_preedit = None;
        } else {
            self.ime_preedit = Some(ImePreedit { text, cursor_byte });
        }
    }

    /// Explicitly clear any in-progress IME composition (e.g. on focus loss).
    pub fn clear_ime_preedit(&mut self) {
        self.ime_preedit = None;
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
        let key_match = self
            .actions
            .get(action)
            .is_some_and(|keys| keys.iter().any(|k| self.is_key_pressed(*k)));
        let mouse_match = self
            .mouse_actions
            .get(action)
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

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────
// Unit tests — pure logic, no winit event loop required
// ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_action_config(keys: &[&str], mouse_buttons: &[&str]) -> ActionConfig {
        ActionConfig {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            mouse_buttons: mouse_buttons.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn string_to_keycode_known_keys() {
        assert_eq!(string_to_keycode("Space"), Some(KeyCode::Space));
        assert_eq!(string_to_keycode("Enter"), Some(KeyCode::Enter));
        assert_eq!(string_to_keycode("Return"), Some(KeyCode::Enter));
        assert_eq!(string_to_keycode("Escape"), Some(KeyCode::Escape));
        assert_eq!(string_to_keycode("Up"), Some(KeyCode::ArrowUp));
        assert_eq!(string_to_keycode("Down"), Some(KeyCode::ArrowDown));
        assert_eq!(string_to_keycode("Left"), Some(KeyCode::ArrowLeft));
        assert_eq!(string_to_keycode("Right"), Some(KeyCode::ArrowRight));
        assert_eq!(string_to_keycode("A"), Some(KeyCode::KeyA));
        assert_eq!(string_to_keycode("W"), Some(KeyCode::KeyW));
        assert_eq!(string_to_keycode("ShiftLeft"), Some(KeyCode::ShiftLeft));
    }

    #[test]
    fn string_to_keycode_unknown_returns_none() {
        assert_eq!(string_to_keycode("F1"), None);
        assert_eq!(string_to_keycode(""), None);
        assert_eq!(string_to_keycode("space"), None); // case-sensitive
    }

    #[test]
    fn string_to_mouse_button_known() {
        assert_eq!(string_to_mouse_button("Left"), Some(MouseButton::Left));
        assert_eq!(string_to_mouse_button("Right"), Some(MouseButton::Right));
        assert_eq!(string_to_mouse_button("Middle"), Some(MouseButton::Middle));
        assert_eq!(string_to_mouse_button("X"), None);
    }

    #[test]
    fn key_press_sets_pressed_and_just_pressed() {
        let mut input = InputState::new();
        input.on_key_pressed(KeyCode::Space);
        assert!(input.is_key_pressed(KeyCode::Space));
        assert!(input.is_key_just_pressed(KeyCode::Space));
    }

    #[test]
    fn key_just_pressed_clears_after_begin_frame() {
        let mut input = InputState::new();
        input.on_key_pressed(KeyCode::Space);
        assert!(input.is_key_just_pressed(KeyCode::Space));
        input.begin_frame();
        // Still held, but no longer "just" pressed
        assert!(input.is_key_pressed(KeyCode::Space));
        assert!(!input.is_key_just_pressed(KeyCode::Space));
    }

    #[test]
    fn key_release_clears_pressed() {
        let mut input = InputState::new();
        input.on_key_pressed(KeyCode::KeyA);
        input.begin_frame();
        input.on_key_released(KeyCode::KeyA);
        assert!(!input.is_key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn key_repeated_press_does_not_retrigger_just_pressed() {
        let mut input = InputState::new();
        input.on_key_pressed(KeyCode::Space);
        input.begin_frame();
        // Already held — pressing again on the same key should NOT mark just_pressed
        input.on_key_pressed(KeyCode::Space);
        assert!(!input.is_key_just_pressed(KeyCode::Space));
    }

    #[test]
    fn mouse_position_tracks_movement() {
        let mut input = InputState::new();
        input.on_mouse_moved(123.0, 456.0);
        assert_eq!(input.mouse_position(), (123.0, 456.0));
    }

    #[test]
    fn mouse_button_state_machine() {
        let mut input = InputState::new();
        input.on_mouse_button_pressed(MouseButton::Left);
        assert!(input.is_mouse_button_pressed(MouseButton::Left));
        assert!(input.is_mouse_button_just_pressed(MouseButton::Left));
        input.begin_frame();
        assert!(input.is_mouse_button_pressed(MouseButton::Left));
        assert!(!input.is_mouse_button_just_pressed(MouseButton::Left));
        input.on_mouse_button_released(MouseButton::Left);
        assert!(!input.is_mouse_button_pressed(MouseButton::Left));
    }

    #[test]
    fn action_mapping_keyboard() {
        let mut input = InputState::new();
        let mut actions = HashMap::new();
        actions.insert("jump".to_string(), make_action_config(&["Space"], &[]));
        input.load_actions(&actions);

        assert!(!input.is_action_just_pressed("jump"));
        input.on_key_pressed(KeyCode::Space);
        assert!(input.is_action_just_pressed("jump"));
        assert!(input.is_action_pressed("jump"));
    }

    #[test]
    fn action_mapping_mouse() {
        let mut input = InputState::new();
        let mut actions = HashMap::new();
        actions.insert("attack".to_string(), make_action_config(&[], &["Left"]));
        input.load_actions(&actions);

        input.on_mouse_button_pressed(MouseButton::Left);
        assert!(input.is_action_just_pressed("attack"));
    }

    #[test]
    fn action_mapping_mixed_keyboard_and_mouse() {
        let mut input = InputState::new();
        let mut actions = HashMap::new();
        actions.insert(
            "fire".to_string(),
            make_action_config(&["Space", "Enter"], &["Left", "Right"]),
        );
        input.load_actions(&actions);

        input.on_mouse_button_pressed(MouseButton::Right);
        assert!(input.is_action_just_pressed("fire"));
        input.begin_frame();

        input.on_key_pressed(KeyCode::Enter);
        assert!(input.is_action_just_pressed("fire"));
    }

    #[test]
    fn action_with_invalid_keys_filters_them_out() {
        let mut input = InputState::new();
        let mut actions = HashMap::new();
        actions.insert(
            "jump".to_string(),
            make_action_config(&["BogusKey", "Space"], &[]),
        );
        input.load_actions(&actions);

        input.on_key_pressed(KeyCode::Space);
        assert!(input.is_action_just_pressed("jump"));
    }

    #[test]
    fn unknown_action_returns_false() {
        let input = InputState::new();
        assert!(!input.is_action_just_pressed("nonexistent"));
        assert!(!input.is_action_pressed("nonexistent"));
    }

    #[test]
    fn chars_received_buffered_and_cleared_each_frame() {
        let mut input = InputState::new();
        input.on_char_received('a');
        input.on_char_received('b');
        assert_eq!(input.chars_this_frame(), &['a', 'b']);
        input.begin_frame();
        assert!(input.chars_this_frame().is_empty());
    }

    #[test]
    fn ime_commit_buffered_and_cleared_each_frame() {
        let mut input = InputState::new();
        assert_eq!(input.ime_commit(), "");
        input.on_ime_commit("你好");
        assert_eq!(input.ime_commit(), "你好");
        // Multiple commits in the same frame concatenate.
        input.on_ime_commit("世界");
        assert_eq!(input.ime_commit(), "你好世界");
        input.begin_frame();
        assert_eq!(input.ime_commit(), "");
    }

    #[test]
    fn ime_commit_clears_active_preedit() {
        let mut input = InputState::new();
        input.on_ime_preedit("nih".to_string(), Some(3));
        assert!(input.ime_preedit().is_some());
        input.on_ime_commit("你");
        assert!(input.ime_preedit().is_none());
    }

    #[test]
    fn ime_preedit_persists_across_frames_until_cleared() {
        let mut input = InputState::new();
        input.on_ime_preedit("ni".to_string(), Some(2));
        let pe = input.ime_preedit().expect("preedit set");
        assert_eq!(pe.text, "ni");
        assert_eq!(pe.cursor_byte, Some(2));

        // begin_frame must NOT clear the preedit (it's a stateful IME composition).
        input.begin_frame();
        assert!(input.ime_preedit().is_some());

        // Empty preedit text clears it.
        input.on_ime_preedit(String::new(), None);
        assert!(input.ime_preedit().is_none());
    }

    #[test]
    fn ime_preedit_explicit_clear() {
        let mut input = InputState::new();
        input.on_ime_preedit("x".to_string(), Some(1));
        input.clear_ime_preedit();
        assert!(input.ime_preedit().is_none());
    }

    #[test]
    fn scroll_delta_accumulates_within_frame() {
        let mut input = InputState::new();
        input.on_mouse_scroll(0.0, 1.0);
        input.on_mouse_scroll(2.0, 3.0);
        assert_eq!(input.mouse_scroll_delta(), 4.0);
        assert_eq!(input.mouse_scroll_delta_x(), 2.0);
        input.begin_frame();
        assert_eq!(input.mouse_scroll_delta(), 0.0);
        assert_eq!(input.mouse_scroll_delta_x(), 0.0);
    }
}

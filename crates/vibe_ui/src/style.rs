/// RGBA color for UI rendering.
#[derive(Debug, Clone, Copy)]
pub struct UiColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl UiColor {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Global UI style configuration.
#[derive(Debug, Clone)]
pub struct Style {
    pub text_color: UiColor,
    pub spacing: f32,
    pub padding: f32,
    pub button: ButtonStyle,
    pub panel: PanelStyle,
    pub text_input: TextInputStyle,
    pub scroll_list: ScrollListStyle,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            text_color: UiColor::WHITE,
            spacing: 4.0,
            padding: 8.0,
            button: ButtonStyle::default(),
            panel: PanelStyle::default(),
            text_input: TextInputStyle::default(),
            scroll_list: ScrollListStyle::default(),
        }
    }
}

/// Visual style for Button widgets.
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub bg_color: UiColor,
    pub hover_color: UiColor,
    pub pressed_color: UiColor,
    pub text_color: UiColor,
    pub padding: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            bg_color: UiColor::new(0.3, 0.3, 0.3, 0.8),
            hover_color: UiColor::new(0.5, 0.5, 0.5, 0.8),
            pressed_color: UiColor::new(0.2, 0.2, 0.2, 0.9),
            text_color: UiColor::WHITE,
            padding: 6.0,
        }
    }
}

/// Visual style for Panel widgets.
#[derive(Debug, Clone)]
pub struct PanelStyle {
    pub bg_color: UiColor,
    pub padding: f32,
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            bg_color: UiColor::new(0.0, 0.0, 0.0, 0.7),
            padding: 12.0,
        }
    }
}

/// Visual style for TextInput widgets.
#[derive(Debug, Clone)]
pub struct TextInputStyle {
    pub bg_color: UiColor,
    pub focused_bg_color: UiColor,
    pub border_color: UiColor,
    pub focused_border_color: UiColor,
    pub text_color: UiColor,
    pub placeholder_color: UiColor,
    pub cursor_color: UiColor,
    pub selection_color: UiColor,
    pub padding: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            bg_color: UiColor::new(0.15, 0.15, 0.15, 0.9),
            focused_bg_color: UiColor::new(0.2, 0.2, 0.2, 0.95),
            border_color: UiColor::new(0.5, 0.5, 0.5, 0.8),
            focused_border_color: UiColor::new(0.4, 0.7, 1.0, 0.9),
            text_color: UiColor::WHITE,
            placeholder_color: UiColor::new(0.5, 0.5, 0.5, 1.0),
            cursor_color: UiColor::WHITE,
            selection_color: UiColor::new(0.3, 0.5, 0.8, 0.5),
            padding: 4.0,
        }
    }
}

/// Visual style for ScrollList widgets.
#[derive(Debug, Clone)]
pub struct ScrollListStyle {
    pub bg_color: UiColor,
    pub scrollbar_color: UiColor,
    pub scrollbar_width: f32,
    pub padding: f32,
}

impl Default for ScrollListStyle {
    fn default() -> Self {
        Self {
            bg_color: UiColor::new(0.1, 0.1, 0.1, 0.5),
            scrollbar_color: UiColor::new(0.5, 0.5, 0.5, 0.5),
            scrollbar_width: 4.0,
            padding: 4.0,
        }
    }
}

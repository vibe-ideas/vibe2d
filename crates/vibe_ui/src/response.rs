/// Interaction response returned by interactive widgets (Button, etc.).
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// Mouse is currently hovering over the widget.
    pub hovered: bool,
    /// Mouse button is held down on the widget.
    pub pressed: bool,
    /// Mouse was clicked on the widget this frame.
    pub clicked: bool,
}

impl Response {
    pub fn clicked(&self) -> bool {
        self.clicked
    }

    pub fn hovered(&self) -> bool {
        self.hovered
    }
}

/// Extended response for TextInput widgets.
#[derive(Debug, Clone, Default)]
pub struct TextInputResponse {
    pub response: Response,
    /// Text content changed this frame.
    pub changed: bool,
    /// User pressed Enter to submit.
    pub submitted: bool,
}

/// Extended response for ScrollList widgets.
#[derive(Debug, Clone, Default)]
pub struct ScrollListResponse {
    pub response: Response,
    /// Current vertical scroll offset in pixels.
    pub scroll_offset: f32,
    /// Current horizontal scroll offset in pixels.
    pub horizontal_offset: f32,
    /// Total height of all content.
    pub content_height: f32,
    /// Total width of all content.
    pub content_width: f32,
    /// Height of the visible area.
    pub visible_height: f32,
    /// Width of the visible area.
    pub visible_width: f32,
}

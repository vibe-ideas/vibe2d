use std::collections::HashMap;

use vibe_render::DrawCommand;

use crate::id::WidgetId;
use crate::vdp::{VdpUiAction, WidgetSnapshot};

/// Persistent state for TextInput widgets, stored across frames.
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    pub text: String,
    pub cursor_position: usize,
    pub selection_start: Option<usize>,
}

/// Which scrollbar is currently being dragged (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarDrag {
    #[default]
    None,
    Vertical,
    Horizontal,
}

/// Persistent state for ScrollList widgets, stored across frames.
#[derive(Debug, Clone, Default)]
pub struct ScrollListState {
    pub scroll_offset: f32,
    pub horizontal_offset: f32,
    pub total_content_height: f32,
    pub total_content_width: f32,

    /// Active scrollbar drag state.
    pub dragging: ScrollbarDrag,
    /// Mouse position at drag start (y for vertical, x for horizontal).
    pub drag_start_mouse: f32,
    /// Scroll offset at drag start.
    pub drag_start_offset: f32,
}

/// Cross-frame persistent UI state, stored in the engine Context.
///
/// Manages focus, text input buffers, scroll positions, VDP snapshots, and pending VDP actions.
pub struct UiState {
    /// Currently focused widget (receives keyboard input).
    pub focused: Option<WidgetId>,

    /// TextInput persistent state indexed by widget ID.
    pub text_inputs: HashMap<WidgetId, TextInputState>,

    /// ScrollList persistent state indexed by widget ID.
    pub scroll_lists: HashMap<WidgetId, ScrollListState>,

    /// Widget tree snapshot from the last frame (for VDP inspection).
    pub last_frame_widgets: Vec<WidgetSnapshot>,

    /// VDP-injected actions to be consumed in the next frame's ui() call.
    pub pending_vdp_actions: Vec<VdpUiAction>,

    /// Cached draw commands from the last `draw_ui()` call, replayed during `on_render`.
    pub cached_draw_commands: Vec<DrawCommand>,

    /// Auto-ID counter, reset each frame.
    pub auto_id_counter: usize,

    /// Elapsed time in seconds (for cursor blink animation).
    pub elapsed_time: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focused: None,
            text_inputs: HashMap::new(),
            scroll_lists: HashMap::new(),
            last_frame_widgets: Vec::new(),
            pending_vdp_actions: Vec::new(),
            cached_draw_commands: Vec::new(),
            auto_id_counter: 0,
            elapsed_time: 0.0,
        }
    }
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate the next auto-ID for this frame.
    pub fn next_auto_id(&mut self) -> WidgetId {
        let id = WidgetId::auto(self.auto_id_counter);
        self.auto_id_counter += 1;
        id
    }

    /// Reset per-frame counters. Called at the start of each ui() invocation.
    pub fn begin_frame(&mut self) {
        self.auto_id_counter = 0;
    }

    /// Update elapsed time (called from engine update loop).
    pub fn update_time(&mut self, dt: f64) {
        self.elapsed_time += dt;
    }

    /// Get or create TextInput state for the given ID.
    pub fn text_input_state(&mut self, id: &WidgetId) -> &mut TextInputState {
        self.text_inputs.entry(id.clone()).or_default()
    }

    /// Get or create ScrollList state for the given ID.
    pub fn scroll_list_state(&mut self, id: &WidgetId) -> &mut ScrollListState {
        self.scroll_lists.entry(id.clone()).or_default()
    }

    /// Drain all pending VDP actions for consumption.
    pub fn drain_vdp_actions(&mut self) -> Vec<VdpUiAction> {
        std::mem::take(&mut self.pending_vdp_actions)
    }

    /// Push a VDP action into the pending queue (called from VDP request handlers).
    pub fn push_vdp_action(&mut self, action: VdpUiAction) {
        self.pending_vdp_actions.push(action);
    }
}

use serde::{Deserialize, Serialize};

use crate::id::WidgetId;

/// Snapshot of a single widget's state, captured each frame for VDP inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSnapshot {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    /// Position and size on screen: [x, y, width, height].
    pub rect: [f32; 4],
    pub visible: bool,
    pub properties: WidgetProperties,
}

/// Type discriminator for widget snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    Label,
    Button,
    Panel,
    ProgressBar,
    TextInput,
    ScrollList,
}

/// Type-specific properties attached to a widget snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WidgetProperties {
    Label {
        text: String,
        color: [f32; 4],
    },
    Button {
        text: String,
        hovered: bool,
        pressed: bool,
    },
    Panel {
        children: Vec<WidgetId>,
    },
    ProgressBar {
        progress: f32,
    },
    TextInput {
        text: String,
        placeholder: String,
        focused: bool,
        cursor_position: usize,
    },
    ScrollList {
        scroll_offset: f32,
        horizontal_offset: f32,
        content_height: f32,
        content_width: f32,
        visible_height: f32,
        visible_width: f32,
        children: Vec<WidgetId>,
    },
}

/// Actions that can be injected via VDP to manipulate UI widgets.
#[derive(Debug, Clone)]
pub enum VdpUiAction {
    Click { id: WidgetId },
    SetText { id: WidgetId, text: String },
    Submit { id: WidgetId },
    SetFocus { id: WidgetId },
    ClearFocus,
    Scroll { id: WidgetId, offset: f32 },
    ScrollHorizontal { id: WidgetId, offset: f32 },
    ScrollToBottom { id: WidgetId },
}

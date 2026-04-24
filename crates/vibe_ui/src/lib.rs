mod context;
mod id;
mod layout;
mod response;
mod state;
mod style;
mod vdp;

pub use context::{UiContext, UiOutput};
pub use id::WidgetId;
pub use layout::{Anchor, LayoutDirection};
pub use response::{Response, ScrollListResponse, TextInputResponse};
pub use state::UiState;
pub use style::{ButtonStyle, PanelStyle, ScrollListStyle, Style, TextInputStyle, UiColor};
pub use vdp::{VdpUiAction, WidgetProperties, WidgetSnapshot, WidgetType};

// ─────────────────────────────────────────────────────────────────────
// Unit tests — pure logic, no GPU/window required
// ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::anchor_origin;

    // ── WidgetId ──────────────────────────────────────────────

    #[test]
    fn widget_id_new_and_display() {
        let id = WidgetId::new("retry_btn");
        assert_eq!(id.0, "retry_btn");
        assert_eq!(format!("{}", id), "retry_btn");
    }

    #[test]
    fn widget_id_auto_format() {
        assert_eq!(WidgetId::auto(0).0, "__auto_0");
        assert_eq!(WidgetId::auto(42).0, "__auto_42");
    }

    #[test]
    fn widget_id_equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(WidgetId::new("foo"));
        assert!(set.contains(&WidgetId::new("foo")));
        assert!(!set.contains(&WidgetId::new("bar")));
    }

    // ── UiColor ───────────────────────────────────────────────

    #[test]
    fn ui_color_constants() {
        assert_eq!(UiColor::WHITE.to_array(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(UiColor::BLACK.to_array(), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(UiColor::TRANSPARENT.to_array(), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn ui_color_from_hex() {
        let c = UiColor::from_hex(0xFF8040);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!((c.g - 128.0 / 255.0).abs() < 1e-5);
        assert!((c.b - 64.0 / 255.0).abs() < 1e-5);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn ui_color_with_alpha() {
        let c = UiColor::WHITE.with_alpha(0.5);
        assert_eq!(c.a, 0.5);
        assert_eq!(c.r, 1.0);
    }

    // ── Anchor / layout::anchor_origin ────────────────────────

    #[test]
    fn anchor_top_left_uses_padding() {
        let (x, y) = anchor_origin(Anchor::TopLeft, 800.0, 600.0, 100.0, 50.0, 10.0);
        assert_eq!((x, y), (10.0, 10.0));
    }

    #[test]
    fn anchor_center_centers_content() {
        let (x, y) = anchor_origin(Anchor::Center, 800.0, 600.0, 200.0, 100.0, 0.0);
        assert_eq!((x, y), (300.0, 250.0));
    }

    #[test]
    fn anchor_bottom_right() {
        let (x, y) = anchor_origin(Anchor::BottomRight, 800.0, 600.0, 100.0, 50.0, 10.0);
        assert_eq!((x, y), (800.0 - 100.0 - 10.0, 600.0 - 50.0 - 10.0));
    }

    #[test]
    fn anchor_top_center_horizontal_only() {
        let (x, y) = anchor_origin(Anchor::TopCenter, 800.0, 600.0, 200.0, 50.0, 5.0);
        assert_eq!((x, y), (300.0, 5.0));
    }

    #[test]
    fn anchor_center_left_vertical_only() {
        let (x, y) = anchor_origin(Anchor::CenterLeft, 800.0, 600.0, 100.0, 100.0, 8.0);
        assert_eq!((x, y), (8.0, 250.0));
    }

    // ── UiState ───────────────────────────────────────────────

    #[test]
    fn ui_state_default_empty() {
        let s = UiState::new();
        assert!(s.focused.is_none());
        assert!(s.text_inputs.is_empty());
        assert!(s.scroll_lists.is_empty());
        assert!(s.last_frame_widgets.is_empty());
        assert!(s.pending_vdp_actions.is_empty());
        assert!(s.cached_draw_commands.is_empty());
        assert_eq!(s.auto_id_counter, 0);
        assert_eq!(s.elapsed_time, 0.0);
    }

    #[test]
    fn ui_state_next_auto_id_increments() {
        let mut s = UiState::new();
        assert_eq!(s.next_auto_id(), WidgetId::auto(0));
        assert_eq!(s.next_auto_id(), WidgetId::auto(1));
        assert_eq!(s.auto_id_counter, 2);
    }

    #[test]
    fn ui_state_begin_frame_resets_auto_id() {
        let mut s = UiState::new();
        s.next_auto_id();
        s.next_auto_id();
        s.begin_frame();
        assert_eq!(s.auto_id_counter, 0);
        assert_eq!(s.next_auto_id(), WidgetId::auto(0));
    }

    #[test]
    fn ui_state_text_input_state_persists() {
        let mut s = UiState::new();
        let id = WidgetId::new("chat");
        s.text_input_state(&id).text = "hello".to_string();
        // Same ID returns the same buffer
        assert_eq!(s.text_input_state(&id).text, "hello");
    }

    #[test]
    fn ui_state_scroll_list_state_persists() {
        let mut s = UiState::new();
        let id = WidgetId::new("list");
        s.scroll_list_state(&id).scroll_offset = 42.0;
        assert_eq!(s.scroll_list_state(&id).scroll_offset, 42.0);
    }

    #[test]
    fn ui_state_update_time_accumulates() {
        let mut s = UiState::new();
        s.update_time(0.016);
        s.update_time(0.016);
        assert!((s.elapsed_time - 0.032).abs() < 1e-9);
    }

    #[test]
    fn ui_state_vdp_action_queue() {
        let mut s = UiState::new();
        s.push_vdp_action(VdpUiAction::Click {
            id: WidgetId::new("btn"),
        });
        s.push_vdp_action(VdpUiAction::ClearFocus);
        let drained = s.drain_vdp_actions();
        assert_eq!(drained.len(), 2);
        // After draining, the queue must be empty
        assert!(s.drain_vdp_actions().is_empty());
    }

    // ── WidgetSnapshot serialization (for VDP ui.listWidgets) ──

    #[test]
    fn widget_snapshot_label_serializes() {
        let snapshot = WidgetSnapshot {
            id: WidgetId::new("title"),
            widget_type: WidgetType::Label,
            rect: [10.0, 20.0, 100.0, 30.0],
            visible: true,
            properties: WidgetProperties::Label {
                text: "Hello".to_string(),
                color: [1.0, 1.0, 1.0, 1.0],
            },
        };
        let json = serde_json::to_value(&snapshot).expect("serialize");
        assert_eq!(json["id"], "title");
        assert_eq!(json["widget_type"], "label");
        assert_eq!(json["visible"], true);
        assert_eq!(json["rect"], serde_json::json!([10.0, 20.0, 100.0, 30.0]));
        // `properties` uses #[serde(untagged)] — variant fields land under
        // `properties`, not the top level.
        assert_eq!(json["properties"]["text"], "Hello");
    }

    #[test]
    fn widget_snapshot_button_serializes_with_state() {
        let snapshot = WidgetSnapshot {
            id: WidgetId::new("retry_btn"),
            widget_type: WidgetType::Button,
            rect: [0.0, 0.0, 80.0, 24.0],
            visible: true,
            properties: WidgetProperties::Button {
                text: "Retry".to_string(),
                hovered: true,
                pressed: false,
            },
        };
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json["widget_type"], "button");
        assert_eq!(json["properties"]["text"], "Retry");
        assert_eq!(json["properties"]["hovered"], true);
        assert_eq!(json["properties"]["pressed"], false);
    }
}

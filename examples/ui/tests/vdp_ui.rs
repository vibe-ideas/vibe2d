//! VDP integration tests for the UI demo.
//!
//! These tests spawn a real `ui-demo` window and drive it via VDP using
//! the engine's [`vibe_test`] helper crate. They verify end-to-end game
//! behaviour — button clicks updating state, text input submission
//! appending to a list, scroll list behaviour, etc.
//!
//! Heavy tests are marked `#[ignore]` so plain `cargo test` stays fast.
//! Run with:
//!
//!     cargo test -p ui-demo -- --ignored --test-threads=1

use serde_json::json;
use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "ui-demo";
// Matches `examples/ui/game.yaml` -> debug.vdp.port.
const VDP_PORT: u16 = 9230;

/// Returns `true` if `widgets` contains a widget snapshot with the given id.
fn widget_by_id<'a>(widgets: &'a [serde_json::Value], id: &str) -> Option<&'a serde_json::Value> {
    widgets
        .iter()
        .find(|w| w.get("id").and_then(|v| v.as_str()) == Some(id))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real ui-demo game window; run with --ignored"]
async fn ui_demo_button_increments_click_count() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch ui-demo");

    // Freeze the loop so widget snapshots and click injection are deterministic.
    h.pause().await.unwrap();
    // Step one frame so the first UI snapshot is captured.
    h.step_and_wait(1).await.unwrap();

    let widgets_before = h.list_widgets().await.unwrap();
    let btn_before = widget_by_id(&widgets_before, "btn_click")
        .expect("btn_click widget missing — did the demo layout change?");
    assert_eq!(
        btn_before["widget_type"], "button",
        "btn_click should be a button widget"
    );

    // Inject a VDP click on the button id and advance one frame so the
    // engine routes the action into the next UI build pass.
    h.ui_click("btn_click").await.unwrap();
    h.step_and_wait(2).await.unwrap();

    // After the click, the demo appends a message like "Button clicked 1 time(s)"
    // — we assert by looking for the counter label in the widget snapshot.
    let widgets_after = h.list_widgets().await.unwrap();
    let counter_label = widgets_after.iter().find(|w| {
        w.get("widget_type").and_then(|t| t.as_str()) == Some("label")
            && w.get("properties")
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .map(|t| t.starts_with("Clicks:"))
                .unwrap_or(false)
    });
    let label = counter_label.expect("`Clicks:` label not found after click");
    let text = label["properties"]["text"].as_str().unwrap();
    assert_eq!(text, "Clicks: 1", "click did not increment counter");

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real ui-demo game window; run with --ignored"]
async fn ui_demo_text_input_submit_appends_message() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch ui-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // Pre-condition: text input exists and starts empty.
    let chat = h
        .find_widget("chat_input")
        .await
        .unwrap()
        .expect("chat_input widget missing");
    assert_eq!(chat["widget_type"], "text_input");
    assert_eq!(chat["properties"]["text"], "");

    // Fill the input and submit via VDP. The demo should push "> hello world"
    // into its `messages` vector on submit, and the submit handler clears
    // the input — both observable via the widget snapshot in the next frame.
    h.ui_set_text("chat_input", "hello world").await.unwrap();
    h.ui_submit("chat_input").await.unwrap();
    h.step_and_wait(2).await.unwrap();

    let widgets = h.list_widgets().await.unwrap();

    // 1. The input was cleared by the submit handler.
    let chat_after = widget_by_id(&widgets, "chat_input").expect("chat_input vanished");
    assert_eq!(
        chat_after["properties"]["text"], "",
        "text input should be cleared after submit"
    );

    // 2. A "> hello world" label appears inside the scroll list.
    let found_echo = widgets.iter().any(|w| {
        w.get("properties")
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            == Some("> hello world")
    });
    assert!(found_echo, "echo line was not appended to messages");

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real ui-demo game window; run with --ignored"]
async fn ui_demo_scroll_list_exposes_offset() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch ui-demo");

    h.pause().await.unwrap();
    h.step_and_wait(1).await.unwrap();

    // Fill the scroll list with enough messages to overflow its visible area.
    for i in 0..30 {
        h.ui_set_text("chat_input", &format!("line {}", i))
            .await
            .unwrap();
        h.ui_submit("chat_input").await.unwrap();
        h.step_and_wait(1).await.unwrap();
    }

    // Scroll to bottom via VDP and observe the offset increases.
    h.ui_scroll_to_bottom("msg_list").await.unwrap();
    h.step_and_wait(2).await.unwrap();

    let list = h
        .find_widget("msg_list")
        .await
        .unwrap()
        .expect("msg_list widget missing");
    assert_eq!(list["widget_type"], "scroll_list");
    let offset = list["properties"]["scroll_offset"]
        .as_f64()
        .expect("scroll_offset must be a number");
    let content_h = list["properties"]["content_height"].as_f64().unwrap();

    // `scrollToBottom` must produce a non-zero offset when the content
    // exceeds the visible area. We don't pin down the exact value here
    // because the engine's clamp formula folds in the scroll list's
    // internal padding (see `scroll_list_impl` in vibe_ui), which isn't
    // reported through the VDP widget snapshot — testing the private
    // clamp math would just duplicate engine internals. Asserting
    // "offset > 0 and bounded by the content height" captures the
    // observable contract instead.
    assert!(
        offset > 0.0,
        "scroll_offset should be > 0 after scrollToBottom (got {})",
        offset
    );
    assert!(
        offset <= content_h,
        "scroll_offset ({}) should never exceed content_height ({})",
        offset,
        content_h
    );

    h.resume().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns a real ui-demo game window; run with --ignored"]
async fn ui_demo_unknown_method_returns_error() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch ui-demo");

    // This game has no custom `handle_vdp`, so unknown methods fall through
    // to the engine's default. Either -32601 (method not found) or -32000
    // (handle_vdp said "Not implemented") is acceptable — both are valid
    // protocol behaviours.
    let resp = h
        .call("something.nonexistent", json!({}))
        .await
        .expect("RPC transport ok");
    let err = resp
        .get("error")
        .expect("unknown method must produce an error envelope");
    let code = err["code"].as_i64().unwrap();
    assert!(
        code == -32601 || code == -32000,
        "unexpected error code: {}",
        code
    );
}

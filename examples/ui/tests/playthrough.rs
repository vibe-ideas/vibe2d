//! Playthrough scenario for the UI demo — drives the game through a short
//! human-paced sequence so a video recorder (Xvfb + ffmpeg, see
//! `.github/workflows/playthrough-record.yml`) can capture a representative
//! "what does this PR actually look like" GIF.
//!
//! Unlike [`vdp_ui.rs`], this scenario deliberately does NOT pause the
//! engine — the recorder needs live frames. Each VDP action is followed by a
//! short sleep so the resulting GIF reads at human speed instead of looking
//! like a sped-up demo reel.
//!
//! Run locally with:
//!
//!     cargo test -p ui-demo --test playthrough -- --ignored --nocapture

use std::time::Duration;

use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "ui-demo";
// Matches `examples/ui/game.yaml` -> debug.vdp.port.
const VDP_PORT: u16 = 9230;

/// Roughly one human "beat" — long enough for the GIF reader's eye to track
/// what just happened. 700ms keeps the full scenario under 15 s while still
/// looking deliberate.
async fn beat() {
    tokio::time::sleep(Duration::from_millis(700)).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "demo recording — used by .github/workflows/playthrough-record.yml"]
async fn ui_demo_full_playthrough() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch ui-demo");

    // CI sets VIBE_TEST_RECORDING_DIR — this spawns a background task
    // that polls game.screenshot at 15fps and writes numbered PNGs.
    // Local runs are unaffected.
    let _recorder = h.start_recorder(GAME_PACKAGE, 15).await.ok().flatten();

    // Let the title / initial layout settle on screen.
    beat().await;
    beat().await;

    // Click the counter button a few times — viewer sees the "Clicks: N"
    // label tick up.
    for _ in 0..3 {
        h.ui_click("btn_click").await.unwrap();
        beat().await;
    }

    // Type and submit a CJK message — exercises both the text input widget
    // and the IME font path, which is one of the demo's main reasons to exist.
    h.ui_set_text("chat_input", "你好 vibe2d").await.unwrap();
    beat().await;
    h.ui_submit("chat_input").await.unwrap();
    beat().await;

    // Follow up with an English message so the scroll list visibly grows.
    h.ui_set_text("chat_input", "hello world").await.unwrap();
    beat().await;
    h.ui_submit("chat_input").await.unwrap();
    beat().await;

    // Fill enough lines to overflow the visible area, then scroll-to-bottom
    // so the GIF ends on the scroll animation.
    for i in 0..6 {
        h.ui_set_text("chat_input", &format!("line {}", i))
            .await
            .unwrap();
        h.ui_submit("chat_input").await.unwrap();
        // No `beat()` here — submit-spam happens fast on purpose so the GIF
        // shows a burst of activity, then we slow back down for the scroll.
    }
    beat().await;
    h.ui_scroll_to_bottom("msg_list").await.unwrap();
    beat().await;
    beat().await;
}

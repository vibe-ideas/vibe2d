//! Playthrough scenario for mari0 — walk right for a few seconds with
//! a couple of jumps so the GIF shows actual platforming motion.
//!
//! Used by `.github/workflows/playthrough.yml`. Does NOT pause the
//! engine — recording wants live frames.
//!
//! Run locally with:
//!
//!     cargo test -p mari0 --test playthrough -- --ignored --nocapture

use std::time::Duration;

use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "mari0";
// Matches `examples/mari0/game.yaml` -> debug.vdp.port.
const VDP_PORT: u16 = 9229;

async fn sleep(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "demo recording — used by .github/workflows/playthrough.yml"]
async fn mari0_playthrough() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch mari0");

    // CI sets VIBE_TEST_RECORDING_DIR — see `examples/ui/tests/playthrough.rs`
    // for the why (x11grab on Xvfb captures black; VDP screenshots don't).
    let _recorder = h.start_recorder(GAME_PACKAGE, 15).await.ok().flatten();

    // Show the title for a beat.
    sleep(800).await;

    // Walk right for ~1.2 s.
    h.simulate_key_press("Right").await.unwrap();
    sleep(1200).await;

    // Jump while still moving (Space). Hold direction across the jump.
    h.simulate_key_tap("Space").await.unwrap();
    sleep(900).await;

    h.simulate_key_tap("Space").await.unwrap();
    sleep(1200).await;

    // Another small hop, then keep walking.
    h.simulate_key_tap("Space").await.unwrap();
    sleep(1500).await;

    // Stop and let the last frame settle.
    h.simulate_key_release("Right").await.unwrap();
    sleep(1200).await;
}

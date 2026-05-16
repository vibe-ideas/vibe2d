//! Playthrough scenario for the AOI demo — let observers float around
//! naturally for a few seconds, then toggle distance-LOD via the `L`
//! action so the viewer sees the lit-set culling kick in.
//!
//! Used by `.github/workflows/playthrough.yml`. Unlike the assertion
//! tests in `vdp_aoi.rs`, this one does NOT pause the engine — the
//! recorder needs live frames to capture the motion.
//!
//! Run locally with:
//!
//!     cargo test -p aoi-demo --test playthrough -- --ignored --nocapture

use std::time::Duration;

use vibe_test::GameHarness;

const GAME_PACKAGE: &str = "aoi-demo";
// Matches `examples/aoi-demo/game.yaml` -> debug.vdp.port.
const VDP_PORT: u16 = 9232;

async fn beat() {
    tokio::time::sleep(Duration::from_millis(700)).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "demo recording — used by .github/workflows/playthrough.yml"]
async fn aoi_demo_playthrough() {
    let mut h = GameHarness::launch(GAME_PACKAGE, VDP_PORT)
        .await
        .expect("launch aoi-demo");

    // Let the observers free-roam and show off the lit-set lighting.
    for _ in 0..6 {
        beat().await;
    }

    // Tap `L` to flip the distance-LOD culling on, so the GIF shows the
    // far-field dots fade out — that's the demo's main pedagogical bit.
    h.simulate_key_tap("L").await.unwrap();
    for _ in 0..6 {
        beat().await;
    }

    // Toggle it back off so a viewer that catches the second half
    // sees what "no LOD" looks like as the contrast.
    h.simulate_key_tap("L").await.unwrap();
    for _ in 0..4 {
        beat().await;
    }
}

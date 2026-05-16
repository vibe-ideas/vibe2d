//! [`Recorder`] — background task that polls `game.screenshot` over VDP at
//! a fixed cadence and writes each frame as a numbered PNG.
//!
//! Used by playthrough tests to capture a GIF-able sequence without
//! depending on `xvfb-run` + `x11grab`, which silently produces black
//! frames under Xvfb + lavapipe (the wgpu surface presents to an X11
//! drawable that x11grab never sees). The screenshot path is a pure
//! `wgpu` texture readback inside the game process, so it sidesteps
//! the X11 display path entirely.
//!
//! Workflow contract: tests set `VIBE_TEST_RECORDING_DIR=<root>` in CI
//! and call [`GameHarness::start_recorder`] passing a package label.
//! The recorder writes to `<root>/<label>/0000.png`, `0001.png`, ...
//! at the requested FPS. After the playthrough's scenario completes,
//! the harness drops the recorder, which kills the polling task; CI
//! then assembles the PNGs into a GIF with ffmpeg.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use tokio::task::JoinHandle;

use crate::client::VdpClient;

pub struct Recorder {
    handle: JoinHandle<()>,
    /// Kept so a test can `println!` the path for the CI step that
    /// assembles the GIF without re-deriving it.
    pub dir: PathBuf,
}

impl Recorder {
    /// Spin up a recorder using its own VDP client connection. Returns
    /// `Ok(None)` when `VIBE_TEST_RECORDING_DIR` is unset — local runs
    /// don't want PNG spam in `/tmp`, so the env var is the opt-in.
    pub async fn start(addr: SocketAddr, label: &str, fps: u32) -> Result<Option<Self>> {
        let Ok(root) = std::env::var("VIBE_TEST_RECORDING_DIR") else {
            return Ok(None);
        };
        let dir = PathBuf::from(root).join(label);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create recording dir {}", dir.display()))?;
        // Separate VDP client so screenshot polling can interleave with
        // whatever the scenario's main client is doing.
        let mut client = VdpClient::connect(addr)
            .await
            .context("recorder VDP connect")?;

        let dir_for_task = dir.clone();
        let frame_period = Duration::from_millis((1000 / fps.max(1)) as u64);

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(frame_period);
            let mut frame: usize = 0;
            loop {
                ticker.tick().await;
                let path = dir_for_task.join(format!("{:04}.png", frame));
                if client
                    .call_ok("game.screenshot", json!({ "path": path.to_string_lossy() }))
                    .await
                    .is_err()
                {
                    // Game probably shut down — exit quietly. The harness
                    // teardown will land its own log if this matters.
                    break;
                }
                frame += 1;
            }
        });

        Ok(Some(Self { handle, dir }))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        // Drop the polling task; one screenshot worth of frames might
        // still be flushing on the game side, but the ffmpeg assembly
        // step in CI sleeps a beat before reading, which is enough.
        self.handle.abort();
    }
}

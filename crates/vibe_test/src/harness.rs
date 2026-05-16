//! [`GameHarness`] — owns a spawned game child process plus a connected
//! [`VdpClient`]. Plus its config struct [`LaunchOptions`] and the
//! TCP/WS probe helpers used to wait for the child to become VDP-ready.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use tokio::net::TcpStream;

use crate::client::VdpClient;

/// Owns a spawned game child process plus a connected [`VdpClient`].
///
/// The child process is killed and reaped when the harness drops, so each
/// test gets a clean slate — just make sure tests don't race on the same
/// VDP port (use `--test-threads=1`, or bind distinct ports in `game.yaml`).
pub struct GameHarness {
    child: Option<Child>,
    pub client: VdpClient,
    pub port: u16,
}

/// Options for [`GameHarness::launch_with`].
pub struct LaunchOptions<'a> {
    /// Workspace package name, e.g. `"ui-demo"` or `"flappy-bird"`.
    pub package: &'a str,
    /// VDP port the game is expected to listen on (must match `game.yaml`).
    pub port: u16,
    /// How long to wait for the game to become VDP-ready. Cold `cargo run`
    /// builds can take a while.
    pub ready_timeout: Duration,
    /// If `Some`, sets `CARGO_TARGET_DIR` for the child — useful in CI to
    /// reuse a shared build cache.
    pub target_dir: Option<PathBuf>,
    /// When `false` (default), the spawned game runs with `VIBE_HEADLESS=1`
    /// so its window is created but invisible. Set `true` via
    /// [`LaunchOptions::visible`] for human debugging where you want to
    /// actually see the game window.
    pub visible: bool,
    /// When `true`, the child is spawned with `cargo run --release`. CI
    /// running on software Vulkan (lavapipe) needs this to avoid timeouts;
    /// local debug iteration keeps the default `false`. Can also be flipped
    /// on globally by setting `VIBE_TEST_RELEASE=1` in the environment.
    pub release: bool,
}

impl<'a> LaunchOptions<'a> {
    pub fn new(package: &'a str, port: u16) -> Self {
        Self {
            package,
            port,
            ready_timeout: Duration::from_secs(180),
            target_dir: None,
            visible: false,
            release: env_flag("VIBE_TEST_RELEASE"),
        }
    }

    /// Show the game window for human-driven debugging. Tests run hidden by
    /// default — flip this on only when you want to watch the game.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Spawn the child with `cargo run --release`. Slow software-GPU CI
    /// (lavapipe) needs this; local debug runs leave it off.
    pub fn release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }
}

/// Read a bool-ish env var: any non-empty value other than `0`/`false` is `true`.
fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"),
        Err(_) => false,
    }
}

impl GameHarness {
    /// Launch a workspace package with default options and connect to its
    /// VDP port. The game is invoked via `cargo run -p <package>` so its
    /// compiled artifacts are reused from the workspace target cache.
    pub async fn launch(package: &str, port: u16) -> Result<Self> {
        Self::launch_with(LaunchOptions::new(package, port)).await
    }

    pub async fn launch_with(opts: LaunchOptions<'_>) -> Result<Self> {
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("run").arg("--quiet");
        if opts.release {
            cmd.arg("--release");
        }
        cmd.args(["-p", opts.package])
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(dir) = &opts.target_dir {
            cmd.env("CARGO_TARGET_DIR", dir);
        }
        if !opts.visible {
            cmd.env("VIBE_HEADLESS", "1");
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn `{}`", opts.package))?;

        let addr: SocketAddr = ([127, 0, 0, 1], opts.port).into();
        let client = wait_for_vdp(addr, opts.ready_timeout)
            .await
            .with_context(|| {
                format!(
                    "`{}` did not become VDP-ready on port {} within {:?}",
                    opts.package, opts.port, opts.ready_timeout
                )
            })?;

        Ok(Self {
            child: Some(child),
            client,
            port: opts.port,
        })
    }
}

impl std::ops::Deref for GameHarness {
    type Target = VdpClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl std::ops::DerefMut for GameHarness {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl Drop for GameHarness {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Poll until a WebSocket handshake succeeds and the VDP server answers
/// `engine.info` with a vibe2d identity — then return the open client.
async fn wait_for_vdp(addr: SocketAddr, timeout: Duration) -> Result<VdpClient> {
    let deadline = Instant::now() + timeout;
    loop {
        // Flatten the probe via guard clauses + `?` on a helper; the nested
        // `if let` chain it replaces would otherwise trip `clippy::collapsible_if`.
        if let Some(client) = try_handshake(addr).await {
            return Ok(client);
        }
        if Instant::now() >= deadline {
            return Err(anyhow!("timed out waiting for VDP at {}", addr));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// One probe attempt: TCP reachable → WS upgrade → `engine.info` identity check.
/// Returns `Some(client)` only when all three steps succeed and the identity
/// matches vibe2d. Any failure is swallowed so the caller can retry.
async fn try_handshake(addr: SocketAddr) -> Option<VdpClient> {
    TcpStream::connect(addr).await.ok()?;
    let mut client = VdpClient::connect(addr).await.ok()?;
    let info = client.engine_info().await.ok()?;
    let engine = info.get("engine").and_then(|v| v.as_str())?;
    (engine == "vibe2d").then_some(client)
}

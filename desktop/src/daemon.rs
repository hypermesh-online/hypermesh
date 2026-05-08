// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — Daemon subprocess management.
//
// Owns the lifecycle of the `hypermesh` binary as a child process:
//   - `start(args)` spawns it (graceful failure if not on $PATH)
//   - `stop()` sends SIGTERM, waits briefly, falls back to SIGKILL
//   - `status()` checks IPC ping first (authoritative), falls back to
//     child-handle liveness, returns Stopped if neither succeeds.
//
// For C.3 alpha the binary is assumed to be on $PATH (installed via
// scripts/install.sh / install.ps1 from C.1/C.2). The bundled-sidecar
// approach (`Resources/bin/hypermesh`) is documented as C.3.5 follow-up
// in `desktop/README.md`.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonState {
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub state: DaemonState,
    pub pid: Option<u32>,
    pub message: Option<String>,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonStartArgs {
    /// Privacy mode (Anonymous / Private / Public). Defaults to "private"
    /// when wizard hasn't been run yet.
    #[serde(default = "default_privacy")]
    pub privacy_mode: String,
    /// Optional network ID to join on launch.
    #[serde(default)]
    pub network_id: Option<String>,
    /// Foreground vs detached. The shell always runs the daemon in
    /// foreground so we can observe the child handle.
    #[serde(default = "default_true")]
    pub foreground: bool,
    /// Extra args, appended verbatim.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_privacy() -> String { "private".into() }
fn default_true() -> bool { true }

/// Shared daemon manager. Cloning is cheap (Arc-wrapped state).
#[derive(Clone)]
pub struct DaemonManager {
    inner: Arc<Mutex<Inner>>,
    socket_path: PathBuf,
    binary: String,
}

struct Inner {
    child: Option<Child>,
    last_state: DaemonState,
}

impl DaemonManager {
    pub fn new() -> Self {
        let socket_path = resolve_socket_path();
        let binary = std::env::var("HYPERMESH_BIN").unwrap_or_else(|_| "hypermesh".into());
        Self {
            inner: Arc::new(Mutex::new(Inner { child: None, last_state: DaemonState::Stopped })),
            socket_path,
            binary,
        }
    }

    #[allow(dead_code)] // Exposed for future tray tooltips / debug commands.
    pub fn socket_path(&self) -> &PathBuf { &self.socket_path }

    /// Start the daemon if not already running. Returns the resolved status.
    pub async fn start(&self, args: DaemonStartArgs) -> anyhow::Result<DaemonStatus> {
        // Quick short-circuit: if socket already responds, daemon is up.
        if self.ping().await {
            return Ok(self.status().await);
        }

        let mut guard = self.inner.lock().await;

        // If we already hold a child, reuse. Pull the alive flag + pid out
        // first to avoid holding a mutable borrow across the assignment to
        // `guard.last_state` below. (Pattern guards are immutable, so we
        // do the try_wait inside the arm body, not the guard.)
        let reuse: Option<u32> = if let Some(child) = guard.child.as_mut() {
            if child.try_wait().ok().flatten().is_none() {
                child.id()
            } else {
                None
            }
        } else {
            None
        };
        if let Some(pid) = reuse {
            guard.last_state = DaemonState::Running;
            return Ok(DaemonStatus {
                state: DaemonState::Running,
                pid: Some(pid),
                message: None,
                socket_path: self.socket_path.display().to_string(),
            });
        }

        // Build CLI invocation: `hypermesh connect <privacy> [--network <id>] [--foreground] ...`
        let mut cmd = Command::new(&self.binary);
        cmd.arg("connect").arg(&args.privacy_mode);
        if let Some(net) = &args.network_id {
            cmd.arg("--network").arg(net);
        }
        if args.foreground {
            cmd.arg("--foreground");
        }
        for extra in &args.extra_args {
            cmd.arg(extra);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let spawned = cmd.spawn();
        let child = match spawned {
            Ok(c) => c,
            Err(e) => {
                guard.last_state = DaemonState::Error;
                return Ok(DaemonStatus {
                    state: DaemonState::Error,
                    pid: None,
                    message: Some(format!(
                        "Failed to spawn `{}`: {}. Is it on PATH? Try `scripts/install.sh`.",
                        self.binary, e
                    )),
                    socket_path: self.socket_path.display().to_string(),
                });
            }
        };

        let pid = child.id();
        guard.child = Some(child);
        guard.last_state = DaemonState::Starting;

        Ok(DaemonStatus {
            state: DaemonState::Starting,
            pid,
            message: Some("Daemon starting; awaiting IPC socket".into()),
            socket_path: self.socket_path.display().to_string(),
        })
    }

    /// Stop the daemon (SIGTERM, then SIGKILL fallback after a short wait).
    pub async fn stop(&self) -> anyhow::Result<DaemonStatus> {
        let mut guard = self.inner.lock().await;
        if let Some(mut child) = guard.child.take() {
            // Try graceful first.
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    // Best-effort SIGTERM via libc.
                    unsafe { libc_kill(pid as i32, 15); }
                }
            }
            #[cfg(not(unix))]
            {
                let _ = child.start_kill();
            }

            // Wait up to 3s for clean exit.
            let _ = timeout(Duration::from_secs(3), child.wait()).await;

            // Force kill if still alive.
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill().await;
            }
        }
        guard.last_state = DaemonState::Stopped;
        Ok(DaemonStatus {
            state: DaemonState::Stopped,
            pid: None,
            message: None,
            socket_path: self.socket_path.display().to_string(),
        })
    }

    /// Resolve current daemon status. IPC ping is authoritative.
    pub async fn status(&self) -> DaemonStatus {
        if self.ping().await {
            let pid = self.inner.lock().await.child.as_ref().and_then(|c| c.id());
            return DaemonStatus {
                state: DaemonState::Running,
                pid,
                message: None,
                socket_path: self.socket_path.display().to_string(),
            };
        }

        let mut guard = self.inner.lock().await;
        // Same NLL workaround: collapse the mutable borrow into a small scope.
        enum ChildCheck { Alive(Option<u32>), Dead, None }
        let check = match guard.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => ChildCheck::Alive(child.id()),
                _ => ChildCheck::Dead,
            },
            None => ChildCheck::None,
        };
        match check {
            ChildCheck::Alive(pid) => {
                guard.last_state = DaemonState::Starting;
                return DaemonStatus {
                    state: DaemonState::Starting,
                    pid,
                    message: Some("Daemon process alive; IPC socket not yet ready".into()),
                    socket_path: self.socket_path.display().to_string(),
                };
            }
            ChildCheck::Dead => {
                guard.child = None;
            }
            ChildCheck::None => {}
        }
        guard.last_state = DaemonState::Stopped;
        DaemonStatus {
            state: DaemonState::Stopped,
            pid: None,
            message: None,
            socket_path: self.socket_path.display().to_string(),
        }
    }

    /// Try a JSON-RPC ping over the Unix socket. Returns true if the daemon
    /// answers. On non-Unix targets always returns false — desktop shell on
    /// Windows would need a named-pipe variant (out of scope for C.3 alpha).
    pub async fn ping(&self) -> bool {
        #[cfg(unix)]
        {
            ping_unix(&self.socket_path).await
        }
        #[cfg(not(unix))]
        {
            // TODO(C.3.5): Windows named pipe (\\.\pipe\hypermesh-ctl).
            false
        }
    }

    /// Try to invoke `system.check_update` over the IPC socket. Returns the
    /// raw JSON if available, or None on any error / no response. Used by
    /// the tray to surface the "Update available" item.
    pub async fn check_update(&self) -> Option<serde_json::Value> {
        #[cfg(unix)]
        {
            ipc_call_unix(&self.socket_path, "system.check_update", serde_json::json!({})).await
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}

fn resolve_socket_path() -> PathBuf {
    if let Ok(s) = std::env::var("HYPERMESH_SOCK") {
        return PathBuf::from(s);
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".hypermesh").join("ctl.sock");
        if p.exists() { return p; }
    }
    // Production install fallback (matches CLAUDE.md note).
    PathBuf::from("/var/lib/hypermesh/.hypermesh/ctl.sock")
}

#[cfg(unix)]
async fn ping_unix(path: &PathBuf) -> bool {
    if !path.exists() { return false; }
    let connect = timeout(Duration::from_millis(500), UnixStream::connect(path)).await;
    let Ok(Ok(mut stream)) = connect else { return false; };

    // JSON-RPC 2.0 ping. The daemon's IPC dispatcher accepts arbitrary
    // method names; "system.ping" or "core.health" are equivalent — use a
    // method we know is universally present. If no handler matches, the
    // dispatcher returns a structured JSON-RPC error which is still a
    // successful round-trip and counts as "alive".
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "core.health",
        "params": {}
    });
    let line = format!("{}\n", req);
    if stream.write_all(line.as_bytes()).await.is_err() { return false; }
    let mut buf = [0u8; 1024];
    matches!(timeout(Duration::from_millis(500), stream.read(&mut buf)).await, Ok(Ok(n)) if n > 0)
}

#[cfg(unix)]
async fn ipc_call_unix(path: &PathBuf, method: &str, params: serde_json::Value)
    -> Option<serde_json::Value>
{
    if !path.exists() { return None; }
    let mut stream = timeout(Duration::from_millis(500), UnixStream::connect(path)).await.ok()?.ok()?;
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let line = format!("{}\n", req);
    stream.write_all(line.as_bytes()).await.ok()?;
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        let read = timeout(Duration::from_millis(800), stream.read(&mut chunk)).await.ok()?.ok()?;
        if read == 0 { break; }
        buf.extend_from_slice(&chunk[..read]);
        if buf.contains(&b'\n') { break; }
    }
    let line = std::str::from_utf8(&buf).ok()?;
    let resp: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    resp.get("result").cloned()
}

// Minimal SIGTERM helper to avoid pulling in a fuller libc dependency.
#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}
#[cfg(unix)]
#[allow(non_snake_case)]
unsafe fn libc_kill(pid: i32, sig: i32) {
    let _ = kill(pid, sig);
}

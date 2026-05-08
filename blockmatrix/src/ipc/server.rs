// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Unix domain socket server for the HyperMesh daemon.
//!
//! Listens on a Unix socket and dispatches JSON-RPC 2.0 requests
//! to the registered [`RequestHandler`].

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{
    self, protocol_versions_compatible, RpcRequest, RpcResponse, IPC_PROTOCOL_VERSION,
    INTERNAL_ERROR, PROTOCOL_VERSION_MISMATCH,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::watch;

/// IPC server that listens on a Unix domain socket.
#[derive(Debug)]
pub struct IpcServer {
    socket_path: PathBuf,
    handler: Arc<RequestHandler>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl IpcServer {
    /// Create a new server using the default socket path.
    pub fn new(handler: Arc<RequestHandler>) -> Result<Self> {
        let socket_path = protocol::socket_path();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Ok(Self {
            socket_path,
            handler,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Create a server bound to a specific socket path (for testing).
    pub fn with_path(handler: Arc<RequestHandler>, socket_path: PathBuf) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Ok(Self {
            socket_path,
            handler,
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Run the server, accepting connections until shutdown is signalled.
    pub async fn run(&self) -> Result<()> {
        self.prepare_socket_path()
            .context("failed to prepare socket path")?;

        let listener = UnixListener::bind(&self.socket_path)
            .context("failed to bind unix socket")?;

        self.write_pid_file()?;

        tracing::info!(path = %self.socket_path.display(), "IPC server listening");

        let mut shutdown_rx = self.shutdown_rx.clone();

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _addr)) => {
                            tracing::debug!("IPC client connected");
                            let handler = Arc::clone(&self.handler);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, handler).await {
                                    tracing::debug!(error = %e, "IPC connection ended");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to accept IPC connection");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("IPC server shutting down");
                        break;
                    }
                }
            }
        }

        self.cleanup();
        Ok(())
    }

    /// Signal the server to shut down.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Path where the server socket is bound.
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Compute the PID file path as a sibling to the socket path.
    fn pid_path(&self) -> PathBuf {
        let mut p = self.socket_path.clone();
        p.set_file_name("daemon.pid");
        p
    }

    /// Ensure the parent directory exists and remove stale socket files.
    fn prepare_socket_path(&self) -> Result<()> {
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create socket directory")?;
        }

        if self.socket_path.exists() {
            let pid_path = self.pid_path();
            let stale = if pid_path.exists() {
                let pid_str = std::fs::read_to_string(&pid_path).unwrap_or_default();
                let pid: u32 = pid_str.trim().parse().unwrap_or(0);
                pid == 0 || !process_alive(pid)
            } else {
                true
            };

            if stale {
                tracing::debug!("removing stale socket file");
                let _ = std::fs::remove_file(&self.socket_path);
                let _ = std::fs::remove_file(&pid_path);
            } else {
                anyhow::bail!(
                    "socket already exists and owning process is alive: {}",
                    self.socket_path.display()
                );
            }
        }

        Ok(())
    }

    /// Write current process PID to the pid file.
    fn write_pid_file(&self) -> Result<()> {
        let pid_path = self.pid_path();
        std::fs::write(&pid_path, std::process::id().to_string())
            .context("failed to write PID file")?;
        Ok(())
    }

    /// Remove socket and PID files on shutdown.
    fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.pid_path());
        tracing::debug!("IPC socket and PID file cleaned up");
    }
}

/// Handle a single client connection: read lines, dispatch, write responses.
async fn handle_connection(
    stream: tokio::net::UnixStream,
    handler: Arc<RequestHandler>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = buf_reader
            .read_line(&mut line)
            .await
            .context("failed to read from IPC client")?;

        if bytes_read == 0 {
            tracing::debug!("IPC client disconnected");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<RpcRequest>(trimmed) {
            Ok(request) => {
                // Phase J.1 — gate on major-version compatibility. Old
                // clients without `protocol_version` are accepted
                // (forward-compat with pre-J.1 nodes); new clients with
                // a mismatched major version are rejected with a
                // helpful upgrade hint.
                if let Some(ref client_v) = request.protocol_version {
                    if !protocol_versions_compatible(client_v, IPC_PROTOCOL_VERSION) {
                        let msg = format!(
                            "incompatible CLI protocol version: client {} vs daemon {} (major \
                             versions differ; run `hypermesh update` to upgrade the client)",
                            client_v, IPC_PROTOCOL_VERSION
                        );
                        RpcResponse::error(request.id, PROTOCOL_VERSION_MISMATCH, msg)
                    } else {
                        handler.dispatch(request).await
                    }
                } else {
                    handler.dispatch(request).await
                }
            }
            Err(e) => RpcResponse::error(0, INTERNAL_ERROR, format!("parse error: {e}")),
        };

        let mut response_bytes =
            serde_json::to_vec(&response).context("failed to serialize response")?;
        response_bytes.push(b'\n');
        writer
            .write_all(&response_bytes)
            .await
            .context("failed to write response")?;
    }

    Ok(())
}

/// Check whether a process with the given PID is alive.
fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: kill(pid, 0) only checks process existence, sends no signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::client::IpcClient;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn test_socket_path(dir: &TempDir) -> PathBuf {
        dir.path().join("test.sock")
    }

    fn make_ping_handler() -> Arc<RequestHandler> {
        let mut handler = RequestHandler::new();
        handler.register(
            "ping",
            Arc::new(|_params| Box::pin(async { Ok(serde_json::json!("pong")) })),
        );
        Arc::new(handler)
    }

    /// Wait for a socket file to appear and be connectable, with timeout.
    async fn wait_for_socket(path: &PathBuf) {
        for _ in 0..200 {
            if path.exists() {
                // Socket file exists — try connecting to confirm listener is ready
                for _ in 0..20 {
                    if tokio::net::UnixStream::connect(path).await.is_ok() {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(path.exists(), "test: socket did not appear within timeout: {}", path.display());
    }

    #[tokio::test]
    async fn test_server_client_round_trip() {
        let tmp = TempDir::new().expect("test: tempdir");
        let sock = test_socket_path(&tmp);

        let handler = make_ping_handler();
        let server = Arc::new(
            IpcServer::with_path(handler, sock.clone()).expect("test: create server"),
        );

        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let _ = server_clone.run().await;
        });

        wait_for_socket(&sock).await;

        let client = IpcClient::with_path(sock.clone());
        let resp = client
            .call("ping", serde_json::json!(null))
            .await
            .expect("test: call ping");
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result.expect("test: result"),
            serde_json::json!("pong")
        );

        server.shutdown();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_server_shutdown_cleans_socket() {
        let tmp = TempDir::new().expect("test: tempdir");
        let sock = test_socket_path(&tmp);

        let handler = make_ping_handler();
        let server = Arc::new(
            IpcServer::with_path(handler, sock.clone()).expect("test: create server"),
        );

        let server_clone = Arc::clone(&server);
        let run_handle = tokio::spawn(async move {
            let _ = server_clone.run().await;
        });

        wait_for_socket(&sock).await;
        assert!(sock.exists(), "test: socket should exist while running");

        server.shutdown();
        let _ = run_handle.await;

        assert!(!sock.exists(), "test: socket should be removed after shutdown");
    }

    #[tokio::test]
    async fn test_client_error_when_no_daemon() {
        let tmp = TempDir::new().expect("test: tempdir");
        let sock = tmp.path().join("nonexistent.sock");

        let client = IpcClient::with_path(sock);
        let result = client.call("ping", serde_json::json!(null)).await;
        assert!(result.is_err(), "test: should fail when no daemon running");
    }
}

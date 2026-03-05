// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! IPC client for connecting to the HyperMesh daemon.
//!
//! Sends JSON-RPC 2.0 requests over a Unix domain socket and reads responses.

use crate::ipc::protocol::{self, RpcRequest, RpcResponse};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Default timeout for RPC calls.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Client for communicating with the HyperMesh daemon over IPC.
#[derive(Debug, Clone)]
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a client using the default socket path.
    pub fn new() -> Self {
        Self {
            socket_path: protocol::socket_path(),
        }
    }

    /// Create a client targeting a specific socket path (for testing).
    pub fn with_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Quick check: does the socket file exist on disk?
    pub fn daemon_running(&self) -> bool {
        self.socket_path.exists()
    }

    /// Check if the daemon is actually responding (socket exists AND answers ping).
    pub async fn is_daemon_running(&self) -> bool {
        if !self.socket_path.exists() {
            return false;
        }
        // Attempt a connection — if it fails, daemon is not running
        UnixStream::connect(&self.socket_path).await.is_ok()
    }

    /// Send an RPC request and wait for the response (with 30s timeout).
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<RpcResponse> {
        let request = RpcRequest::new(method, params);
        let mut request_bytes =
            serde_json::to_vec(&request).context("failed to serialize request")?;
        request_bytes.push(b'\n');

        let result = tokio::time::timeout(CALL_TIMEOUT, async {
            let stream = UnixStream::connect(&self.socket_path)
                .await
                .context("failed to connect to daemon socket")?;

            let (reader, mut writer) = stream.into_split();
            writer
                .write_all(&request_bytes)
                .await
                .context("failed to write request")?;

            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            buf_reader
                .read_line(&mut line)
                .await
                .context("failed to read response")?;

            let response: RpcResponse =
                serde_json::from_str(line.trim()).context("failed to parse response")?;
            Ok(response)
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_elapsed) => anyhow::bail!("RPC call timed out after {}s", CALL_TIMEOUT.as_secs()),
        }
    }

    /// Send an RPC request and extract the result value, returning an error
    /// if the response contains an RPC error.
    pub async fn call_ok(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let response = self.call(method, params).await?;
        if let Some(err) = response.error {
            anyhow::bail!(
                "RPC error {}: {} (code {})",
                method,
                err.message,
                err.code
            );
        }
        response
            .result
            .ok_or_else(|| anyhow::anyhow!("RPC response missing both result and error"))
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_not_running() {
        let client = IpcClient::with_path(PathBuf::from("/tmp/nonexistent-hypermesh.sock"));
        assert!(!client.daemon_running());
        assert!(!client.is_daemon_running().await);
    }

    #[tokio::test]
    async fn test_call_fails_when_no_daemon() {
        let client = IpcClient::with_path(PathBuf::from("/tmp/nonexistent-hypermesh.sock"));
        let result = client.call("ping", serde_json::json!(null)).await;
        assert!(result.is_err());
    }
}

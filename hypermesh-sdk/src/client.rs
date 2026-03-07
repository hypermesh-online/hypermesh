// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! HyperMesh daemon client over Unix domain sockets.

use crate::api;
use crate::error::SdkError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Default timeout for RPC calls.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How to connect to the HyperMesh daemon.
#[derive(Debug, Clone)]
pub enum ConnectionMode {
    /// Connect via a local Unix domain socket.
    Local {
        /// Override the socket path. `None` uses the standard 3-tier fallback.
        socket_path: Option<PathBuf>,
    },
}

type StreamPair = (
    BufReader<tokio::io::ReadHalf<UnixStream>>,
    tokio::io::WriteHalf<UnixStream>,
);

/// Async client for the HyperMesh daemon IPC protocol.
#[derive(Debug)]
pub struct HyperMeshClient {
    stream: Option<tokio::sync::Mutex<StreamPair>>,
    next_id: AtomicU64,
}

/// JSON-RPC 2.0 request (internal).
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: &'a serde_json::Value,
}

/// JSON-RPC 2.0 response (internal).
#[derive(Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<RpcErrorObj>,
}

/// JSON-RPC 2.0 error object (internal).
#[derive(Deserialize)]
struct RpcErrorObj {
    code: i64,
    message: String,
}

impl HyperMeshClient {
    /// Connect to the daemon using the given mode.
    pub async fn connect(mode: ConnectionMode) -> Result<Self, SdkError> {
        let path = match mode {
            ConnectionMode::Local { socket_path } => {
                socket_path.unwrap_or_else(resolve_socket_path)
            }
        };

        let stream = UnixStream::connect(&path).await.map_err(|e| {
            SdkError::Connection(format!("{}: {}", path.display(), e))
        })?;

        let (reader, writer) = tokio::io::split(stream);
        let pair = (BufReader::new(reader), writer);

        Ok(Self {
            stream: Some(tokio::sync::Mutex::new(pair)),
            next_id: AtomicU64::new(1),
        })
    }

    /// Connect to the daemon using the default socket path.
    pub async fn connect_local() -> Result<Self, SdkError> {
        Self::connect(ConnectionMode::Local { socket_path: None }).await
    }

    /// Returns `true` if the client holds an active connection handle.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Drop the connection to the daemon.
    pub async fn disconnect(&mut self) {
        self.stream = None;
    }

    /// Send a raw JSON-RPC call and return the result value.
    ///
    /// Returns [`SdkError::Rpc`] when the daemon returns an error response,
    /// [`SdkError::Timeout`] when the call exceeds 30 seconds.
    pub async fn raw_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        let pair_mutex = self.stream.as_ref().ok_or(SdkError::NotConnected)?;

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = RpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params: &params,
        };

        let mut line = serde_json::to_string(&request)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        line.push('\n');

        let result = tokio::time::timeout(DEFAULT_TIMEOUT, async {
            let mut guard = pair_mutex.lock().await;
            let (ref mut reader, ref mut writer) = *guard;

            writer
                .write_all(line.as_bytes())
                .await
                .map_err(|e| SdkError::Ipc(e.to_string()))?;
            writer
                .flush()
                .await
                .map_err(|e| SdkError::Ipc(e.to_string()))?;

            let mut response_line = String::new();
            reader
                .read_line(&mut response_line)
                .await
                .map_err(|e| SdkError::Ipc(e.to_string()))?;

            if response_line.is_empty() {
                return Err(SdkError::Ipc("daemon closed the connection".into()));
            }

            let resp: RpcResponse = serde_json::from_str(&response_line)
                .map_err(|e| SdkError::Serialization(e.to_string()))?;

            if let Some(err) = resp.error {
                return Err(SdkError::Rpc {
                    code: err.code,
                    message: err.message,
                });
            }

            resp.result
                .ok_or_else(|| SdkError::Ipc("response has neither result nor error".into()))
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(SdkError::Timeout(DEFAULT_TIMEOUT)),
        }
    }

    /// Access the node status API.
    pub fn node(&self) -> api::node::NodeApi<'_> {
        api::node::NodeApi { client: self }
    }

    /// Access the DNS API.
    pub fn dns(&self) -> api::dns::DnsApi<'_> {
        api::dns::DnsApi { client: self }
    }

    /// Access the asset API.
    pub fn asset(&self) -> api::asset::AssetApi<'_> {
        api::asset::AssetApi { client: self }
    }

    /// Access the network API.
    pub fn network(&self) -> api::network::NetworkApi<'_> {
        api::network::NetworkApi { client: self }
    }

    /// Access the blockchain API.
    pub fn blockchain(&self) -> api::blockchain::BlockchainApi<'_> {
        api::blockchain::BlockchainApi { client: self }
    }

    /// Access the topology API.
    pub fn topology(&self) -> api::topology::TopologyApi<'_> {
        api::topology::TopologyApi { client: self }
    }

    /// Access the domain API.
    pub fn domain(&self) -> api::domain::DomainApi<'_> {
        api::domain::DomainApi { client: self }
    }

    /// Access the dashboard API.
    pub fn dashboard(&self) -> api::dashboard::DashboardApi<'_> {
        api::dashboard::DashboardApi { client: self }
    }

    /// Access the config API.
    pub fn config(&self) -> api::config::ConfigApi<'_> {
        api::config::ConfigApi { client: self }
    }
}

/// Resolve the Unix socket path using the 3-tier fallback:
/// 1. `$HYPERMESH_SOCK`
/// 2. `$XDG_RUNTIME_DIR/hypermesh/ctl.sock`
/// 3. `~/.hypermesh/ctl.sock`
fn resolve_socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("HYPERMESH_SOCK") {
        return PathBuf::from(path);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("hypermesh")
            .join("ctl.sock");
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".hypermesh").join("ctl.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_nonexistent_socket_returns_connection_error() {
        let result = HyperMeshClient::connect(ConnectionMode::Local {
            socket_path: Some(PathBuf::from("/tmp/hypermesh-sdk-test-nonexistent.sock")),
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SdkError::Connection(_)),
            "expected Connection error, got: {err}"
        );
    }

    #[test]
    fn disconnected_client_reports_not_connected() {
        let client = HyperMeshClient {
            stream: None,
            next_id: AtomicU64::new(1),
        };
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn raw_call_without_connection_returns_not_connected() {
        let client = HyperMeshClient {
            stream: None,
            next_id: AtomicU64::new(1),
        };
        let result = client.raw_call("status", serde_json::json!({})).await;
        assert!(matches!(result, Err(SdkError::NotConnected)));
    }

    #[tokio::test]
    async fn mock_server_round_trip() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let sock_path = dir.path().join("test.sock");

        let listener = tokio::net::UnixListener::bind(&sock_path).expect("test: bind");

        let path = sock_path.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            let (reader, mut writer) = tokio::io::split(stream);
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            buf_reader
                .read_line(&mut line)
                .await
                .expect("test: read request");

            let req: serde_json::Value =
                serde_json::from_str(&line).expect("test: parse request");
            let id = req["id"].as_u64().expect("test: id");

            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"node_id": "test-node", "chain_height": 42}
            });
            let mut resp_line = serde_json::to_string(&resp).expect("test: serialize");
            resp_line.push('\n');
            writer
                .write_all(resp_line.as_bytes())
                .await
                .expect("test: write response");
            writer.flush().await.expect("test: flush");
        });

        let client = HyperMeshClient::connect(ConnectionMode::Local {
            socket_path: Some(path),
        })
        .await
        .expect("test: connect");

        let result = client
            .raw_call("status", serde_json::json!({}))
            .await
            .expect("test: raw_call");
        assert_eq!(result["node_id"], "test-node");
        assert_eq!(result["chain_height"], 42);

        server.await.expect("test: server join");
    }

    #[tokio::test]
    async fn mock_server_rpc_error() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let sock_path = dir.path().join("err.sock");

        let listener = tokio::net::UnixListener::bind(&sock_path).expect("test: bind");

        let path = sock_path.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            let (reader, mut writer) = tokio::io::split(stream);
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            buf_reader
                .read_line(&mut line)
                .await
                .expect("test: read request");

            let req: serde_json::Value =
                serde_json::from_str(&line).expect("test: parse request");
            let id = req["id"].as_u64().expect("test: id");

            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "method not found"}
            });
            let mut resp_line = serde_json::to_string(&resp).expect("test: serialize");
            resp_line.push('\n');
            writer
                .write_all(resp_line.as_bytes())
                .await
                .expect("test: write");
            writer.flush().await.expect("test: flush");
        });

        let client = HyperMeshClient::connect(ConnectionMode::Local {
            socket_path: Some(path),
        })
        .await
        .expect("test: connect");

        let result = client.raw_call("bad.method", serde_json::json!({})).await;
        assert!(matches!(
            result,
            Err(SdkError::Rpc {
                code: -32601,
                ..
            })
        ));

        server.await.expect("test: server join");
    }
}

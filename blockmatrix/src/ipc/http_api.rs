// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Lightweight HTTP API server for dashboard access.
//!
//! Serves dashboard content from **blockchain-registered Dashboard assets**.
//! The active dashboard is resolved by scanning the blockchain for the most
//! recent `Dashboard` asset registration, then loading its bundle from the
//! asset store (keyed by BLAKE3 content hash). `/api/v1/*` requests are
//! translated into IPC handler calls.
//!
//! Binds to `[::1]:<port>` (localhost IPv6 only). The gateway at
//! `trust.hypermesh.online` proxies external HTTP traffic here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{debug, warn};

use crate::blockchain::node_chain::NodeBlockchain;
use crate::dashboard::deploy;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::RpcRequest;

/// Default port for the HTTP API server.
pub const DEFAULT_HTTP_API_PORT: u16 = 9293;

/// CORS headers applied to every response.
const CORS_HEADERS: &str = "\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type\r\n\
Access-Control-Max-Age: 86400";

/// Start the HTTP API server, translating REST requests to IPC calls.
///
/// Serves dashboard content from blockchain-registered Dashboard assets for
/// non-API paths, and routes `/api/v1/*` to IPC handlers.
///
/// Binds to `[::1]:<port>` and runs until the future is dropped.
pub async fn run_http_api(
    handler: Arc<RequestHandler>,
    port: u16,
    data_dir: PathBuf,
    blockchain: Arc<NodeBlockchain>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Bind to all interfaces so external visitors can reach the dashboard
    let addr = format!("[::]:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("HTTP API listening on {}", addr);

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("HTTP accept error: {}", e);
                continue;
            }
        };

        let handler = handler.clone();
        let data_dir = data_dir.clone();
        let blockchain = blockchain.clone();
        let is_local = peer.ip().is_loopback();
        tokio::spawn(async move {
            if let Err(e) =
                handle_connection(stream, &handler, &data_dir, &blockchain, is_local).await
            {
                debug!("HTTP connection from {} error: {}", peer, e);
            }
        });
    }
}

/// Parsed HTTP request (method, path, body).
struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

/// Parse an HTTP/1.1 request from a TCP stream.
///
/// Reads the request line, headers (to find Content-Length), then body.
async fn parse_request(
    stream: &mut BufReader<tokio::net::TcpStream>,
) -> Result<HttpRequest, Box<dyn std::error::Error + Send + Sync>> {
    // Read request line: "GET /path HTTP/1.1\r\n"
    let mut request_line = String::new();
    stream.read_line(&mut request_line).await?;
    let request_line = request_line.trim_end();

    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err("malformed request line".into());
    }

    let method = parts[0].to_uppercase();
    let path = parts[1].to_string();

    // Read headers, extract Content-Length
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        stream.read_line(&mut line).await?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(val) = trimmed.strip_prefix("Content-Length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        }
        // Case-insensitive header match
        if let Some(val) = trimmed.strip_prefix("content-length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        }
    }

    // Read body if present
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(stream, &mut buf).await?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    Ok(HttpRequest { method, path, body })
}

/// Route an HTTP request to the corresponding IPC method and params.
///
/// Returns `(ipc_method, params_json)` or `None` if the path is unrecognized.
fn route(req: &HttpRequest) -> Option<(String, serde_json::Value)> {
    let path = req.path.split('?').next().unwrap_or(&req.path);
    let segments: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // All routes live under /api/v1/...
    if segments.len() < 3 || segments[0] != "api" || segments[1] != "v1" {
        return None;
    }

    let rest = &segments[2..];

    match (req.method.as_str(), rest) {
        // Core
        ("GET", ["status"]) => Some(("status".into(), serde_json::json!(null))),
        ("GET", ["ping"]) => Some(("ping".into(), serde_json::json!(null))),

        // Blockchain
        ("GET", ["blockchain", "height"]) => {
            Some(("blockchain.height".into(), serde_json::json!(null)))
        }
        ("GET", ["blockchain", "block", index]) => {
            if let Ok(n) = index.parse::<u64>() {
                Some(("blockchain.block".into(), serde_json::json!({"index": n})))
            } else {
                None
            }
        }
        ("GET", ["blockchain", "validate"]) => {
            Some(("blockchain.validate".into(), serde_json::json!(null)))
        }

        // DNS
        ("GET", ["dns", "list"]) => Some(("dns.list".into(), serde_json::json!(null))),
        ("GET", ["dns", "resolve", name]) => {
            Some(("dns.resolve".into(), serde_json::json!({"name": *name})))
        }
        ("POST", ["dns", "register"]) => {
            let params = serde_json::from_str(&req.body).unwrap_or(serde_json::json!(null));
            Some(("dns.register".into(), params))
        }

        // Network
        ("GET", ["network", "peers"]) => {
            Some(("network.peers".into(), serde_json::json!(null)))
        }

        // Topology
        ("GET", ["topology", "info"]) => {
            Some(("topology.info".into(), serde_json::json!(null)))
        }
        ("GET", ["topology", "neighbors"]) => {
            Some(("topology.neighbors".into(), serde_json::json!(null)))
        }

        // Assets
        ("GET", ["asset", "list"]) => Some(("asset.list".into(), serde_json::json!(null))),

        // Dashboard
        ("GET", ["dashboard", "list"]) => {
            Some(("dashboard.list".into(), serde_json::json!(null)))
        }
        ("GET", ["dashboard", "info"]) => {
            Some(("dashboard.info".into(), serde_json::json!({})))
        }

        // Config
        ("GET", ["config", "show"]) => Some(("config.show".into(), serde_json::json!(null))),
        ("GET", ["config", "get", key]) => {
            Some(("config.get".into(), serde_json::json!({"key": *key})))
        }

        // Domain
        ("GET", ["domain", "list"]) => Some(("domain.list".into(), serde_json::json!(null))),
        ("POST", ["domain", "register"]) => {
            let params = serde_json::from_str(&req.body).unwrap_or(serde_json::json!(null));
            Some(("domain.register".into(), params))
        }
        ("POST", ["domain", "join"]) => {
            let params = serde_json::from_str(&req.body).unwrap_or(serde_json::json!(null));
            Some(("domain.join".into(), params))
        }

        _ => None,
    }
}

/// Guess MIME content type from a file extension.
fn guess_content_type(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Write an HTTP response with a custom content type (for static file serving).
async fn write_static_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let header = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         {}\r\n\
         Cache-Control: public, max-age=3600\r\n\
         Connection: close\r\n\
         \r\n",
        status,
        status_text,
        content_type,
        body.len(),
        CORS_HEADERS,
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Write a complete HTTP JSON response to the stream.
async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    status_text: &str,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let header = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         {}\r\n\
         Connection: close\r\n\
         \r\n",
        status,
        status_text,
        body.len(),
        CORS_HEADERS,
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Load the active dashboard's file map from the blockchain asset store.
///
/// Queries the blockchain for the most recent Dashboard asset, loads the
/// bundle by content hash, and unbundles into a file map.
///
/// Returns `None` if no dashboard is registered or the bundle is missing.
async fn load_active_dashboard(
    data_dir: &Path,
    blockchain: &NodeBlockchain,
) -> Option<BTreeMap<String, Vec<u8>>> {
    let chain = blockchain.get_chain().await;
    let (content_hash, _block_idx) = deploy::find_active_dashboard(&chain)?;
    let (_manifest, bundle) = deploy::load_dashboard_bundle(data_dir, &content_hash)?;
    deploy::unbundle_files(&bundle)
}

/// Serve a dashboard file from a pre-loaded file map, with SPA fallback.
///
/// Returns `true` if a response was written.
async fn serve_from_bundle(
    stream: &mut tokio::net::TcpStream,
    files: &BTreeMap<String, Vec<u8>>,
    scope: &str,
    file_path: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let clean_path = file_path.trim_start_matches('/');
    let scoped_path = if clean_path.is_empty() {
        format!("{scope}/index.html")
    } else {
        format!("{scope}/{clean_path}")
    };

    // Try exact file match
    if let Some(content) = files.get(&scoped_path) {
        let content_type = guess_content_type(Path::new(&scoped_path));
        write_static_response(stream, 200, "OK", &content_type, content).await?;
        return Ok(true);
    }

    // SPA fallback: serve scope's index.html
    let index_path = format!("{scope}/index.html");
    if let Some(content) = files.get(&index_path) {
        write_static_response(stream, 200, "OK", "text/html; charset=utf-8", content).await?;
        return Ok(true);
    }

    Ok(false)
}

/// Handle a single HTTP connection: parse, route, dispatch, respond.
async fn handle_connection(
    tcp_stream: tokio::net::TcpStream,
    handler: &RequestHandler,
    data_dir: &Path,
    blockchain: &NodeBlockchain,
    is_local: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = BufReader::new(tcp_stream);
    let req = parse_request(&mut reader).await?;

    // Recover the underlying TcpStream for writing
    let mut stream = reader.into_inner();

    // Handle CORS preflight
    if req.method == "OPTIONS" {
        let header = format!(
            "HTTP/1.1 204 No Content\r\n\
             {}\r\n\
             Content-Length: 0\r\n\
             Connection: close\r\n\
             \r\n",
            CORS_HEADERS,
        );
        stream.write_all(header.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    // Serve dashboard files from blockchain asset store for non-API GET requests
    if req.method == "GET" && !req.path.starts_with("/api/") {
        // Load active dashboard from blockchain
        let files = load_active_dashboard(data_dir, blockchain).await;

        if let Some(files) = files {
            // Scope routing:
            // - /public/* always serves public scope
            // - /private/* always serves private scope (node management UI)
            // - Root path: local connections get private (node operator),
            //   external connections get public (visitor-facing)
            let (scope, file_path) = if req.path.starts_with("/public") {
                let rest = req.path.strip_prefix("/public").unwrap_or("/");
                let rest = if rest.is_empty() || rest == "/" {
                    "/index.html".to_string()
                } else {
                    rest.to_string()
                };
                ("public", rest)
            } else if req.path.starts_with("/private") {
                let rest = req.path.strip_prefix("/private").unwrap_or("/");
                let rest = if rest.is_empty() || rest == "/" {
                    "/index.html".to_string()
                } else {
                    rest.to_string()
                };
                ("private", rest)
            } else {
                // Default scope depends on connection origin
                let default_scope = if is_local { "private" } else { "public" };
                let file_path = if req.path == "/" {
                    "/index.html".to_string()
                } else {
                    req.path.clone()
                };
                (default_scope, file_path)
            };

            if serve_from_bundle(&mut stream, &files, scope, &file_path).await? {
                return Ok(());
            }
        }

        // No dashboard asset registered
        let body =
            b"{\"error\":\"No dashboard installed. Deploy with: hypermesh dashboard deploy <path>\"}";
        write_response(&mut stream, 404, "Not Found", body).await?;
        return Ok(());
    }

    // Route HTTP path to IPC method
    let (ipc_method, params) = match route(&req) {
        Some(r) => r,
        None => {
            let body = serde_json::json!({
                "error": "not found",
                "path": req.path,
            });
            let body_bytes = serde_json::to_vec(&body)?;
            write_response(&mut stream, 404, "Not Found", &body_bytes).await?;
            return Ok(());
        }
    };

    // Dispatch to IPC handler
    let rpc_request = RpcRequest::new(&ipc_method, params);
    let rpc_response = handler.dispatch(rpc_request).await;

    // Convert RPC response to HTTP response
    let (status, status_text, body_json) = if let Some(ref err) = rpc_response.error {
        let status = match err.code {
            -32601 => 404, // method not found
            -32602 => 400, // invalid params
            _ => 500,
        };
        let text = match status {
            404 => "Not Found",
            400 => "Bad Request",
            _ => "Internal Server Error",
        };
        (status, text, serde_json::to_vec(&rpc_response)?)
    } else {
        // Return just the result field for cleaner API responses
        let result = rpc_response.result.unwrap_or(serde_json::json!(null));
        (200, "OK", serde_json::to_vec(&result)?)
    };

    write_response(&mut stream, status, status_text, &body_json).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn test_blockchain() -> Arc<NodeBlockchain> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        Arc::new(NodeBlockchain::new(coord))
    }

    /// Register a dashboard asset on the blockchain and store its bundle.
    async fn register_test_dashboard(
        blockchain: &NodeBlockchain,
        data_dir: &Path,
        files: &BTreeMap<String, Vec<u8>>,
    ) {
        use crate::assets::core::{
            AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
        };
        use crate::StateProof;

        let bundle = deploy::bundle_files(files);
        let manifest_toml = r#"[dashboard]
name = "test"
version = "1.0.0"
description = "Test"
domain = "test.hypermesh"

[access]
public = "public"
private = "private"
"#;
        let asset_data = AssetData {
            config: b"DASHBOARD:DEPLOY:test".to_vec(),
            definition: bundle.clone(),
            metadata: manifest_toml.as_bytes().to_vec(),
        };
        let registration = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Dashboard),
        );
        let content_hash = registration.content_hash;
        let state_proof = StateProof::new_for_testing();
        blockchain
            .register_asset_record(registration, &state_proof)
            .await
            .expect("test: register asset");
        deploy::store_dashboard_bundle(data_dir, &content_hash, manifest_toml, &bundle)
            .expect("test: store bundle");
    }

    #[test]
    fn test_route_status() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/status".into(),
            body: String::new(),
        };
        let (method, _params) = route(&req).expect("test: route matched");
        assert_eq!(method, "status");
    }

    #[test]
    fn test_route_blockchain_height() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/blockchain/height".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "blockchain.height");
    }

    #[test]
    fn test_route_blockchain_block_index() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/blockchain/block/42".into(),
            body: String::new(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "blockchain.block");
        assert_eq!(params["index"], 42);
    }

    #[test]
    fn test_route_blockchain_block_invalid_index() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/blockchain/block/abc".into(),
            body: String::new(),
        };
        assert!(route(&req).is_none());
    }

    #[test]
    fn test_route_dns_list() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/dns/list".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "dns.list");
    }

    #[test]
    fn test_route_dns_resolve() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/dns/resolve/trust.hypermesh.online".into(),
            body: String::new(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "dns.resolve");
        assert_eq!(params["name"], "trust.hypermesh.online");
    }

    #[test]
    fn test_route_dns_register_post() {
        let req = HttpRequest {
            method: "POST".into(),
            path: "/api/v1/dns/register".into(),
            body: r#"{"name":"test.local","address":"::1"}"#.into(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "dns.register");
        assert_eq!(params["name"], "test.local");
    }

    #[test]
    fn test_route_network_peers() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/network/peers".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "network.peers");
    }

    #[test]
    fn test_route_topology_info() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/topology/info".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "topology.info");
    }

    #[test]
    fn test_route_asset_list() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/asset/list".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "asset.list");
    }

    #[test]
    fn test_route_dashboard_list() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/dashboard/list".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "dashboard.list");
    }

    #[test]
    fn test_route_config_show() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/config/show".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "config.show");
    }

    #[test]
    fn test_route_config_get_key() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/config/get/network.port".into(),
            body: String::new(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "config.get");
        assert_eq!(params["key"], "network.port");
    }

    #[test]
    fn test_route_domain_list() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/domain/list".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "domain.list");
    }

    #[test]
    fn test_route_domain_register_post() {
        let req = HttpRequest {
            method: "POST".into(),
            path: "/api/v1/domain/register".into(),
            body: r#"{"name":"mynet"}"#.into(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "domain.register");
        assert_eq!(params["name"], "mynet");
    }

    #[test]
    fn test_route_domain_join_post() {
        let req = HttpRequest {
            method: "POST".into(),
            path: "/api/v1/domain/join".into(),
            body: r#"{"token":"abc123"}"#.into(),
        };
        let (method, params) = route(&req).expect("test: route matched");
        assert_eq!(method, "domain.join");
        assert_eq!(params["token"], "abc123");
    }

    #[test]
    fn test_route_unknown_path() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/nonexistent".into(),
            body: String::new(),
        };
        assert!(route(&req).is_none());
    }

    #[test]
    fn test_route_no_api_prefix() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/status".into(),
            body: String::new(),
        };
        assert!(route(&req).is_none());
    }

    #[test]
    fn test_route_wrong_method() {
        let req = HttpRequest {
            method: "POST".into(),
            path: "/api/v1/status".into(),
            body: String::new(),
        };
        assert!(route(&req).is_none());
    }

    #[test]
    fn test_route_with_query_string() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/status?verbose=true".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "status");
    }

    #[test]
    fn test_route_ping() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/ping".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "ping");
    }

    #[test]
    fn test_route_topology_neighbors() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/topology/neighbors".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "topology.neighbors");
    }

    #[test]
    fn test_route_blockchain_validate() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/api/v1/blockchain/validate".into(),
            body: String::new(),
        };
        let (method, _) = route(&req).expect("test: route matched");
        assert_eq!(method, "blockchain.validate");
    }

    #[tokio::test]
    async fn test_http_api_end_to_end() {
        // Start server with a simple handler
        let mut rh = RequestHandler::new();
        rh.register(
            "status",
            Arc::new(|_| Box::pin(async { Ok(serde_json::json!({"ok": true})) })),
        );
        let handler = Arc::new(rh);

        // Bind to random port
        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let data_dir = std::env::temp_dir().join("http_api_test_e2e");
        let dd = data_dir.clone();
        let h = handler.clone();
        let bc = test_blockchain();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc, true)
                .await
                .expect("test: handle");
        });

        // Connect and send HTTP request
        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET /api/v1/status HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        // Read response
        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Access-Control-Allow-Origin: *"));
        assert!(response.contains(r#"{"ok":true}"#));

        server.await.expect("test: server join");
    }

    #[tokio::test]
    async fn test_http_api_cors_preflight() {
        let handler = Arc::new(RequestHandler::new());

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let data_dir = std::env::temp_dir().join("http_api_test_cors");
        let dd = data_dir.clone();
        let h = handler.clone();
        let bc = test_blockchain();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "OPTIONS /api/v1/status HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 204 No Content"));
        assert!(response.contains("Access-Control-Allow-Origin: *"));

        server.await.expect("test: server join");
    }

    #[test]
    fn test_route_root_not_api() {
        // Root path should NOT go through the API router (it's handled earlier)
        let req = HttpRequest {
            method: "GET".into(),
            path: "/".into(),
            body: String::new(),
        };
        assert!(route(&req).is_none());
    }

    #[tokio::test]
    async fn test_http_api_serves_dashboard_from_blockchain() {
        let handler = Arc::new(RequestHandler::new());
        let data_dir = std::env::temp_dir().join("http_api_test_dashboard_bc");
        let _ = std::fs::remove_dir_all(&data_dir);

        let bc = test_blockchain();
        let mut files = BTreeMap::new();
        files.insert(
            "private/index.html".to_string(),
            b"<!DOCTYPE html><html><body>Hello, HyperMesh!</body></html>".to_vec(),
        );
        register_test_dashboard(&bc, &data_dir, &files).await;

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc2 = bc.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc2, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 8192];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("text/html; charset=utf-8"));
        assert!(response.contains("Hello, HyperMesh!"));

        server.await.expect("test: server join");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_http_api_serves_static_assets() {
        let handler = Arc::new(RequestHandler::new());
        let data_dir = std::env::temp_dir().join("http_api_test_static_bc");
        let _ = std::fs::remove_dir_all(&data_dir);

        let bc = test_blockchain();
        let mut files = BTreeMap::new();
        files.insert(
            "private/assets/app.js".to_string(),
            b"console.log('hello');".to_vec(),
        );
        files.insert(
            "private/index.html".to_string(),
            b"<html></html>".to_vec(),
        );
        register_test_dashboard(&bc, &data_dir, &files).await;

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc2 = bc.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc2, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET /assets/app.js HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("application/javascript"));
        assert!(response.contains("console.log"));

        server.await.expect("test: server join");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_http_api_spa_fallback() {
        let handler = Arc::new(RequestHandler::new());
        let data_dir = std::env::temp_dir().join("http_api_test_spa_bc");
        let _ = std::fs::remove_dir_all(&data_dir);

        let bc = test_blockchain();
        let mut files = BTreeMap::new();
        files.insert(
            "private/index.html".to_string(),
            b"<!DOCTYPE html><html><body>SPA</body></html>".to_vec(),
        );
        register_test_dashboard(&bc, &data_dir, &files).await;

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc2 = bc.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc2, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET /some/deep/route HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("text/html"));
        assert!(response.contains("SPA"));

        server.await.expect("test: server join");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_http_api_public_scope() {
        let handler = Arc::new(RequestHandler::new());
        let data_dir = std::env::temp_dir().join("http_api_test_public_bc");
        let _ = std::fs::remove_dir_all(&data_dir);

        let bc = test_blockchain();
        let mut files = BTreeMap::new();
        files.insert(
            "public/index.html".to_string(),
            b"<!DOCTYPE html><html><body>Public Dashboard</body></html>".to_vec(),
        );
        register_test_dashboard(&bc, &data_dir, &files).await;

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc2 = bc.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc2, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET /public/ HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Public Dashboard"));

        server.await.expect("test: server join");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_http_api_no_dashboard_returns_404() {
        let handler = Arc::new(RequestHandler::new());

        // Use a non-existent directory
        let data_dir = std::env::temp_dir().join("http_api_test_no_dash");
        let _ = std::fs::remove_dir_all(&data_dir);

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc = test_blockchain();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("No dashboard installed"));

        server.await.expect("test: server join");
    }

    #[tokio::test]
    async fn test_http_api_not_found() {
        let handler = Arc::new(RequestHandler::new());

        let data_dir = std::env::temp_dir().join("http_api_test_not_found");

        let listener = TcpListener::bind("[::1]:0")
            .await
            .expect("test: bind");
        let port = listener.local_addr().expect("test: addr").port();

        let dd = data_dir.clone();
        let h = handler.clone();
        let bc = test_blockchain();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test: accept");
            handle_connection(stream, &h, &dd, &bc, true)
                .await
                .expect("test: handle");
        });

        let mut client = tokio::net::TcpStream::connect(format!("[::1]:{}", port))
            .await
            .expect("test: connect");

        let request = "GET /api/v1/nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client
            .write_all(request.as_bytes())
            .await
            .expect("test: write");

        let mut buf = vec![0u8; 4096];
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .expect("test: read");
        let response = String::from_utf8_lossy(&buf[..n]);

        assert!(response.starts_with("HTTP/1.1 404 Not Found"));

        server.await.expect("test: server join");
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(
            guess_content_type(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("data.json")),
            "application/json"
        );
        assert_eq!(guess_content_type(Path::new("logo.png")), "image/png");
        assert_eq!(guess_content_type(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(
            guess_content_type(Path::new("icon.svg")),
            "image/svg+xml"
        );
        assert_eq!(
            guess_content_type(Path::new("font.woff2")),
            "font/woff2"
        );
        assert_eq!(
            guess_content_type(Path::new("module.wasm")),
            "application/wasm"
        );
        assert_eq!(
            guess_content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }
}

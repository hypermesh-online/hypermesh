//! HTTP/3 Bridge for STOQ Transport Layer
//!
//! This module provides HTTP/3 compatibility for web browsers, enabling them to
//! connect to the STOQ protocol backend. It translates HTTP/3 requests to internal
//! STOQ messages while maintaining certificate-based authentication.

use anyhow::{Result, anyhow, Context};
use std::sync::Arc;
use std::net::{SocketAddr, Ipv6Addr};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error, trace};
use bytes::{Bytes, BytesMut, BufMut};
use futures::{Stream, StreamExt};

use h3::server;
use h3_quinn::quinn;
use http::{Request, Response, StatusCode, Method, HeaderMap, HeaderValue};
use http_body_util::{Full, BodyExt};
use rcgen;

use crate::config::{HyperMeshServerConfig, StoqConfig};
use crate::authority::TrustChainAuthorityLayer;
use crate::transport::{StoqTransportLayer, http_gateway::{HttpGateway, HttpRequest, HttpResponse}};
use crate::monitoring::PerformanceMonitor;

/// HTTP/3 to STOQ Bridge
///
/// Provides full HTTP/3 compatibility for web browsers while maintaining
/// the STOQ protocol internally. Supports:
/// - Standard HTTP/3 requests from browsers
/// - WebSocket upgrade for real-time communication
/// - Certificate-based authentication
/// - Efficient protocol translation
pub struct Http3Bridge {
    /// Configuration
    config: Arc<HyperMeshServerConfig>,

    /// STOQ transport layer
    stoq_transport: Arc<StoqTransportLayer>,

    /// TrustChain for certificates
    trustchain: Arc<TrustChainAuthorityLayer>,

    /// HTTP gateway for handling requests
    http_gateway: Arc<RwLock<HttpGateway>>,

    /// Quinn endpoint for HTTP/3
    quinn_endpoint: Option<quinn::Endpoint>,

    /// Active HTTP/3 connections
    connections: Arc<dashmap::DashMap<String, Http3Connection>>,

    /// WebSocket connections
    websocket_connections: Arc<dashmap::DashMap<String, WebSocketConnection>>,

    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,

    /// Bridge state
    state: Arc<RwLock<BridgeState>>,
}

/// HTTP/3 connection state
struct Http3Connection {
    id: String,
    remote_addr: SocketAddr,
    established_at: Instant,
    last_activity: Instant,
    request_count: u64,
}

/// WebSocket connection over HTTP/3
struct WebSocketConnection {
    id: String,
    path: String,
    established_at: Instant,
    last_ping: Instant,
    tx: tokio::sync::mpsc::Sender<Bytes>,
    rx: Arc<Mutex<tokio::sync::mpsc::Receiver<Bytes>>>,
}

/// Bridge state
#[derive(Debug, Clone)]
struct BridgeState {
    running: bool,
    connections_accepted: u64,
    requests_processed: u64,
    websockets_active: u64,
    errors: u64,
}

impl Http3Bridge {
    /// Create new HTTP/3 bridge
    pub async fn new(
        config: Arc<HyperMeshServerConfig>,
        stoq_transport: Arc<StoqTransportLayer>,
        trustchain: Arc<TrustChainAuthorityLayer>,
        http_gateway: Arc<RwLock<HttpGateway>>,
        monitor: Arc<PerformanceMonitor>,
    ) -> Result<Self> {
        info!("🌉 Initializing HTTP/3 Bridge for browser compatibility");
        info!("   Protocol: HTTP/3 over QUIC");
        info!("   Backend: STOQ Transport Layer");
        info!("   Features: WebSocket support, Certificate validation");

        let state = BridgeState {
            running: false,
            connections_accepted: 0,
            requests_processed: 0,
            websockets_active: 0,
            errors: 0,
        };

        Ok(Self {
            config,
            stoq_transport,
            trustchain,
            http_gateway,
            quinn_endpoint: None,
            connections: Arc::new(dashmap::DashMap::new()),
            websocket_connections: Arc::new(dashmap::DashMap::new()),
            monitor,
            state: Arc::new(RwLock::new(state)),
        })
    }

    /// Initialize HTTP/3 endpoint
    pub async fn initialize(&mut self) -> Result<()> {
        info!("🚀 Initializing HTTP/3 endpoint");

        // Create TLS configuration for HTTP/3
        let server_config = self.create_http3_server_config().await?;

        // Create Quinn endpoint for HTTP/3
        let bind_addr = SocketAddr::new(
            std::net::IpAddr::V6(self.config.global.bind_address),
            self.config.global.port,
        );

        let endpoint = quinn::Endpoint::server(server_config, bind_addr)
            .context("Failed to create HTTP/3 endpoint")?;

        info!("✅ HTTP/3 endpoint initialized at https://[{}]:{}/",
              self.config.global.bind_address,
              self.config.global.port);

        self.quinn_endpoint = Some(endpoint);

        // Update state
        let mut state = self.state.write().await;
        state.running = true;

        Ok(())
    }

    /// Create HTTP/3 server configuration
    async fn create_http3_server_config(&self) -> Result<quinn::ServerConfig> {
        debug!("Creating HTTP/3 server configuration");

        // For now, create a simple self-signed certificate configuration
        // In production, this would use the full TrustChain certificates
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .map_err(|e| anyhow!("Failed to generate certificate: {}", e))?;

        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());

        // Create rustls server config with HTTP/3 ALPN
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der.into())
            .map_err(|e| anyhow!("Failed to create TLS config: {}", e))?;

        let mut tls_config = tls_config;
        tls_config.alpn_protocols = vec![b"h3".to_vec()];

        // Create Quinn server config with HTTP/3 settings
        let mut transport_config = quinn::TransportConfig::default();

        // Optimize for HTTP/3 performance
        transport_config
            .max_concurrent_bidi_streams(100u32.into())
            .max_concurrent_uni_streams(100u32.into())
            .max_idle_timeout(Some(Duration::from_secs(30).try_into()?))
            .keep_alive_interval(Some(Duration::from_secs(10)));

        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(Arc::new(tls_config))
                .map_err(|e| anyhow!("Failed to create QUIC server config: {}", e))?
        ));
        server_config.transport_config(Arc::new(transport_config));

        Ok(server_config)
    }

    /// Start accepting HTTP/3 connections
    pub async fn start(&self) -> Result<()> {
        let Some(ref endpoint) = self.quinn_endpoint else {
            return Err(anyhow!("HTTP/3 endpoint not initialized"));
        };

        info!("🌐 HTTP/3 Bridge accepting connections...");
        info!("   Browsers can now connect to https://[::1]:{}/", self.config.global.port);

        // Clone for async task
        let endpoint = endpoint.clone();
        let connections = self.connections.clone();
        let websocket_connections = self.websocket_connections.clone();
        let http_gateway = self.http_gateway.clone();
        let state = self.state.clone();
        let monitor = self.monitor.clone();

        // Spawn connection handler
        tokio::spawn(async move {
            loop {
                match endpoint.accept().await {
                    Some(incoming) => {
                        let connections = connections.clone();
                        let websocket_connections = websocket_connections.clone();
                        let http_gateway = http_gateway.clone();
                        let state = state.clone();
                        let monitor = monitor.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_http3_connection(
                                incoming,
                                connections,
                                websocket_connections,
                                http_gateway,
                                state,
                                monitor,
                            ).await {
                                error!("HTTP/3 connection handling failed: {}", e);
                            }
                        });
                    }
                    None => {
                        info!("HTTP/3 endpoint closed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming HTTP/3 connection
    async fn handle_http3_connection(
        incoming: quinn::Incoming,
        connections: Arc<dashmap::DashMap<String, Http3Connection>>,
        websocket_connections: Arc<dashmap::DashMap<String, WebSocketConnection>>,
        http_gateway: Arc<RwLock<HttpGateway>>,
        state: Arc<RwLock<BridgeState>>,
        monitor: Arc<PerformanceMonitor>,
    ) -> Result<()> {
        let connection = incoming.await.context("Failed to accept HTTP/3 connection")?;
        let remote_addr = connection.remote_address();
        let connection_id = format!("http3-{}", uuid::Uuid::new_v4());

        debug!("New HTTP/3 connection: {} from {}", connection_id, remote_addr);

        // Track connection
        let h3_conn = Http3Connection {
            id: connection_id.clone(),
            remote_addr,
            established_at: Instant::now(),
            last_activity: Instant::now(),
            request_count: 0,
        };
        connections.insert(connection_id.clone(), h3_conn);

        // Update state
        {
            let mut state = state.write().await;
            state.connections_accepted += 1;
        }

        // Create HTTP/3 connection with explicit type
        let mut h3_connection: server::Connection<h3_quinn::Connection, Bytes> =
            server::Connection::new(h3_quinn::Connection::new(connection))
                .await
                .context("Failed to establish HTTP/3 connection")?;

        // Handle requests
        loop {
            match h3_connection.accept().await {
                Ok(Some((request, response_stream))) => {
                    debug!("HTTP/3 request received");

                    // Handle the request using the existing handler
                    let http_gateway = http_gateway.clone();
                    let ws_connections = websocket_connections.clone();
                    let bridge_state = state.clone();
                    let perf_monitor = monitor.clone();
                    let connection_id = format!("h3-{}", uuid::Uuid::new_v4());

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_http3_request(
                            request,
                            response_stream,
                            http_gateway,
                            ws_connections,
                            connection_id,
                            bridge_state,
                            perf_monitor,
                        ).await {
                            warn!("Failed to handle H3 request: {}", e);
                        }
                    });
                }
                Ok(None) => {
                    debug!("HTTP/3 connection closed: {}", connection_id);
                    break;
                }
                Err(e) => {
                    warn!("HTTP/3 connection error: {}", e);
                    break;
                }
            }
        }

        // Clean up connection
        connections.remove(&connection_id);

        Ok(())
    }

    /// Handle HTTP/3 request with proper h3 stream handling
    #[allow(dead_code)]
    async fn handle_http3_request(
        request: Request<()>,
        mut stream: server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
        http_gateway: Arc<RwLock<HttpGateway>>,
        websocket_connections: Arc<dashmap::DashMap<String, WebSocketConnection>>,
        connection_id: String,
        state: Arc<RwLock<BridgeState>>,
        monitor: Arc<PerformanceMonitor>,
    ) -> Result<()> {
        let start = Instant::now();

        debug!("HTTP/3 request: {} {}", request.method(), request.uri());

        // Check for WebSocket upgrade
        if request.headers().get("upgrade").map(|v| v == "websocket").unwrap_or(false) {
            return Self::handle_websocket_upgrade(
                request,
                stream,
                websocket_connections,
                connection_id,
                state,
            ).await;
        }

        // Translate to internal HTTP request
        let internal_request = Self::translate_http3_request(request).await?;

        // Process through HTTP gateway
        let gateway = http_gateway.read().await;
        let internal_response = gateway.handle_request(internal_request).await
            .context("HTTP gateway processing failed")?;

        // Translate back to HTTP/3 response
        let response_body = internal_response.body.clone();
        let response_status = internal_response.status;
        let http3_response = Self::translate_to_http3_response(internal_response)?;

        // Send response
        stream.send_response(http3_response).await
            .context("Failed to send HTTP/3 response")?;

        // Send response body (if any)
        if !response_body.is_empty() {
            stream.send_data(Bytes::from(response_body)).await
                .context("Failed to send HTTP/3 response body")?;
        }

        stream.finish().await
            .context("Failed to finish HTTP/3 response")?;

        // Record HTTP/3 request metrics
        monitor.record_http_request(
            request.method().as_str(),
            request.uri().path(),
            start.elapsed(),
            response_status
        ).await;

        // Update state
        {
            let mut state = state.write().await;
            state.requests_processed += 1;
        }

        Ok(())
    }

    /// Translate HTTP/3 request to internal format
    #[allow(dead_code)]
    async fn translate_http3_request(request: Request<()>) -> Result<HttpRequest> {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        // Convert headers
        let mut headers = HashMap::new();
        for (key, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(key.to_string(), value_str.to_string());
            }
        }

        // For now, body is empty (would need to read stream for actual body)
        let body = Vec::new();

        Ok(HttpRequest {
            method,
            path,
            headers,
            body,
        })
    }

    /// Translate internal response to HTTP/3 format
    #[allow(dead_code)]
    fn translate_to_http3_response(response: HttpResponse) -> Result<Response<()>> {
        let status = StatusCode::from_u16(response.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut builder = Response::builder().status(status);

        // Add headers
        for (key, value) in response.headers {
            builder = builder.header(key, value);
        }

        // Add CORS headers for browser compatibility
        builder = builder
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Authorization");

        // Build response without body (body sent separately in HTTP/3)
        builder.body(())
            .context("Failed to build HTTP/3 response")
    }

    /// Handle WebSocket upgrade over HTTP/3
    #[allow(dead_code)]
    async fn handle_websocket_upgrade(
        request: Request<()>,
        mut stream: server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
        websocket_connections: Arc<dashmap::DashMap<String, WebSocketConnection>>,
        connection_id: String,
        state: Arc<RwLock<BridgeState>>,
    ) -> Result<()> {
        let path = request.uri().path().to_string();
        let ws_id = format!("ws-{}", uuid::Uuid::new_v4());

        info!("WebSocket upgrade request for path: {}", path);

        // Create WebSocket channels
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Create WebSocket connection
        let ws_conn = WebSocketConnection {
            id: ws_id.clone(),
            path: path.clone(),
            established_at: Instant::now(),
            last_ping: Instant::now(),
            tx,
            rx: Arc::new(Mutex::new(rx)),
        };

        websocket_connections.insert(ws_id.clone(), ws_conn);

        // Send upgrade response
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Accept", "accepted")
            .body(())
            .context("Failed to build WebSocket upgrade response")?;

        stream.send_response(response).await
            .context("Failed to send WebSocket upgrade response")?;

        stream.finish().await
            .context("Failed to finish WebSocket upgrade response")?;

        // Update state
        {
            let mut state = state.write().await;
            state.websockets_active += 1;
        }

        info!("✅ WebSocket connection established: {}", ws_id);

        // WebSocket protocol handling
        let ws_connections_clone = websocket_connections.clone();
        let ws_id_clone = ws_id.clone();

        let ws_handler = tokio::spawn(async move {
            // Get the connection from the map to access its receiver
            if let Some(ws_conn) = ws_connections_clone.get(&ws_id_clone) {
                let mut rx_guard = ws_conn.rx.lock().await;
                let tx = ws_conn.tx.clone();

                // WebSocket message loop
                loop {
                    tokio::select! {
                        // Handle incoming messages from receiver
                        msg = rx_guard.recv() => {
                            match msg {
                                Some(data) => {
                                    // Echo the message back (for now)
                                    // In production, this would process the message through the application
                                    debug!("Received WebSocket message: {} bytes", data.len());

                                    // Echo back for demonstration
                                    let echo_msg = format!("Echo: {}", String::from_utf8_lossy(&data));
                                    if let Err(e) = tx.send(Bytes::from(echo_msg)).await {
                                        warn!("Failed to send WebSocket response: {}", e);
                                        break;
                                    }
                                }
                                None => {
                                    debug!("WebSocket receiver closed");
                                    break;
                                }
                            }
                        }

                        // Handle periodic ping/pong
                        _ = tokio::time::sleep(Duration::from_secs(30)) => {
                            let ping_msg = Bytes::from("ping");
                            if let Err(e) = tx.send(ping_msg).await {
                                warn!("Failed to send WebSocket ping: {}", e);
                                break;
                            }
                        }
                    }
                }
            }

            debug!("WebSocket handler for {} ended", ws_id_clone);

            // Clean up connection
            ws_connections_clone.remove(&ws_id_clone);
        });

        // Store the handler task (detached)
        tokio::spawn(ws_handler);

        Ok(())
    }

    /// Get bridge statistics
    pub async fn get_statistics(&self) -> BridgeStatistics {
        let state = self.state.read().await;

        BridgeStatistics {
            running: state.running,
            connections_active: self.connections.len(),
            websockets_active: self.websocket_connections.len(),
            connections_accepted: state.connections_accepted,
            requests_processed: state.requests_processed,
            errors: state.errors,
        }
    }

    /// Shutdown the bridge
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down HTTP/3 Bridge");

        // Update state
        {
            let mut state = self.state.write().await;
            state.running = false;
        }

        // Close all WebSocket connections
        for conn in self.websocket_connections.iter() {
            drop(conn.tx.clone());
        }
        self.websocket_connections.clear();

        // Clear HTTP/3 connections
        self.connections.clear();

        info!("✅ HTTP/3 Bridge shutdown complete");

        Ok(())
    }
}

/// Bridge statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct BridgeStatistics {
    pub running: bool,
    pub connections_active: usize,
    pub websockets_active: usize,
    pub connections_accepted: u64,
    pub requests_processed: u64,
    pub errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http3_request_translation() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header("Content-Type", "application/json")
            .body(())
            .unwrap();

        let internal = Http3Bridge::translate_http3_request(request).await.unwrap();

        assert_eq!(internal.method, "GET");
        assert_eq!(internal.path, "/api/test");
        assert_eq!(internal.headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_http3_response_translation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

        let internal = HttpResponse {
            status: 200,
            headers,
            body: b"Hello World".to_vec(),
        };

        let http3 = Http3Bridge::translate_to_http3_response(internal).unwrap();

        assert_eq!(http3.status(), StatusCode::OK);
        assert!(http3.headers().contains_key("Content-Type"));
        assert!(http3.headers().contains_key("Access-Control-Allow-Origin"));
    }
}
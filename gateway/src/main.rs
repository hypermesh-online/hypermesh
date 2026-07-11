// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

mod bootstrap;
mod config;
mod dashboard_server;
mod error;
mod middleware;
mod onboarding;
mod pool;
mod proxy;
mod router;
mod sse_engauge;
mod stoq_bridge;
mod stoq_listener;

use anyhow::Result;
use bytes::{Buf, Bytes};
use h3::{quic, server::Connection};
use h3_quinn::quinn;
use http::{Method, Request, Response};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{GatewayConfig, StoqAuthMode};
use crate::router::GatewayRouter;
use crate::stoq_bridge::{StoqBridge, StoqBridgeConfig};
use crate::stoq_listener::StoqListener;

/// Resolve the directory used to persist this gateway node's FALCON-1024
/// identity in full-STOQ-PoS mode. Uses `HYPERMESH_DATA` if set, else
/// `~/.hypermesh/gateway`. The identity is loaded on restart so the
/// gateway keeps a stable node id.
fn gateway_identity_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("HYPERMESH_DATA") {
        return std::path::PathBuf::from(dir).join("gateway");
    }
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    home.join(".hypermesh").join("gateway")
}

/// Build the FALCON-1024 signer + TrustChain proof provider for
/// full-STOQ-PoS mode. Returns an error if the identity cannot be
/// loaded/created.
fn build_pos_identity() -> Result<(
    Arc<dyn hypermesh_lib::NodeSigner>,
    Arc<dyn hypermesh_lib::StateProofProvider>,
)> {
    let data_dir = gateway_identity_dir();
    std::fs::create_dir_all(&data_dir)?;
    let identity = trustchain::identity::FalconIdentity::load_or_create(&data_dir)?;
    let node_id = identity.node_id.clone();
    let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(identity);
    let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> = Arc::new(
        trustchain::proof_of_state::TrustChainProofProvider::new(node_id, signer.clone()),
    );
    Ok((signer, proof_provider))
}

/// Parse `--config <path>` from CLI arguments.
/// Returns `Some(path)` if found, `None` otherwise.
fn parse_config_path() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" {
            if let Some(path) = args.get(i + 1) {
                return Some(std::path::PathBuf::from(path));
            }
        }
        i += 1;
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gateway=info,h3=info,quinn=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting HyperMesh HTTP/3 Gateway");

    // Load configuration from --config file or fall back to environment variables
    let config = if let Some(config_path) = parse_config_path() {
        info!("Loading configuration from file: {}", config_path.display());
        GatewayConfig::from_file(&config_path)?
    } else {
        info!("Loading configuration from environment variables (defaults)");
        GatewayConfig::from_env()?
    };
    info!("Configuration loaded: listening on {}", config.listen_addr);

    // Initialize router
    let router = Arc::new(GatewayRouter::new(&config).await?);
    info!("Router initialized with backends:");
    info!("  - TrustChain: {}", config.trustchain_addr);
    info!("  - BlockMatrix: {}", config.blockmatrix_addr);
    info!("  - Caesar: {}", config.caesar_addr);
    info!("  - Catalog: {}", config.catalog_addr);

    // Load TLS certificates
    let (cert, key) = load_certificates(&config.cert_path, &config.key_path).await?;

    // Create QUIC server configuration
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)?;

    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?,
    ));

    // Create endpoint
    let endpoint = quinn::Endpoint::server(server_config, config.listen_addr)?;
    info!("HTTP/3 server listening on {}", config.listen_addr);

    // Start STOQ listener if configured (graceful degradation on failure)
    let stoq_bridge: Option<Arc<StoqBridge>> = if let Some(stoq_addr) = config.stoq_listen_addr {
        let bridge_config = StoqBridgeConfig {
            bind_addr: stoq_addr,
            max_connections: config.stoq_max_connections,
            auth_mode: config.stoq_auth_mode,
            ..StoqBridgeConfig::default()
        };

        // F8: choose the constructor based on the configured auth mode.
        // FullStoqPos installs a FALCON identity + proof provider so the
        // listener can enforce a bilateral PoS handshake; HttpProxy skips it.
        let bridge_result = match config.stoq_auth_mode {
            StoqAuthMode::FullStoqPos => {
                info!("STOQ auth mode: full-stoq-pos (bilateral PoS handshake required)");
                match build_pos_identity() {
                    Ok((signer, proof_provider)) => {
                        info!("Gateway FALCON identity: {}", signer.node_id());
                        StoqBridge::new_with_pos(bridge_config, signer, proof_provider).await
                    }
                    Err(e) => Err(e),
                }
            }
            StoqAuthMode::HttpProxy => {
                info!("STOQ auth mode: http-proxy (no PoS handshake — passthrough)");
                StoqBridge::new(bridge_config).await
            }
        };

        match bridge_result {
            Ok(bridge) => {
                let bridge = Arc::new(bridge);
                info!("STOQ bridge listening on {}", stoq_addr);
                Some(bridge)
            }
            Err(e) => {
                error!(
                    "Failed to start STOQ bridge on {}: {} — continuing with HTTP/3 only",
                    stoq_addr, e
                );
                None
            }
        }
    } else {
        info!("STOQ listener disabled by configuration");
        None
    };

    // Spawn STOQ accept loop in the background (if bridge started successfully)
    let stoq_handle = stoq_bridge.as_ref().map(|bridge| {
        let listener = StoqListener::new(Arc::clone(bridge));
        tokio::spawn(async move {
            if let Err(e) = listener
                .run(|info| async move {
                    info!(
                        connection_id = %info.connection_id,
                        remote = %info.remote_addr,
                        "Processing STOQ connection"
                    );
                    Ok(())
                })
                .await
            {
                error!("STOQ listener stopped: {}", e);
            }
        })
    });

    // Accept HTTP/3 connections with graceful shutdown
    info!("Gateway ready — press Ctrl+C to stop");
    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                match incoming {
                    Some(conn) => {
                        let router = router.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(conn, router).await {
                                error!("Failed to handle connection: {}", e);
                            }
                        });
                    }
                    None => {
                        info!("HTTP/3 endpoint closed, shutting down");
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal, draining connections...");
                endpoint.close(quinn::VarInt::from_u32(0), b"shutdown");
                break;
            }
        }
    }

    // Shutdown STOQ bridge gracefully
    if let Some(bridge) = stoq_bridge.as_ref() {
        info!("Shutting down STOQ bridge...");
        bridge.shutdown().await;
    }
    if let Some(handle) = stoq_handle {
        handle.abort();
    }

    info!("Gateway stopped");
    Ok(())
}

/// Handle incoming QUIC connection
async fn handle_connection(incoming: quinn::Incoming, router: Arc<GatewayRouter>) -> Result<()> {
    let connection = incoming.await?;
    let remote_addr = connection.remote_address();

    info!("New connection from {}", remote_addr);

    // Create HTTP/3 connection
    let mut h3_conn: h3::server::Connection<_, Bytes> =
        Connection::new(h3_quinn::Connection::new(connection)).await?;

    // Handle requests via h3 0.0.8 RequestResolver API
    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let router = router.clone();
                tokio::spawn(async move {
                    match resolver.resolve_request().await {
                        Ok((req, stream)) => {
                            if let Err(e) = handle_request(req, stream, router).await {
                                error!("Failed to handle request: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to resolve request: {}", e);
                        }
                    }
                });
            }
            Ok(None) => {
                // Connection closed
                break;
            }
            Err(e) => {
                error!("Error accepting stream: {}", e);
                break;
            }
        }
    }

    info!("Connection closed from {}", remote_addr);
    Ok(())
}

/// Handle individual HTTP/3 request
async fn handle_request<T>(
    req: Request<()>,
    mut stream: h3::server::RequestStream<T, Bytes>,
    router: Arc<GatewayRouter>,
) -> Result<()>
where
    T: quic::BidiStream<Bytes>,
{
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    // Intercept engauge SSE streaming requests before reading any body. SSE
    // is GET-only and must NOT be passed through the buffering proxy path.
    if let Some(handshake) = router.try_engauge_sse(&req) {
        return handle_sse_request(handshake, stream, router, method, path, start).await;
    }

    // Read request body if present
    let body = if has_body(&req) {
        let mut body_data = Vec::new();
        while let Some(mut data) = stream.recv_data().await? {
            body_data.extend_from_slice(&data.copy_to_bytes(data.remaining()));
        }
        Some(Bytes::from(body_data))
    } else {
        None
    };

    // Route request
    let response = match router.route(req, body).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Routing error: {}", e);
            Response::builder()
                .status(502)
                .body(Bytes::from(format!("Gateway error: {e}")))?
        }
    };

    let status = response.status().as_u16();
    let latency = start.elapsed();

    // Send response
    let (parts, body) = response.into_parts();
    let response = Response::from_parts(parts, ());

    stream.send_response(response).await?;
    stream.send_data(body).await?;
    stream.finish().await?;

    info!(
        method = %method,
        path = %path,
        status = status,
        latency_ms = latency.as_millis() as u64,
        "request"
    );

    Ok(())
}

/// Drive an engauge SSE streaming response. Sends response headers immediately,
/// then pumps each `Bytes` chunk received from the bridge into the HTTP/3
/// response body until the producer closes or the client disconnects.
async fn handle_sse_request<T>(
    handshake: crate::sse_engauge::SseHandshake,
    mut stream: h3::server::RequestStream<T, Bytes>,
    router: Arc<GatewayRouter>,
    method: http::Method,
    path: String,
    start: std::time::Instant,
) -> Result<()>
where
    T: quic::BidiStream<Bytes>,
{
    use crate::sse_engauge::SseHandshake;

    match handshake {
        SseHandshake::Error(resp) => {
            let status = resp.status().as_u16();
            let (parts, body) = resp.into_parts();
            let head = Response::from_parts(parts, ());
            stream.send_response(head).await?;
            stream.send_data(body).await?;
            stream.finish().await?;
            info!(
                method = %method,
                path = %path,
                status = status,
                latency_ms = start.elapsed().as_millis() as u64,
                "sse-rejected"
            );
            Ok(())
        }
        SseHandshake::Stream(sse) => {
            let mut parts = sse.parts;
            router.apply_sse_cors(&mut parts);
            let status = parts.status.as_u16();
            let head = Response::from_parts(parts, ());
            stream.send_response(head).await?;

            info!(
                method = %method,
                path = %path,
                status = status,
                "sse-open"
            );

            let mut chunks = sse.chunks;
            let mut bytes_sent: u64 = 0;
            while let Some(chunk) = chunks.recv().await {
                let len = chunk.len() as u64;
                if let Err(e) = stream.send_data(chunk).await {
                    info!(
                        path = %path,
                        bytes = bytes_sent,
                        duration_ms = start.elapsed().as_millis() as u64,
                        reason = %e,
                        "sse-closed"
                    );
                    return Ok(());
                }
                bytes_sent = bytes_sent.saturating_add(len);
            }

            stream.finish().await?;
            info!(
                path = %path,
                bytes = bytes_sent,
                duration_ms = start.elapsed().as_millis() as u64,
                "sse-finish"
            );
            Ok(())
        }
    }
}

/// Check if request has a body
fn has_body(req: &Request<()>) -> bool {
    matches!(req.method(), &Method::POST | &Method::PUT | &Method::PATCH)
        || req.headers().contains_key("content-length")
        || req.headers().contains_key("transfer-encoding")
}

/// Load TLS certificates from files
async fn load_certificates(
    cert_path: &Path,
    key_path: &Path,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    // Load certificate
    let mut cert_file = tokio::fs::File::open(cert_path).await?;
    let mut cert_data = Vec::new();
    cert_file.read_to_end(&mut cert_data).await?;

    let certs = if cert_path
        .extension()
        .is_some_and(|e| e == "pem" || e == "crt")
    {
        rustls_pemfile::certs(&mut cert_data.as_slice()).collect::<Result<Vec<_>, _>>()?
    } else {
        vec![CertificateDer::from(cert_data)]
    };

    // Load private key
    let mut key_file = tokio::fs::File::open(key_path).await?;
    let mut key_data = Vec::new();
    key_file.read_to_end(&mut key_data).await?;

    let key = if key_path
        .extension()
        .is_some_and(|e| e == "pem" || e == "key")
    {
        rustls_pemfile::private_key(&mut key_data.as_slice())?
            .ok_or_else(|| anyhow::anyhow!("No private key found in PEM file"))?
    } else {
        PrivateKeyDer::from(rustls::pki_types::PrivatePkcs8KeyDer::from(key_data))
    };

    info!("TLS certificates loaded successfully");
    Ok((certs, key))
}

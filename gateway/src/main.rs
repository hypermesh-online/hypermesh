// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

mod auth;
mod bootstrap;
mod config;
mod dashboard_server;
mod middleware;
mod onboarding;
mod pool;
mod proxy;
mod router;

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

use crate::config::GatewayConfig;
use crate::router::GatewayRouter;

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

    // Accept connections with graceful shutdown
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
                        info!("Endpoint closed, shutting down");
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

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain CA Service - Certificate Authority for Multi-Node Communication
//!
//! This service provides certificate issuance and validation for BlockMatrix nodes,
//! enabling secure STOQ connections between nodes. Supports both local development
//! and production deployment modes.

use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use trustchain::ca::{CAConfig, CAMode, CertificateRequest, TrustChainCA};
use trustchain::consensus::ConsensusProof;
use trustchain::http3::{Http3StoqServer, Router};

use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};

/// CA service configuration
#[derive(Debug, Clone)]
struct ServiceConfig {
    bind_addr: SocketAddr,
    ca_mode: CAMode,
    ca_id: String,
    allow_self_signed: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8443),
            ca_mode: CAMode::LocalhostTesting,
            ca_id: "trustchain-ca-local".to_string(),
            allow_self_signed: true,
        }
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    ca_id: String,
    mode: String,
    uptime_seconds: u64,
}

/// Certificate issuance request
#[derive(Deserialize)]
struct IssueRequest {
    node_id: String,
    common_name: String,
    ipv6_addresses: Vec<String>,
    san_entries: Vec<String>,
}

/// Certificate issuance response
#[derive(Serialize)]
struct IssueResponse {
    certificate_pem: String,
    chain_pem: String,
    serial_number: String,
    fingerprint: String,
    expires_at: i64,
}

/// Certificate validation request
#[derive(Deserialize)]
struct ValidateRequest {
    certificate_pem: String,
}

/// Certificate validation response
#[derive(Serialize)]
struct ValidateResponse {
    valid: bool,
    common_name: String,
    issuer: String,
    expires_at: i64,
}

/// CA root certificate response
#[derive(Serialize)]
struct RootCertificateResponse {
    certificate_pem: String,
    fingerprint: String,
}

/// Build JSON response helper
fn build_json_response<T: Serialize>(data: T) -> Response<Vec<u8>> {
    match serde_json::to_vec(&data) {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(body)
            .unwrap_or_else(|e| {
                error!("Failed to build response: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(vec![])
                    .unwrap_or_default()
            }),
        Err(e) => {
            error!("Failed to serialize response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(vec![])
                .unwrap_or_default()
        }
    }
}

/// Build error response
fn build_error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    let error_data = serde_json::json!({
        "error": message,
        "status": status.as_u16()
    });

    match serde_json::to_vec(&error_data) {
        Ok(body) => Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(body)
            .unwrap_or_default(),
        Err(_) => Response::builder()
            .status(status)
            .body(vec![])
            .unwrap_or_default(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("trustchain=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("TrustChain CA Service starting...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let mut config = ServiceConfig::default();

    // Parse command line flags
    for i in 1..args.len() {
        match args[i].as_str() {
            "--production" | "-p" => {
                config.ca_mode = CAMode::Production;
                config.ca_id = "trustchain-ca-production".to_string();
                config.allow_self_signed = false;
                // Bind to all IPv6 interfaces in production
                config.bind_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8443);
                info!("Running in PRODUCTION mode");
            }
            "--port" => {
                if i + 1 < args.len() {
                    let port: u16 = args[i + 1].parse().context("Invalid port number")?;
                    config.bind_addr.set_port(port);
                }
            }
            "--bind" => {
                if i + 1 < args.len() {
                    config.bind_addr = args[i + 1].parse().context("Invalid bind address")?;
                }
            }
            "--help" | "-h" => {
                println!("TrustChain CA Service");
                println!();
                println!("Usage: trustchain_ca [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --production, -p     Run in production mode");
                println!("  --port <PORT>        Port to bind to (default: 8443)");
                println!("  --bind <ADDR>        Address to bind to (default: [::1]:8443)");
                println!("  --help, -h           Show this help message");
                println!();
                println!("Examples:");
                println!("  # Local development mode");
                println!("  trustchain_ca");
                println!();
                println!("  # Production mode on all interfaces");
                println!("  trustchain_ca --production");
                println!();
                println!("  # Custom port");
                println!("  trustchain_ca --port 9443");
                return Ok(());
            }
            _ => {}
        }
    }

    // Initialize CA
    let ca_config = if matches!(config.ca_mode, CAMode::Production) {
        CAConfig::production()
    } else {
        CAConfig::default()
    };

    let ca = Arc::new(
        TrustChainCA::new(ca_config)
            .await
            .context("Failed to initialize CA")?,
    );

    info!(
        "CA initialized: {} (mode: {:?})",
        config.ca_id, config.ca_mode
    );

    let start_time = std::time::Instant::now();
    let service_config = Arc::new(config.clone());

    // Create router with CA endpoints
    let router = Router::new()
        // Health check endpoint
        .get("/health", {
            let ca_id = config.ca_id.clone();
            let mode = format!("{:?}", config.ca_mode);
            move |_req| {
                let ca_id = ca_id.clone();
                let mode = mode.clone();
                async move {
                    let uptime = start_time.elapsed().as_secs();
                    let response = HealthResponse {
                        status: "healthy".to_string(),
                        ca_id: ca_id.clone(),
                        mode: mode.clone(),
                        uptime_seconds: uptime,
                    };
                    build_json_response(response)
                }
            }
        })
        // Get root certificate
        .get("/ca/root", {
            let ca = ca.clone();
            move |_req| {
                let ca = ca.clone();
                async move {
                    match ca.get_root_certificate().await {
                        Ok(cert_der) => {
                            // Convert DER to PEM
                            let pem = pem::Pem::new("CERTIFICATE", cert_der.clone());
                            let certificate_pem = pem::encode(&pem);

                            // Calculate fingerprint
                            use sha2::{Digest, Sha256};
                            let mut hasher = Sha256::new();
                            hasher.update(&cert_der);
                            let fingerprint = hex::encode(hasher.finalize());

                            let response = RootCertificateResponse {
                                certificate_pem,
                                fingerprint,
                            };
                            build_json_response(response)
                        }
                        Err(e) => {
                            error!("Failed to get root certificate: {}", e);
                            build_error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to retrieve root certificate",
                            )
                        }
                    }
                }
            }
        })
        // Issue certificate endpoint
        .post("/certificate/issue", {
            let ca = ca.clone();
            let service_config = service_config.clone();
            move |req| {
                let ca = ca.clone();
                let service_config = service_config.clone();
                async move {
                    // Parse request body
                    let body = req.into_body();
                    let issue_req: IssueRequest = match serde_json::from_slice(&body) {
                        Ok(req) => req,
                        Err(e) => {
                            warn!("Invalid certificate request: {}", e);
                            return build_error_response(
                                StatusCode::BAD_REQUEST,
                                "Invalid request format",
                            );
                        }
                    };

                    // Parse IPv6 addresses
                    let mut ipv6_addresses = Vec::new();
                    for addr_str in &issue_req.ipv6_addresses {
                        match addr_str.parse::<Ipv6Addr>() {
                            Ok(addr) => ipv6_addresses.push(addr),
                            Err(e) => {
                                warn!("Invalid IPv6 address '{}': {}", addr_str, e);
                                return build_error_response(
                                    StatusCode::BAD_REQUEST,
                                    &format!("Invalid IPv6 address: {addr_str}"),
                                );
                            }
                        }
                    }

                    // Create consensus proof (for local testing, use default)
                    let consensus_proof = if service_config.allow_self_signed {
                        ConsensusProof::new_for_testing()
                    } else {
                        // In production, would validate proof from request
                        ConsensusProof::new_for_testing()
                    };

                    // Create certificate request
                    let cert_request = CertificateRequest {
                        common_name: issue_req.common_name.clone(),
                        san_entries: issue_req.san_entries,
                        node_id: issue_req.node_id,
                        ipv6_addresses,
                        consensus_proof,
                        timestamp: SystemTime::now(),
                    };

                    // Issue certificate
                    match ca.issue_certificate(cert_request).await {
                        Ok(issued_cert) => {
                            let response = IssueResponse {
                                certificate_pem: issued_cert.certificate_pem,
                                chain_pem: issued_cert.chain_pem,
                                serial_number: issued_cert.serial_number,
                                fingerprint: hex::encode(issued_cert.fingerprint),
                                expires_at: issued_cert
                                    .expires_at
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64,
                            };

                            info!("Certificate issued for: {}", issue_req.common_name);
                            build_json_response(response)
                        }
                        Err(e) => {
                            error!("Failed to issue certificate: {}", e);
                            build_error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to issue certificate",
                            )
                        }
                    }
                }
            }
        })
        // Validate certificate endpoint
        .post("/certificate/validate", {
            let ca = ca.clone();
            move |req| {
                let ca = ca.clone();
                async move {
                    // Parse request body
                    let body = req.into_body();
                    let validate_req: ValidateRequest = match serde_json::from_slice(&body) {
                        Ok(req) => req,
                        Err(e) => {
                            warn!("Invalid validation request: {}", e);
                            return build_error_response(
                                StatusCode::BAD_REQUEST,
                                "Invalid request format",
                            );
                        }
                    };

                    // Parse PEM certificate
                    let pem_parsed = match pem::parse(validate_req.certificate_pem.as_bytes()) {
                        Ok(pem) => pem,
                        Err(e) => {
                            warn!("Invalid PEM certificate: {}", e);
                            return build_error_response(
                                StatusCode::BAD_REQUEST,
                                "Invalid PEM certificate",
                            );
                        }
                    };

                    // Validate certificate chain
                    match ca.validate_certificate_chain(pem_parsed.contents()).await {
                        Ok(is_valid) => {
                            // Parse certificate to extract details
                            let (_, cert) =
                                match x509_parser::parse_x509_certificate(pem_parsed.contents()) {
                                    Ok(parsed) => parsed,
                                    Err(e) => {
                                        error!("Failed to parse certificate: {}", e);
                                        return build_error_response(
                                            StatusCode::BAD_REQUEST,
                                            "Invalid certificate format",
                                        );
                                    }
                                };

                            let common_name = cert
                                .subject()
                                .iter_common_name()
                                .next()
                                .and_then(|cn| cn.as_str().ok())
                                .unwrap_or("Unknown")
                                .to_string();

                            let issuer = cert
                                .issuer()
                                .iter_common_name()
                                .next()
                                .and_then(|cn| cn.as_str().ok())
                                .unwrap_or("Unknown")
                                .to_string();

                            let response = ValidateResponse {
                                valid: is_valid,
                                common_name,
                                issuer,
                                expires_at: cert.validity().not_after.timestamp(),
                            };

                            build_json_response(response)
                        }
                        Err(e) => {
                            error!("Certificate validation error: {}", e);
                            build_error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Certificate validation failed",
                            )
                        }
                    }
                }
            }
        })
        // Simple certificate request endpoint (for node bootstrap)
        .get("/certificate", {
            let ca = ca.clone();
            let service_config = service_config.clone();
            move |req| {
                let ca = ca.clone();
                let service_config = service_config.clone();
                async move {
                    // Extract node info from query parameters or headers
                    let uri = req.uri();
                    let query = uri.query().unwrap_or("");

                    // Parse query parameters
                    let mut node_id = "node-unknown".to_string();
                    let mut common_name = "localhost".to_string();

                    for param in query.split('&') {
                        if let Some((key, value)) = param.split_once('=') {
                            match key {
                                "node_id" => node_id = value.to_string(),
                                "common_name" => common_name = value.to_string(),
                                _ => {}
                            }
                        }
                    }

                    // For local testing, auto-issue certificate
                    if service_config.allow_self_signed {
                        let cert_request = CertificateRequest {
                            common_name: common_name.clone(),
                            san_entries: vec![common_name.clone()],
                            node_id,
                            ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
                            consensus_proof: ConsensusProof::new_for_testing(),
                            timestamp: SystemTime::now(),
                        };

                        match ca.issue_certificate(cert_request).await {
                            Ok(issued_cert) => {
                                info!("Auto-issued certificate for: {}", common_name);

                                // Return simplified response for easy consumption
                                let response = serde_json::json!({
                                    "certificate_pem": issued_cert.certificate_pem,
                                    "chain_pem": issued_cert.chain_pem,
                                    "serial_number": issued_cert.serial_number,
                                });

                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header("content-type", "application/json")
                                    .body(serde_json::to_vec(&response).unwrap_or_default())
                                    .unwrap_or_default()
                            }
                            Err(e) => {
                                error!("Failed to auto-issue certificate: {}", e);
                                build_error_response(
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to issue certificate",
                                )
                            }
                        }
                    } else {
                        build_error_response(
                            StatusCode::FORBIDDEN,
                            "Auto-issuance not allowed in production mode",
                        )
                    }
                }
            }
        });

    // Start HTTP/3 server with STOQ transport
    let server = Http3StoqServer::new(config.bind_addr, router);

    info!(
        "TrustChain CA Service listening on https://{}",
        config.bind_addr
    );
    info!("Mode: {:?}", config.ca_mode);
    if config.allow_self_signed {
        info!("Auto-issuance enabled for development");
    }

    // Run server with graceful shutdown
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("TrustChain CA Service stopped");
    Ok(())
}

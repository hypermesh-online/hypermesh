// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use http::{Request, Response, StatusCode};
use serde::Serialize;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use trustchain::ca::certificate_store::CertificateStore;
use trustchain::ca::federation::{FederationManager, FederationPolicy};
use trustchain::ca::ocsp::OcspResponder;
use trustchain::ca::TrustChainCA;
use trustchain::config::TrustChainConfig;
use trustchain::crypto::falcon::FalconCrypto;
use trustchain::crypto::KeyUsage;
use trustchain::http3::handlers::{
    self, DnsResolveRequest as HandlerDnsResolveRequest, HttpHandlerContext,
    IssueCertificateRequest, OcspHttpRequest, RevokeCertificateRequest,
    ValidateCertificateRequest,
};
use trustchain::http3::{ApiResponse, Http3StoqServer, Router};
use trustchain::security::{SecurityConfig, SecurityMonitor};

/// Build a JSON success response wrapping `data` in `ApiResponse`.
fn build_json_response<T: Serialize>(data: T, request_id: String) -> Response<Vec<u8>> {
    match serde_json::to_vec(&ApiResponse::success(data, request_id)) {
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

/// Build a JSON error response using the `ApiResponse` format.
fn build_error_response(code: &str, message: String, request_id: String) -> Response<Vec<u8>> {
    let api_resp: ApiResponse<()> = ApiResponse::error(code.to_string(), message, request_id);
    match serde_json::to_vec(&api_resp) {
        Ok(body) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("content-type", "application/json")
            .body(body)
            .unwrap_or_else(|e| {
                error!("Failed to build error response: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(vec![])
                    .unwrap_or_default()
            }),
        Err(e) => {
            error!("Failed to serialize error response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(vec![])
                .unwrap_or_default()
        }
    }
}

/// Parse a JSON request body, returning an error response on failure.
#[allow(clippy::result_large_err)]
fn parse_request_body<T: serde::de::DeserializeOwned>(
    req: &Request<Vec<u8>>,
    request_id: &str,
) -> Result<T, Response<Vec<u8>>> {
    serde_json::from_slice(req.body()).map_err(|e| {
        build_error_response(
            "BAD_REQUEST",
            format!("Invalid request body: {e}"),
            request_id.to_string(),
        )
    })
}

/// Extract the last path segment from a URI (for /{id} style routes).
fn extract_path_id(req: &Request<Vec<u8>>) -> &str {
    req.uri().path().split('/').next_back().unwrap_or("unknown")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("TrustChain HTTP/3 Server starting...");

    // Load configuration (env var -> ~/.hypermesh/trustchain.toml -> /etc -> defaults)
    let (tc_config, config_source) = TrustChainConfig::load()?;
    match &config_source {
        Some(path) => info!("Configuration loaded from: {}", path),
        None => info!("Using default configuration (no config file found)"),
    }

    // Initialize real service components
    let ca = Arc::new(TrustChainCA::new(tc_config.ca.clone()).await?);

    let certificate_store = Arc::new(CertificateStore::new().await?);

    let security_config = SecurityConfig::default();
    let security_monitor = Arc::new(SecurityMonitor::new(security_config).await?);

    // Phase F.2 — OCSP responder + federation manager.  Federation
    // starts empty (alpha pattern: opt-in via add_peer); when a peer is
    // attached, the federated_check fallback fires automatically.
    let falcon = FalconCrypto::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize FALCON-1024: {e}"))?;
    let ocsp_keypair = falcon
        .generate_keypair(KeyUsage::CertificateAuthority)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate OCSP signing key: {e}"))?;
    let ocsp_responder = Arc::new(OcspResponder::new(
        Arc::clone(&certificate_store),
        ocsp_keypair.private_key,
        format!("trustchain-ocsp-{}", uuid::Uuid::new_v4()),
        None,
    )?);

    let federation = Arc::new(FederationManager::new(
        "local-ca".to_string(),
        FederationPolicy::default(),
    ));

    let ctx = Arc::new(HttpHandlerContext {
        ca,
        certificate_store,
        security_monitor,
        start_time: std::time::Instant::now(),
        ocsp_responder: Some(ocsp_responder),
        federation: Some(federation),
    });

    // Create router with all endpoints wired to real handler functions
    let router = build_router(ctx);

    // Start server using configured bind address and port
    let bind_addr = tc_config.api.bind_address;
    let bind_port = if tc_config.api.port == 0 { 50053 } else { tc_config.api.port };
    let addr = SocketAddr::new(IpAddr::V6(bind_addr), bind_port);
    let server = Http3StoqServer::new(addr, router);

    info!("TrustChain HTTP/3 server (STOQ transport) listening on https://[{}]:{}", bind_addr, bind_port);

    // Run server with graceful shutdown
    let server_task = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("HTTP/3 server error: {}", e);
        }
    });

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down TrustChain HTTP/3 server...");

    // Abort server task (Http3StoqServer doesn't expose a stop method)
    server_task.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), server_task).await;

    info!("TrustChain HTTP/3 server shutdown complete");

    Ok(())
}

fn build_router(ctx: Arc<HttpHandlerContext>) -> Router {
    // Health endpoint
    let ctx_health = Arc::clone(&ctx);
    let router = Router::new().get("/api/v1/trustchain/health", move |_req| {
        let ctx = Arc::clone(&ctx_health);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            match handlers::handle_health(&ctx).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("HEALTH_ERROR", e.to_string(), rid),
            }
        }
    });

    // Status endpoint (no real handler — keep as stub)
    let router = router.get("/api/v1/trustchain/status", |_req| async move {
        // TODO: Implement handle_status() in handlers.rs — needs node-level
        // blockchain height, peer count, and zone count from BlockMatrix integration.
        let response = serde_json::json!({
            "node_id": uuid::Uuid::new_v4().to_string(),
            "blockchain_height": 0,
            "peers_connected": 0,
            "certificates_issued": 0,
            "dns_zones": 0,
        });
        build_json_response(response, uuid::Uuid::new_v4().to_string())
    });

    // Metrics endpoint
    let ctx_metrics = Arc::clone(&ctx);
    let router = router.get("/api/v1/trustchain/metrics", move |_req| {
        let ctx = Arc::clone(&ctx_metrics);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            match handlers::handle_metrics(&ctx).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("METRICS_ERROR", e.to_string(), rid),
            }
        }
    });

    // List certificates
    let ctx_list = Arc::clone(&ctx);
    let router = router.get("/api/v1/trustchain/certificates", move |_req| {
        let ctx = Arc::clone(&ctx_list);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            match handlers::handle_list_certificates(&ctx).await {
                Ok(certs) => build_json_response(certs, rid),
                Err(e) => build_error_response("LIST_CERTS_ERROR", e.to_string(), rid),
            }
        }
    });

    // Issue certificate
    let ctx_issue = Arc::clone(&ctx);
    let router = router.post("/api/v1/trustchain/certificates/issue", move |req| {
        let ctx = Arc::clone(&ctx_issue);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let body: IssueCertificateRequest = match parse_request_body(&req, &rid) {
                Ok(b) => b,
                Err(resp) => return resp,
            };
            match handlers::handle_issue_certificate(&ctx, body).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("ISSUE_CERT_ERROR", e.to_string(), rid),
            }
        }
    });

    // Validate certificate
    let ctx_validate = Arc::clone(&ctx);
    let router = router.post("/api/v1/trustchain/certificates/validate", move |req| {
        let ctx = Arc::clone(&ctx_validate);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let body: ValidateCertificateRequest = match parse_request_body(&req, &rid) {
                Ok(b) => b,
                Err(resp) => return resp,
            };
            match handlers::handle_validate_certificate(&ctx, body).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("VALIDATE_CERT_ERROR", e.to_string(), rid),
            }
        }
    });

    // Get certificate by ID
    let ctx_get = Arc::clone(&ctx);
    let router = router.get("/api/v1/trustchain/certificates/{id}", move |req| {
        let ctx = Arc::clone(&ctx_get);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let serial = extract_path_id(&req).to_string();
            match handlers::handle_get_certificate(&ctx, &serial).await {
                Ok(Some(cert)) => build_json_response(cert, rid),
                Ok(None) => build_error_response(
                    "NOT_FOUND",
                    format!("Certificate {serial} not found"),
                    rid,
                ),
                Err(e) => build_error_response("GET_CERT_ERROR", e.to_string(), rid),
            }
        }
    });

    // Revoke certificate
    let ctx_revoke = Arc::clone(&ctx);
    let router = router.post("/api/v1/trustchain/certificates/revoke", move |req| {
        let ctx = Arc::clone(&ctx_revoke);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let body: RevokeCertificateRequest = match parse_request_body(&req, &rid) {
                Ok(b) => b,
                Err(resp) => return resp,
            };
            match handlers::handle_revoke_certificate(&ctx, body).await {
                Ok(revoked) => build_json_response(
                    serde_json::json!({
                        "revoked": revoked,
                        "revocation_time": chrono::Utc::now().timestamp(),
                    }),
                    rid,
                ),
                Err(e) => build_error_response("REVOKE_CERT_ERROR", e.to_string(), rid),
            }
        }
    });

    // DNS resolve
    let ctx_dns = Arc::clone(&ctx);
    let router = router.post("/api/v1/trustchain/dns/resolve", move |req| {
        let ctx = Arc::clone(&ctx_dns);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let body: HandlerDnsResolveRequest = match parse_request_body(&req, &rid) {
                Ok(b) => b,
                Err(resp) => return resp,
            };
            match handlers::handle_dns_resolve(&ctx, body).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("DNS_RESOLVE_ERROR", e.to_string(), rid),
            }
        }
    });

    // DNS zones (stub)
    let router = router.get("/api/v1/trustchain/dns/zones", |_req| async move {
        // TODO: Implement handle_dns_zones() in handlers.rs — needs DNS zone
        // enumeration from DnsResolver, which requires STOQ transport integration.
        let zones: Vec<serde_json::Value> = Vec::new();
        build_json_response(zones, uuid::Uuid::new_v4().to_string())
    });

    // DNS register (stub)
    let router = router.post("/api/v1/trustchain/dns/register", |_req| async move {
        // TODO: Implement handle_dns_register() in handlers.rs — DNS registration
        // is an asset operation requiring full Proof of State and blockchain
        // integration via BlockMatrix.
        build_error_response(
            "NOT_IMPLEMENTED",
            "DNS registration requires BlockMatrix asset integration (not yet wired)".to_string(),
            uuid::Uuid::new_v4().to_string(),
        )
    });

    // DNS record lookup (stub)
    let router = router.get(
        "/api/v1/trustchain/dns/record/{domain}",
        |_req| async move {
            // TODO: Implement handle_dns_record_lookup() in handlers.rs — needs
            // per-domain record enumeration from DnsResolver via STOQ transport.
            build_error_response(
                "NOT_IMPLEMENTED",
                "DNS record lookup requires STOQ transport integration (not yet wired)".to_string(),
                uuid::Uuid::new_v4().to_string(),
            )
        },
    );

    // State proof status (stub)
    let router = router.get("/api/v1/trustchain/state_proof/status", |_req| async move {
        // TODO: Implement handle_state_proof_status() in handlers.rs — needs
        // FourProofValidator round/validator state exposure.
        let response = serde_json::json!({
            "state_proof_active": false,
            "current_round": 0,
            "validators": 0,
            "finality_threshold": 0.67,
        });
        build_json_response(response, uuid::Uuid::new_v4().to_string())
    });

    // State proof validate (stub)
    let router = router.post("/api/v1/trustchain/state_proof/validate", |_req| async move {
        // TODO: Implement handle_state_proof_validate() in handlers.rs — needs
        // StateProof deserialization from request body and validation via
        // SecurityMonitor::validate_certificate_operation().
        build_error_response(
            "NOT_IMPLEMENTED",
            "State proof validation endpoint requires proof deserialization (not yet wired)"
                .to_string(),
            uuid::Uuid::new_v4().to_string(),
        )
    });

    // State proofs by asset ID (stub)
    let router = router.get(
        "/api/v1/trustchain/state_proof/proofs/{asset_id}",
        |_req| async move {
            // TODO: Implement handle_state_proofs() in handlers.rs — needs
            // per-asset proof retrieval from blockchain/state proof layer.
            build_error_response(
                "NOT_IMPLEMENTED",
                "State proof retrieval requires blockchain integration (not yet wired)"
                    .to_string(),
                uuid::Uuid::new_v4().to_string(),
            )
        },
    );

    // OCSP endpoint (Phase F.2) — local-first, federation fallback.
    let ctx_ocsp = Arc::clone(&ctx);
    let router = router.post("/api/v1/trustchain/ocsp", move |req| {
        let ctx = Arc::clone(&ctx_ocsp);
        async move {
            let rid = uuid::Uuid::new_v4().to_string();
            let body: OcspHttpRequest = match parse_request_body(&req, &rid) {
                Ok(b) => b,
                Err(resp) => return resp,
            };
            match handlers::handle_ocsp(&ctx, body).await {
                Ok(resp) => build_json_response(resp, rid),
                Err(e) => build_error_response("OCSP_ERROR", e.to_string(), rid),
            }
        }
    });

    // Auth certificate (stub)

    router.post("/api/v1/trustchain/auth/certificate", |_req| async move {
        // TODO: Implement handle_auth_certificate() in handlers.rs — needs
        // certificate-based authentication flow: validate cert against CA,
        // generate session token, assign permissions based on cert attributes.
        build_error_response(
            "NOT_IMPLEMENTED",
            "Certificate authentication requires session management (not yet wired)".to_string(),
            uuid::Uuid::new_v4().to_string(),
        )
    })
}

/// Graceful shutdown signal handler (SIGTERM + Ctrl+C)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }
}

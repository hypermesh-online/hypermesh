// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use h3::server::Connection as H3Connection;
use h3_quinn::quinn;
use http::{Request, Response};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use super::middleware::{add_cors_headers, RequestLogger};
use super::router::Router;
use stoq::{StoqTransport, TransportConfig as StoqTransportConfig};

pub struct Http3StoqServer {
    addr: SocketAddr,
    router: Arc<Router>,
}

impl Http3StoqServer {
    pub fn new(addr: SocketAddr, router: Router) -> Self {
        Self {
            addr,
            router: Arc::new(router),
        }
    }

    pub async fn run(self) -> Result<()> {
        info!("Starting HTTP/3 server (STOQ transport) on {}", self.addr);

        // Install default crypto provider for rustls
        let _ = rustls::crypto::ring::default_provider().install_default();

        // Ensure IPv6 address
        if !self.addr.is_ipv6() {
            return Err(anyhow::anyhow!(
                "HTTP/3 server requires IPv6 address, got: {}",
                self.addr
            ));
        }

        let ipv6_addr = match self.addr.ip() {
            IpAddr::V6(addr) => addr,
            _ => unreachable!(),
        };

        // Create STOQ transport configuration
        let stoq_config = StoqTransportConfig {
            bind_address: ipv6_addr,
            port: self.addr.port(),
            max_connections: None,
            connection_timeout: Duration::from_secs(30),
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(60),
            cert_rotation_interval: Duration::from_secs(86400),
            max_concurrent_streams: 100,
            send_buffer_size: 2 * 1024 * 1024,    // 2MB
            receive_buffer_size: 2 * 1024 * 1024, // 2MB
            connection_pool_size: 10,
            enable_zero_copy: true,
            max_datagram_size: 1400,
            memory_pool_size: 1000,
            congestion_control: stoq::transport::CongestionControl::Cubic,
            enable_memory_pool: true,
            frame_batch_size: 64,
            enable_cpu_affinity: false,
            health_check_interval: 10,
            connection_idle_timeout: 30,
            enable_large_send_offload: true,
            enable_falcon_crypto: false,
            falcon_variant: stoq::transport::falcon::FalconVariant::Falcon1024,
            ebpf_interface: None,
            ewma_alpha: 0.125,
            mtu_probe_interval_secs: 30,
            loss_window_size: 10,
        };

        // Initialize STOQ transport
        let transport = StoqTransport::new(stoq_config)
            .await
            .context("Failed to initialize STOQ transport")?;

        info!("HTTP/3 server (STOQ) listening on https://{}", self.addr);

        // Accept connections using STOQ's underlying Quinn endpoint
        let endpoint = transport.quinn_endpoint();

        while let Some(incoming) = endpoint.accept().await {
            let router = self.router.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(connection) => {
                        if let Err(e) = handle_connection(connection, router).await {
                            error!("Connection error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(quic_conn: quinn::Connection, router: Arc<Router>) -> Result<()> {
    // Create h3 connection using h3_quinn adapter
    let quinn_conn = h3_quinn::Connection::new(quic_conn);
    let mut h3_conn = H3Connection::new(quinn_conn).await?;

    loop {
        // Accept incoming request
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let router = router.clone();

                // Spawn task to handle request
                tokio::spawn(async move {
                    // Resolve the request
                    let (req, mut stream) = match resolver.resolve_request().await {
                        Ok(resolved) => resolved,
                        Err(e) => {
                            error!("Failed to resolve request: {}", e);
                            return;
                        }
                    };

                    let start_time = std::time::Instant::now();
                    let request_id = RequestLogger::generate_request_id();

                    // Read request body
                    let mut body = Vec::new();
                    loop {
                        match stream.recv_data().await {
                            Ok(Some(mut data)) => {
                                // data is impl Buf, so we need to extract bytes from it
                                while data.has_remaining() {
                                    let chunk = data.chunk();
                                    body.extend_from_slice(chunk);
                                    let len = chunk.len();
                                    data.advance(len);
                                }
                            }
                            Ok(None) => break, // End of stream
                            Err(e) => {
                                error!("Error reading body: {}", e);
                                break;
                            }
                        }
                    }

                    // Create full request
                    let (parts, _) = req.into_parts();
                    let request = Request::from_parts(parts, body);

                    RequestLogger::log_request(&request, &request_id);

                    // Route request
                    let mut response = router.handle(request).await;

                    // Add CORS headers
                    add_cors_headers(&mut response);

                    // Add request ID header
                    if let Ok(header_value) = request_id.parse() {
                        response.headers_mut().insert("x-request-id", header_value);
                    }

                    RequestLogger::log_response(&response, &request_id, start_time);

                    // Send response
                    let (parts, body) = response.into_parts();
                    let resp = Response::from_parts(parts, ());

                    if let Err(e) = stream.send_response(resp).await {
                        error!("Failed to send response headers: {}", e);
                        return;
                    }

                    // Send body if not empty
                    if !body.is_empty() {
                        if let Err(e) = stream.send_data(Bytes::from(body)).await {
                            error!("Failed to send response body: {}", e);
                            return;
                        }
                    }

                    // Finish stream
                    if let Err(e) = stream.finish().await {
                        error!("Failed to finish stream: {}", e);
                    }
                });
            }
            Ok(None) => {
                // Connection closed
                break;
            }
            Err(e) => {
                error!("Error accepting request: {}", e);
                break;
            }
        }
    }

    Ok(())
}

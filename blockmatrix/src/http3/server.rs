// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use bytes::{Buf, Bytes};
use h3_quinn::quinn;
use http::{Request, Response};
use quinn::Endpoint;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

use super::router::Router;
use super::middleware::{RequestLogger, add_cors_headers};

pub struct Http3Server {
    addr: SocketAddr,
    router: Arc<Router>,
}

impl Http3Server {
    pub fn new(addr: SocketAddr, router: Router) -> Self {
        Self {
            addr,
            router: Arc::new(router),
        }
    }

    pub async fn run(self) -> Result<()> {
        info!("Starting HTTP/3 server on {}", self.addr);

        // Generate self-signed certificate
        let (cert, key) = generate_self_signed_cert()?;

        // Create Quinn server config
        let server_config = create_server_config(cert, key)?;

        // Create endpoint
        let endpoint = Endpoint::server(server_config, self.addr)?;
        info!("HTTP/3 server listening on https://{}", self.addr);

        // Accept connections
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

async fn handle_connection(
    connecting: quinn::Connection,
    router: Arc<Router>,
) -> Result<()> {
    let connection = connecting;
    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(connection)).await?;

    loop {
        match h3_conn.accept().await {
            Ok(Some(resolver)) => {
                let router = router.clone();
                tokio::spawn(async move {
                    // Resolve the request
                    let (req, mut stream) = match resolver.resolve_request().await {
                        Ok(resolved) => resolved,
                        Err(e) => {
                            error!("Error resolving request: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = handle_request(req, &mut stream, router).await {
                        error!("Request handling error: {}", e);
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

    Ok(())
}

async fn handle_request(
    req: Request<()>,
    stream: &mut h3::server::RequestStream<h3_quinn::BidiStream<bytes::Bytes>, bytes::Bytes>,
    router: Arc<Router>,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let request_id = RequestLogger::generate_request_id();

    // Read request body
    let mut body_bytes = Vec::new();
    loop {
        match stream.recv_data().await {
            Ok(Some(mut data)) => {
                // data is impl Buf, extract bytes from it
                while data.has_remaining() {
                    let chunk = data.chunk();
                    body_bytes.extend_from_slice(chunk);
                    let len = chunk.len();
                    data.advance(len);
                }
            }
            Ok(None) => break, // End of stream
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    // Create request with body
    let (parts, _) = req.into_parts();
    let request = Request::from_parts(parts, body_bytes);

    RequestLogger::log_request(&request, &request_id);

    // Route request
    let mut response = router.handle(request).await;

    // Add CORS headers
    add_cors_headers(&mut response);

    // Add request ID header
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );

    RequestLogger::log_response(&response, &request_id, start_time);

    // Send response
    let (parts, body) = response.into_parts();
    let response = Response::from_parts(parts, ());

    stream.send_response(response).await?;
    stream.send_data(Bytes::from(body)).await?;
    stream.finish().await?;

    Ok(())
}

fn generate_self_signed_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];

    let cert = generate_simple_self_signed(subject_alt_names)?;
    let cert_der = cert.cert.der().to_vec();
    let key_der = cert.key_pair.serialize_der();

    let cert = CertificateDer::from(cert_der);
    let key = PrivateKeyDer::try_from(key_der)
        .map_err(|e| anyhow::anyhow!("Failed to create private key: {}", e))?;

    Ok((vec![cert], key))
}

fn create_server_config(certs: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> Result<quinn::ServerConfig> {
    let mut crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    crypto.alpn_protocols = vec![b"h3".to_vec()];

    Ok(quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?
    )))
}
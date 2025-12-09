use anyhow::Result;
use bytes::Bytes;
use h3::client::SendRequest;
use h3_quinn::quinn;
use http::{Request, StatusCode};
use quinn::{ClientConfig, Endpoint};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::test]
async fn test_blockmatrix_http3_quic_health() -> Result<()> {
    // Initialize logging
    let _ = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .try_init();

    // Create QUIC client configuration
    let mut client_crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    // Set ALPN protocol to "h3" for HTTP/3
    client_crypto.alpn_protocols = vec![b"h3".to_vec()];

    let mut client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)?
    ));

    let transport_config = Arc::get_mut(&mut client_config.transport)
        .expect("Failed to get transport config");

    transport_config.max_concurrent_bidi_streams(100u32.into());
    transport_config.max_concurrent_uni_streams(100u32.into());

    // Create client endpoint
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    // Connect to server
    let server_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446);
    let conn = endpoint
        .connect(server_addr, "localhost")?
        .await?;

    info!("Connected via QUIC to {}", server_addr);

    // Create HTTP/3 client
    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, send_request) = h3::client::new(quinn_conn).await?;

    // Spawn driver task
    tokio::spawn(async move {
        driver.wait_idle().await.unwrap();
    });

    // Test health endpoint
    let response = send_http3_request(
        send_request.clone(),
        "/api/v1/blockmatrix/health",
    ).await?;

    assert_eq!(response.0, StatusCode::OK);
    assert!(response.1.contains("healthy"));
    info!("Health check passed: {}", response.1);

    // Test status endpoint
    let response = send_http3_request(
        send_request.clone(),
        "/api/v1/blockmatrix/status",
    ).await?;

    assert_eq!(response.0, StatusCode::OK);
    assert!(response.1.contains("blockmatrix"));
    assert!(response.1.contains("HTTP/3"));
    info!("Status check passed: {}", response.1);

    // Test matrix endpoint
    let response = send_http3_request(
        send_request.clone(),
        "/api/v1/blockmatrix/matrix",
    ).await?;

    assert_eq!(response.0, StatusCode::OK);
    assert!(response.1.contains("dimensions"));
    info!("Matrix info passed: {}", response.1);

    Ok(())
}

async fn send_http3_request(
    mut send_request: SendRequest<h3_quinn::OpenStreams, Bytes>,
    path: &str,
) -> Result<(StatusCode, String)> {
    // Create request
    let req = Request::builder()
        .uri(format!("https://localhost:8446{}", path))
        .method("GET")
        .body(())?;

    // Send request and get response
    let mut stream = send_request.send_request(req).await?;

    // Wait for response
    let response = stream.recv_response().await?;
    let status = response.status();

    // Read body
    let mut body = Vec::new();
    while let Some(mut chunk) = stream.recv_data().await? {
        body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
    }

    let body_str = String::from_utf8(body)?;

    Ok((status, body_str))
}

// Certificate verification bypass for testing
#[derive(Debug)]
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[tokio::test]
async fn test_http3_performance() -> Result<()> {
    // Similar setup as above but with timing measurements
    let start = std::time::Instant::now();

    // Run health check
    test_blockmatrix_http3_quic_health().await?;

    let elapsed = start.elapsed();

    info!("Total test time: {:?}", elapsed);
    assert!(elapsed.as_millis() < 1000, "Test took too long");

    Ok(())
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use std::net::{Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::timeout;

use super::StoqTransport;
use crate::transport::certificate_strategy::NetworkType;
use crate::transport::config::TransportConfig;

async fn server() -> Result<StoqTransport> {
    let config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        enable_kernel_acceleration: false,
        ..TransportConfig::default()
    };
    StoqTransport::new_for_network(config, NetworkType::Anonymous).await
}

async fn client(server: &StoqTransport, bad_alpn: bool) -> Result<quinn::Endpoint> {
    let mut crypto = server.cert_manager.client_crypto_config().await?;
    if bad_alpn {
        crypto.alpn_protocols = vec![b"stoq-invalid-alpn".to_vec()];
    }
    let quic = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?;
    let config = quinn::ClientConfig::new(Arc::new(quic));
    let mut endpoint = quinn::Endpoint::client(SocketAddr::from((Ipv6Addr::LOCALHOST, 0)))?;
    endpoint.set_default_client_config(config);
    Ok(endpoint)
}

#[tokio::test]
async fn idle_is_pending_and_close_is_terminal_for_both_apis() -> Result<()> {
    let server = server().await?;
    assert!(timeout(Duration::from_millis(100), server.accept_next())
        .await
        .is_err());
    server.shutdown().await;
    let next = timeout(Duration::from_secs(5), server.accept_next()).await??;
    assert!(next.is_none());
    let error = timeout(Duration::from_secs(5), server.accept()).await?;
    assert_eq!(
        error.err().map(|e| e.to_string()).as_deref(),
        Some("No incoming connection")
    );
    Ok(())
}

#[tokio::test]
async fn bad_alpn_is_error_and_later_good_connection_is_accepted() -> Result<()> {
    let server = server().await?;
    let addr = server.local_addr()?;
    let bad = client(&server, true).await?;
    let bad_connect = bad.connect(addr, "localhost")?;
    let (accepted, client_result) = timeout(Duration::from_secs(5), async {
        tokio::join!(server.accept_next(), bad_connect)
    })
    .await?;
    assert!(accepted.is_err(), "bad ALPN must be a per-peer error");
    assert!(client_result.is_err(), "bad ALPN must fail on the client");
    let good = client(&server, false).await?;
    let good_connect = good.connect(addr, "localhost")?;
    let (accepted, client_result) = timeout(Duration::from_secs(5), async {
        tokio::join!(server.accept_next(), good_connect)
    })
    .await?;
    assert!(accepted?.is_some(), "later peer must still be accepted");
    let connection = client_result?;
    connection.close(0u32.into(), b"test complete");
    server.shutdown().await;
    Ok(())
}

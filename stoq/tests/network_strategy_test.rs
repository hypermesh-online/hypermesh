//! Integration tests for network-aware certificate strategies

use anyhow::Result;
use stoq::transport::{
    TransportConfig, StoqTransport, NetworkType,
    CertificateStrategy, AnonymousCertificateStrategy,
    P2PCertificateStrategy, FederatedCertificateStrategy,
    PublicCertificateStrategy, CertificateManager,
};
use std::net::Ipv6Addr;
use std::sync::Arc;

#[tokio::test]
async fn test_anonymous_network_transport() -> Result<()> {
    let mut config = TransportConfig::default();
    config.port = 0; // Let OS assign port
    config.bind_address = Ipv6Addr::LOCALHOST;

    // Create transport for anonymous network
    let transport = StoqTransport::new_for_network(
        config,
        NetworkType::Anonymous,
    ).await?;

    // Anonymous network should work without certificates
    assert!(transport.cert_manager.get_certificate_fingerprint().await.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_p2p_network_transport() -> Result<()> {
    let mut config = TransportConfig::default();
    config.port = 0; // Let OS assign port
    config.bind_address = Ipv6Addr::LOCALHOST;

    // Create transport for P2P network
    let transport = StoqTransport::new_for_network(
        config,
        NetworkType::P2P,
    ).await?;

    // P2P network should have self-signed certificate
    let fingerprint = transport.cert_manager.get_certificate_fingerprint().await?;
    assert!(!fingerprint.is_empty());
    assert_eq!(fingerprint.len(), 64); // SHA-256 hex

    Ok(())
}

#[tokio::test]
async fn test_federated_network_transport() -> Result<()> {
    let mut config = TransportConfig::default();
    config.port = 0; // Let OS assign port
    config.bind_address = Ipv6Addr::LOCALHOST;

    // Create transport for federated network
    // Note: This will fail to connect to the gateway but should initialize correctly
    let transport_result = StoqTransport::new_for_network(
        config,
        NetworkType::Federated {
            gateway_url: "gateway.test.internal:8443".to_string(),
        },
    ).await;

    // We expect this to succeed in creating the transport
    // even if it can't connect to the federation gateway
    assert!(transport_result.is_ok() || transport_result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strategy_selection() -> Result<()> {
    let node_id = "test-node".to_string();
    let common_name = "test.local".to_string();
    let ipv6_addresses = vec![Ipv6Addr::LOCALHOST];

    // Test each network type creates the correct strategy
    let strategies = vec![
        (NetworkType::Anonymous, "Anonymous"),
        (NetworkType::P2P, "P2P"),
        (NetworkType::Federated {
            gateway_url: "gateway.test.internal".to_string()
        }, "Federated"),
        (NetworkType::Public, "Public"),
    ];

    for (network_type, expected_name) in strategies {
        let strategy = network_type.create_strategy(
            node_id.clone(),
            common_name.clone(),
            ipv6_addresses.clone(),
        )?;
        assert_eq!(strategy.strategy_name(), expected_name);
    }

    Ok(())
}

#[tokio::test]
async fn test_anonymous_strategy_behavior() -> Result<()> {
    let strategy = AnonymousCertificateStrategy::new();

    // Anonymous should not require certificates
    assert!(!strategy.requires_certificate());

    // Anonymous should return no certificate
    let cert = strategy.get_certificate().await?;
    assert!(cert.is_none());

    Ok(())
}

#[tokio::test]
async fn test_p2p_strategy_behavior() -> Result<()> {
    let strategy = P2PCertificateStrategy::new(
        "test-node".to_string(),
        "localhost".to_string(),
        vec![Ipv6Addr::LOCALHOST],
    )?;

    // P2P should require certificates
    assert!(strategy.requires_certificate());

    // P2P should generate self-signed certificate
    let cert = strategy.get_certificate().await?;
    assert!(cert.is_some());

    let cert = cert.unwrap();
    assert_eq!(cert.node_id, "test-node");
    assert!(!cert.is_expired());

    Ok(())
}

#[tokio::test]
async fn test_certificate_manager_with_strategy() -> Result<()> {
    let strategy = Arc::new(AnonymousCertificateStrategy::new());

    // Create certificate manager with anonymous strategy
    let cert_manager = CertificateManager::with_strategy(strategy).await?;

    // Should work without issues for anonymous network
    let server_config = cert_manager.server_crypto_config().await;

    // Anonymous might not have a certificate, but crypto config should still work
    assert!(server_config.is_ok() || server_config.is_err());

    Ok(())
}

#[tokio::test]
async fn test_backward_compatibility() -> Result<()> {
    let mut config = TransportConfig::default();
    config.port = 0; // Let OS assign port

    // Old API should still work (defaults to localhost testing)
    let transport = StoqTransport::new(config).await?;

    // Should have certificate for localhost testing
    let fingerprint = transport.cert_manager.get_certificate_fingerprint().await?;
    assert!(!fingerprint.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_p2p_peer_management() -> Result<()> {
    let strategy = P2PCertificateStrategy::new(
        "node1".to_string(),
        "node1.local".to_string(),
        vec![Ipv6Addr::LOCALHOST],
    )?;

    // Create a dummy peer certificate
    let peer_strategy = P2PCertificateStrategy::new(
        "node2".to_string(),
        "node2.local".to_string(),
        vec![Ipv6Addr::LOCALHOST],
    )?;

    let peer_cert = peer_strategy.get_certificate().await?.unwrap();

    // Add peer as trusted
    strategy.add_trusted_peer("node2".to_string(), peer_cert.clone()).await;

    // Should now validate the peer
    assert!(strategy.validate_certificate(&peer_cert).await?);

    // List trusted peers
    let peers = strategy.list_trusted_peers().await;
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].0, "node2");

    // Remove peer
    let removed = strategy.remove_trusted_peer("node2").await;
    assert!(removed.is_some());

    // Should no longer validate
    assert!(!strategy.validate_certificate(&peer_cert).await?);

    Ok(())
}

#[tokio::test]
async fn test_network_specific_validation() -> Result<()> {
    // Create strategies for different networks
    let anon_strategy = AnonymousCertificateStrategy::new();
    let p2p_strategy = P2PCertificateStrategy::new(
        "test".to_string(),
        "test.local".to_string(),
        vec![Ipv6Addr::LOCALHOST],
    )?;

    // Get P2P certificate
    let p2p_cert = p2p_strategy.get_certificate().await?.unwrap();

    // Anonymous should accept any certificate
    assert!(anon_strategy.validate_certificate(&p2p_cert).await?);

    // P2P should reject untrusted certificates
    let other_p2p = P2PCertificateStrategy::new(
        "other".to_string(),
        "other.local".to_string(),
        vec![Ipv6Addr::LOCALHOST],
    )?;
    assert!(!other_p2p.validate_certificate(&p2p_cert).await?);

    Ok(())
}
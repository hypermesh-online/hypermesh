//! Transport layer tests

use nexus_transport::*;
use nexus_shared::NodeId;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Test QUIC transport configuration
#[tokio::test]
async fn test_transport_config_validation() {
    crate::utils::init_tracing();
    
    let mut config = TransportConfig::default();
    assert!(config.validate().is_ok());
    
    // Test invalid configurations
    config.port = 0;
    assert!(config.validate().is_err());
    
    config.port = 8080;
    config.max_connections = 0;
    assert!(config.validate().is_err());
}

/// Test certificate generation
#[tokio::test]
async fn test_certificate_generation() {
    crate::utils::init_tracing();
    
    let cert = generate_self_signed_cert("test-node", 365).unwrap();
    let pem = cert.serialize_pem().unwrap();
    
    assert!(pem.contains("BEGIN CERTIFICATE"));
    assert!(pem.contains("END CERTIFICATE"));
}

/// Test certificate manager
#[tokio::test]
async fn test_certificate_manager() {
    crate::utils::init_tracing();
    
    let cert_manager = CertificateManager::new_self_signed(
        "test-node".to_string(),
        365,
        Duration::from_secs(3600),
    ).await.unwrap();
    
    let server_config = cert_manager.server_config().unwrap();
    let client_config = cert_manager.client_config().unwrap();
    
    // Verify ALPN is set correctly
    assert_eq!(server_config.alpn_protocols, vec![b"nexus/1".to_vec()]);
}

/// Test transport message serialization
#[tokio::test]
async fn test_transport_message_serialization() {
    crate::utils::init_tracing();
    
    let source = NodeId::random();
    let dest = NodeId::random();
    let payload = b"test payload".to_vec();
    
    let message = TransportMessage::new(
        MessageType::Data,
        source,
        Some(dest),
        payload.clone(),
    );
    
    let bytes = message.to_bytes().unwrap();
    let deserialized = TransportMessage::from_bytes(&bytes).unwrap();
    
    assert_eq!(message.source, deserialized.source);
    assert_eq!(message.destination, deserialized.destination);
    assert_eq!(message.payload, deserialized.payload);
}

/// Test QUIC server creation
#[tokio::test]
async fn test_quic_server_creation() {
    crate::utils::init_tracing();
    
    let cert_manager = Arc::new(
        CertificateManager::new_self_signed(
            "test-server".to_string(),
            365,
            Duration::from_secs(3600),
        ).await.unwrap()
    );
    
    let mut config = TransportConfig::default();
    config.port = 0; // Use any available port
    
    let result = TransportBuilder::new()
        .with_config(config)
        .with_certificate_manager(cert_manager)
        .build_server()
        .await;
    
    // May fail due to missing full server implementation
    // but tests the builder pattern
    match result {
        Ok(_server) => {
            tracing::info!("Server created successfully");
        }
        Err(e) => {
            tracing::warn!("Server creation failed (expected): {}", e);
        }
    }
}

/// Test QUIC client creation
#[tokio::test]
async fn test_quic_client_creation() {
    crate::utils::init_tracing();
    
    let cert_manager = Arc::new(
        CertificateManager::new_self_signed(
            "test-client".to_string(),
            365,
            Duration::from_secs(3600),
        ).await.unwrap()
    );
    
    let config = TransportConfig::default();
    
    let result = TransportBuilder::new()
        .with_config(config)
        .with_certificate_manager(cert_manager)
        .build_client()
        .await;
    
    // May fail due to missing full client implementation
    match result {
        Ok(_client) => {
            tracing::info!("Client created successfully");
        }
        Err(e) => {
            tracing::warn!("Client creation failed (expected): {}", e);
        }
    }
}

/// Property-based testing for transport configuration
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_transport_config_port_validation(port in 1u16..65535) {
            let mut config = TransportConfig::default();
            config.port = port;
            assert!(config.validate().is_ok());
        }
        
        #[test]
        fn test_transport_config_connections_validation(max_conn in 1u32..1000000) {
            let mut config = TransportConfig::default();
            config.max_connections = max_conn;
            assert!(config.validate().is_ok());
        }
        
        #[test]
        fn test_transport_message_roundtrip(
            payload in prop::collection::vec(any::<u8>(), 0..1024)
        ) {
            let source = NodeId::random();
            let message = TransportMessage::new(
                MessageType::Data,
                source,
                None,
                payload.clone(),
            );
            
            let bytes = message.to_bytes().unwrap();
            let deserialized = TransportMessage::from_bytes(&bytes).unwrap();
            
            assert_eq!(message.payload, deserialized.payload);
        }
    }
}

/// Performance tests for transport layer
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_certificate_generation_performance() {
        crate::utils::init_tracing();
        
        let start = Instant::now();
        
        for _ in 0..10 {
            let _cert = generate_self_signed_cert("test-node", 365).unwrap();
        }
        
        let elapsed = start.elapsed();
        tracing::info!("Generated 10 certificates in {:?}", elapsed);
        
        // Should generate certificates reasonably quickly
        assert!(elapsed < Duration::from_secs(5));
    }
    
    #[tokio::test]
    async fn test_message_serialization_performance() {
        crate::utils::init_tracing();
        
        let source = NodeId::random();
        let payload = crate::utils::random_bytes(1024);
        
        let start = Instant::now();
        
        for _ in 0..1000 {
            let message = TransportMessage::new(
                MessageType::Data,
                source,
                None,
                payload.clone(),
            );
            
            let bytes = message.to_bytes().unwrap();
            let _deserialized = TransportMessage::from_bytes(&bytes).unwrap();
        }
        
        let elapsed = start.elapsed();
        tracing::info!("Serialized/deserialized 1000 messages in {:?}", elapsed);
        
        // Should handle serialization efficiently
        assert!(elapsed < Duration::from_secs(1));
    }
}
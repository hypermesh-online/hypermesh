//! Unit tests for nexus-shared components

use crate::{TestResult, init_test_logging, unit_test};
use nexus_shared::*;
use std::time::Duration;
use tokio::time::timeout;

pub async fn run_shared_tests() -> TestResult {
    init_test_logging();
    
    // Test crypto functionality
    test_crypto_key_generation().await?;
    test_crypto_signing_verification().await?;
    test_authenticated_messages().await?;
    
    // Test configuration
    test_config_serialization().await?;
    test_config_validation().await?;
    
    // Test ID generation
    test_node_id_generation().await?;
    test_resource_id_generation().await?;
    
    // Test error handling
    test_error_categorization().await?;
    
    // Test metrics
    test_metrics_collection().await?;
    
    Ok(())
}

unit_test!(test_crypto_key_generation, "crypto", {
    let keypair1 = KeyPair::generate()?;
    let keypair2 = KeyPair::generate()?;
    
    // Keys should be different
    assert_ne!(keypair1.public_key(), keypair2.public_key());
    
    // Keys should be 32 bytes
    assert_eq!(keypair1.public_key().len(), 32);
    
    Ok(())
});

unit_test!(test_crypto_signing_verification, "crypto", {
    let keypair = KeyPair::generate()?;
    let message = b"Hello, Nexus!";
    
    // Sign and verify
    let signature = keypair.sign(message);
    assert!(KeyPair::verify(keypair.public_key(), message, &signature));
    
    // Verification should fail with wrong message
    let wrong_message = b"Wrong message";
    assert!(!KeyPair::verify(keypair.public_key(), wrong_message, &signature));
    
    // Verification should fail with wrong key
    let wrong_keypair = KeyPair::generate()?;
    assert!(!KeyPair::verify(wrong_keypair.public_key(), message, &signature));
    
    Ok(())
});

unit_test!(test_authenticated_messages, "crypto", {
    let keypair = KeyPair::generate()?;
    let payload = b"sensitive data".to_vec();
    
    // Create authenticated message
    let auth_msg = AuthenticatedMessage::new(payload.clone(), &keypair);
    
    // Verify message
    assert!(auth_msg.verify(keypair.public_key()));
    assert_eq!(auth_msg.payload, payload);
    assert!(auth_msg.is_fresh(300)); // 5 minutes
    
    // Should fail with wrong key
    let wrong_keypair = KeyPair::generate()?;
    assert!(!auth_msg.verify(wrong_keypair.public_key()));
    
    Ok(())
});

unit_test!(test_config_serialization, "config", {
    let config = NexusConfig::default();
    
    // Serialize to TOML
    let toml_str = toml::to_string(&config)?;
    assert!(!toml_str.is_empty());
    
    // Deserialize back
    let parsed: NexusConfig = toml::from_str(&toml_str)?;
    assert_eq!(config.transport.port, parsed.transport.port);
    assert_eq!(config.node.name, parsed.node.name);
    
    Ok(())
});

unit_test!(test_config_validation, "config", {
    let mut config = NexusConfig::default();
    
    // Default config should be valid
    assert!(config.validate().is_ok());
    
    // Invalid port should fail validation
    config.transport.port = 0;
    assert!(config.validate().is_err());
    
    // Fix port and test max connections
    config.transport.port = 7777;
    config.transport.max_connections = 0;
    assert!(config.validate().is_err());
    
    Ok(())
});

unit_test!(test_node_id_generation, "id", {
    let id1 = NodeId::random();
    let id2 = NodeId::random();
    
    // IDs should be different
    assert_ne!(id1, id2);
    
    // IDs should serialize/deserialize correctly
    let serialized = serde_json::to_string(&id1)?;
    let deserialized: NodeId = serde_json::from_str(&serialized)?;
    assert_eq!(id1, deserialized);
    
    Ok(())
});

unit_test!(test_resource_id_generation, "id", {
    let id1 = ResourceId::random();
    let id2 = ResourceId::random();
    
    // IDs should be different
    assert_ne!(id1, id2);
    
    // IDs should serialize/deserialize correctly
    let serialized = serde_json::to_string(&id1)?;
    let deserialized: ResourceId = serde_json::from_str(&serialized)?;
    assert_eq!(id1, deserialized);
    
    Ok(())
});

unit_test!(test_error_categorization, "error", {
    let network_error = NexusError::Network(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "Connection refused"
    ));
    
    let timeout_error = NexusError::Timeout { duration_ms: 5000 };
    let auth_error = NexusError::Authentication { 
        reason: "Invalid token".to_string() 
    };
    
    // Test error categorization
    assert_eq!(network_error.category(), "network");
    assert_eq!(timeout_error.category(), "timeout");
    assert_eq!(auth_error.category(), "auth");
    
    // Test retryable errors
    assert!(network_error.is_retryable());
    assert!(timeout_error.is_retryable());
    assert!(!auth_error.is_retryable());
    
    Ok(())
});

unit_test!(test_metrics_collection, "metrics", {
    let mut collector = MetricsCollector::new("test");
    
    // Test counter
    collector.increment_counter("requests", 1);
    collector.increment_counter("requests", 5);
    
    let metrics = collector.snapshot();
    assert_eq!(metrics.get_counter("requests"), Some(6));
    
    // Test histogram
    collector.record_histogram("response_time", 100.0);
    collector.record_histogram("response_time", 200.0);
    collector.record_histogram("response_time", 150.0);
    
    let metrics = collector.snapshot();
    let histogram = metrics.get_histogram("response_time").unwrap();
    assert_eq!(histogram.count(), 3);
    assert!(histogram.mean() > 0.0);
    
    Ok(())
});

#[tokio::test]
async fn test_timeout_functionality() -> TestResult {
    init_test_logging();
    
    // Test that operations can be timed out
    let result = timeout(
        Duration::from_millis(100),
        async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "completed"
        }
    ).await;
    
    assert!(result.is_err()); // Should timeout
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> TestResult {
    init_test_logging();
    
    // Test concurrent key generation
    let futures = (0..10).map(|_| async {
        KeyPair::generate()
    });
    
    let results = futures::future::join_all(futures).await;
    
    // All should succeed
    assert_eq!(results.len(), 10);
    for result in &results {
        assert!(result.is_ok());
    }
    
    // All keys should be unique
    let mut keys = std::collections::HashSet::new();
    for result in results {
        let keypair = result?;
        let key_bytes = keypair.public_key();
        assert!(keys.insert(key_bytes.to_vec()));
    }
    
    Ok(())
}
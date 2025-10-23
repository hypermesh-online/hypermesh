//! Tests for shared utilities and components

use nexus_shared::*;
use std::time::Duration;

#[tokio::test]
async fn test_node_id_generation() {
    let id1 = NodeId::random();
    let id2 = NodeId::random();
    
    assert_ne!(id1, id2);
    assert_eq!(id1.as_bytes().len(), 32);
    
    let hex_str = id1.to_hex();
    assert_eq!(hex_str.len(), 64);
    
    let parsed = NodeId::from_hex(&hex_str).unwrap();
    assert_eq!(id1, parsed);
}

#[test]
fn test_resource_id() {
    let rid = ResourceId::new("namespace", "name", "deployment");
    
    assert_eq!(rid.namespace(), "namespace");
    assert_eq!(rid.name(), "name");
    assert_eq!(rid.kind(), "deployment");
    assert_eq!(rid.to_string(), "deployment/namespace/name");
}

#[test]
fn test_service_id() {
    let sid = ServiceId::new("web-service", "production");
    
    assert_eq!(sid.name(), "web-service");
    assert_eq!(sid.namespace(), "production");
    assert_eq!(sid.to_string(), "web-service.production");
}

#[test]
fn test_error_categories() {
    let network_error = NexusError::Network(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "Connection refused"
    ));
    
    assert_eq!(network_error.category(), "network");
    assert!(network_error.is_retryable());
    
    let config_error = NexusError::Config("Invalid configuration".to_string());
    assert_eq!(config_error.category(), "config");
    assert!(!config_error.is_retryable());
}

#[test]
fn test_metrics_collector() {
    let collector = metrics::MetricsCollector::new();
    
    collector.increment_counter("requests", 5);
    collector.set_gauge("memory_usage", 1024);
    collector.record_histogram("request_duration", Duration::from_millis(100));
    
    assert_eq!(collector.get_counter("requests"), 5);
    assert_eq!(collector.get_gauge("memory_usage"), 1024);
}

#[test]
fn test_histogram() {
    let hist = metrics::Histogram::new();
    
    hist.record(Duration::from_millis(50));
    hist.record(Duration::from_millis(100));
    hist.record(Duration::from_millis(150));
    
    assert_eq!(hist.count(), 3);
    assert_eq!(hist.average(), 100_000.0); // microseconds
    
    let p50 = hist.percentile(50.0);
    assert!(p50 >= 100_000); // Should be around 100ms
}

#[test]
fn test_config_validation() {
    let config = config::NexusConfig::default();
    assert!(config.validate().is_ok());
    
    let mut invalid_config = config::NexusConfig::default();
    invalid_config.transport.port = 0;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_config_serialization() {
    let config = config::NexusConfig::default();
    let toml_str = toml::to_string(&config).unwrap();
    let parsed: config::NexusConfig = toml::from_str(&toml_str).unwrap();
    
    assert_eq!(config.transport.port, parsed.transport.port);
    assert_eq!(config.node.name, parsed.node.name);
}

#[tokio::test]
async fn test_crypto_keypair() {
    let keypair = crypto::KeyPair::generate().unwrap();
    let public_key = keypair.public_key();
    
    let message = b"Hello, world!";
    let signature = keypair.sign(message);
    
    assert!(crypto::KeyPair::verify(public_key, message, &signature));
    
    // Verify fails with wrong message
    let wrong_message = b"Goodbye, world!";
    assert!(!crypto::KeyPair::verify(public_key, wrong_message, &signature));
}

#[tokio::test]
async fn test_authenticated_message() {
    let keypair = crypto::KeyPair::generate().unwrap();
    let payload = b"secret data".to_vec();
    
    let msg = crypto::AuthenticatedMessage::new(payload.clone(), &keypair);
    
    assert!(msg.verify(keypair.public_key()));
    assert!(msg.is_fresh(60)); // 60 seconds
    assert_eq!(msg.payload, payload);
}

#[test]
fn test_self_signed_certificate() {
    let keypair = crypto::KeyPair::generate().unwrap();
    let cert = crypto::cert::Certificate::self_signed(
        "test-node".to_string(),
        &keypair,
        365
    );
    
    assert!(cert.verify(keypair.public_key()));
    assert!(cert.is_valid());
    assert_eq!(cert.subject, "test-node");
}

#[test]
fn test_timestamp() {
    let ts1 = time::Timestamp::now();
    std::thread::sleep(Duration::from_millis(10));
    let ts2 = time::Timestamp::now();
    
    assert!(ts2 > ts1);
    assert!(ts1.elapsed() >= Duration::from_millis(10));
    
    let future = ts1.add(Duration::from_secs(100));
    assert_eq!(future.as_secs(), ts1.as_secs() + 100);
}

#[test]
fn test_time_window() {
    let window = time::TimeWindow::new(
        Duration::from_secs(60),  // 1 minute skew
        Duration::from_secs(300), // 5 minute age
    );
    
    let now = time::Timestamp::now();
    assert!(window.is_valid(now));
    
    let future = now.add(Duration::from_secs(30));
    assert!(window.is_valid(future));
    
    let far_future = now.add(Duration::from_secs(120));
    assert!(!window.is_valid(far_future));
}

#[tokio::test]
async fn test_rate_limiter() {
    let limiter = time::RateLimiter::new(10, 5); // 10 tokens, 5/sec refill
    
    // Should be able to acquire initial tokens
    assert!(limiter.try_acquire(5));
    assert!(limiter.try_acquire(5));
    
    // Should fail to acquire more
    assert!(!limiter.try_acquire(1));
    
    // Wait for refill and try again
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(limiter.try_acquire(5));
}

#[test]
fn test_hash_function() {
    let data = b"test data";
    let hash1 = crypto::hash(data);
    let hash2 = crypto::hash(data);
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 32);
    
    let different_data = b"different data";
    let hash3 = crypto::hash(different_data);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_random_bytes() {
    let bytes1 = crypto::random_bytes(32);
    let bytes2 = crypto::random_bytes(32);
    
    assert_eq!(bytes1.len(), 32);
    assert_eq!(bytes2.len(), 32);
    assert_ne!(bytes1, bytes2);
}

/// Property-based tests for shared utilities
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_node_id_hex_roundtrip(bytes in prop::array::uniform32(any::<u8>())) {
            let id = NodeId::new(bytes);
            let hex = id.to_hex();
            let parsed = NodeId::from_hex(&hex).unwrap();
            assert_eq!(id, parsed);
        }
        
        #[test]
        fn test_hash_consistency(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let hash1 = crypto::hash(&data);
            let hash2 = crypto::hash(&data);
            assert_eq!(hash1, hash2);
        }
        
        #[test]
        fn test_resource_id_parts(
            namespace in "[a-z0-9-]{1,20}",
            name in "[a-z0-9-]{1,20}",
            kind in "[a-z0-9-]{1,20}"
        ) {
            let rid = ResourceId::new(&namespace, &name, &kind);
            assert_eq!(rid.namespace(), namespace);
            assert_eq!(rid.name(), name);
            assert_eq!(rid.kind(), kind);
        }
    }
}

/// Performance tests for shared utilities
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_node_id_generation_performance() {
        let start = Instant::now();
        
        for _ in 0..1000 {
            let _ = NodeId::random();
        }
        
        let elapsed = start.elapsed();
        println!("Generated 1000 NodeIDs in {:?}", elapsed);
        
        // Should generate IDs quickly
        assert!(elapsed < Duration::from_millis(100));
    }
    
    #[test]
    fn test_hash_performance() {
        let data = vec![0u8; 1024]; // 1KB
        let start = Instant::now();
        
        for _ in 0..1000 {
            let _ = crypto::hash(&data);
        }
        
        let elapsed = start.elapsed();
        println!("Hashed 1MB (1000x1KB) in {:?}", elapsed);
        
        // Should hash data efficiently
        assert!(elapsed < Duration::from_millis(50));
    }
    
    #[test]
    fn test_crypto_signing_performance() {
        let keypair = crypto::KeyPair::generate().unwrap();
        let message = b"test message for signing performance";
        let start = Instant::now();
        
        for _ in 0..100 {
            let signature = keypair.sign(message);
            assert!(crypto::KeyPair::verify(keypair.public_key(), message, &signature));
        }
        
        let elapsed = start.elapsed();
        println!("Signed and verified 100 messages in {:?}", elapsed);
        
        // Should handle signing efficiently
        assert!(elapsed < Duration::from_millis(100));
    }
}
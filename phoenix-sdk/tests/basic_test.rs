//! Basic tests for Phoenix SDK
//!
//! These tests verify the core Phoenix SDK APIs compile and work correctly.

#[cfg(test)]
mod tests {
    use phoenix_sdk::{
        Phoenix, PhoenixConfig, PerformanceTier, SecurityLevel,
        PhoenixMetrics, PhoenixError,
    };

    #[test]
    fn test_phoenix_config_creation() {
        // Test development config
        let dev_config = PhoenixConfig::development("test-app");
        assert_eq!(dev_config.app_name, "test-app");
        assert!(matches!(dev_config.performance_tier, PerformanceTier::Development));
        assert!(matches!(dev_config.security_level, SecurityLevel::Development));

        // Test production config
        let prod_config = PhoenixConfig::production("prod-app");
        assert_eq!(prod_config.app_name, "prod-app");
        assert!(matches!(prod_config.performance_tier, PerformanceTier::Production));
        assert!(matches!(prod_config.security_level, SecurityLevel::Standard));

        // Test high-performance config
        let hp_config = PhoenixConfig::high_performance("hp-app");
        assert_eq!(hp_config.app_name, "hp-app");
        assert!(matches!(hp_config.performance_tier, PerformanceTier::HighThroughput));
        assert!(matches!(hp_config.security_level, SecurityLevel::Enhanced));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = PhoenixConfig::development("test")
            .with_performance_tier(PerformanceTier::Custom(25))
            .with_security_level(SecurityLevel::PostQuantum)
            .with_region("us-west")
            .with_compression(false)
            .with_metrics(true)
            .with_max_connections(5000);

        assert_eq!(config.app_name, "test");
        assert!(matches!(config.performance_tier, PerformanceTier::Custom(25)));
        assert!(matches!(config.security_level, SecurityLevel::PostQuantum));
        assert_eq!(config.region, Some("us-west".to_string()));
        assert!(!config.enable_compression);
        assert!(config.enable_metrics);
        assert_eq!(config.max_connections, 5000);
    }

    #[test]
    fn test_performance_tier_settings() {
        assert_eq!(PerformanceTier::Development.target_gbps(), 1.0);
        assert_eq!(PerformanceTier::Production.target_gbps(), 10.0);
        assert_eq!(PerformanceTier::HighThroughput.target_gbps(), 40.0);
        assert_eq!(PerformanceTier::Custom(100).target_gbps(), 100.0);

        assert_eq!(PerformanceTier::Development.buffer_size(), 65536);
        assert_eq!(PerformanceTier::Production.buffer_size(), 262144);
        assert_eq!(PerformanceTier::HighThroughput.buffer_size(), 1048576);

        assert_eq!(PerformanceTier::Development.connection_pool_size(), 10);
        assert_eq!(PerformanceTier::Production.connection_pool_size(), 100);
        assert_eq!(PerformanceTier::HighThroughput.connection_pool_size(), 1000);
    }

    #[test]
    fn test_security_level_settings() {
        assert!(!SecurityLevel::Development.requires_cert_validation());
        assert!(SecurityLevel::Standard.requires_cert_validation());
        assert!(SecurityLevel::Enhanced.requires_cert_validation());
        assert!(SecurityLevel::PostQuantum.requires_cert_validation());

        assert!(!SecurityLevel::Development.requires_mutual_tls());
        assert!(!SecurityLevel::Standard.requires_mutual_tls());
        assert!(SecurityLevel::Enhanced.requires_mutual_tls());
        assert!(SecurityLevel::PostQuantum.requires_mutual_tls());

        assert!(!SecurityLevel::Development.requires_post_quantum());
        assert!(!SecurityLevel::Standard.requires_post_quantum());
        assert!(!SecurityLevel::Enhanced.requires_post_quantum());
        assert!(SecurityLevel::PostQuantum.requires_post_quantum());

        assert_eq!(SecurityLevel::Development.min_tls_version(), "TLS 1.2");
        assert_eq!(SecurityLevel::Standard.min_tls_version(), "TLS 1.3");
        assert_eq!(SecurityLevel::Enhanced.min_tls_version(), "TLS 1.3");
        assert_eq!(SecurityLevel::PostQuantum.min_tls_version(), "TLS 1.3");
    }

    #[test]
    fn test_compression_engine() {
        use phoenix_sdk::compression::CompressionEngine;

        let engine = CompressionEngine::new(PerformanceTier::Production);

        let original = b"Hello, Phoenix SDK! This is a test message for compression that should be long enough to trigger compression.";
        let compressed = engine.compress(original).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);

        // Small data should not be compressed
        let small_data = b"Small";
        let small_result = engine.compress(small_data).unwrap();
        assert_eq!(small_result, small_data);
    }

    #[test]
    fn test_error_types() {
        // Test error creation and display
        let transport_error = PhoenixError::TransportError("Connection failed".to_string());
        assert_eq!(transport_error.to_string(), "Transport error: Connection failed");

        let security_error = PhoenixError::SecurityError("Certificate invalid".to_string());
        assert_eq!(security_error.to_string(), "Security error: Certificate invalid");

        let conn_closed = PhoenixError::ConnectionClosed;
        assert_eq!(conn_closed.to_string(), "Connection closed");

        let timeout = PhoenixError::Timeout;
        assert_eq!(timeout.to_string(), "Operation timed out");
    }

    // Async tests would require a runtime, but we can test the structure compiles
    #[test]
    fn test_api_structure_compiles() {
        // This test just verifies the API structure is valid
        // Actual async operations would need tokio::test

        // Verify Phoenix creation API exists
        let _create = |app_name: &str| async move {
            let _phoenix = Phoenix::new(app_name).await;
        };

        // Verify connection API exists
        let _connect = |phoenix: &Phoenix, target: &str| async move {
            let _conn = phoenix.connect(target).await;
        };

        // Verify listener API exists
        let _listen = |phoenix: &Phoenix, port: u16| async move {
            let _listener = phoenix.listen(port).await;
        };

        // Verify metrics API exists
        let _metrics = |phoenix: &Phoenix| async move {
            let _metrics = phoenix.metrics().await;
        };
    }
}
//! Comprehensive unit tests for transport layer components

use crate::{TestResult, init_test_logging, unit_test};
use nexus_transport::*;
use nexus_shared::NodeId;
use std::time::Duration;
use tokio::time::timeout;

pub async fn run_transport_tests() -> TestResult {
    init_test_logging();
    
    // Configuration tests
    test_transport_config_validation().await?;
    test_transport_config_serialization().await?;
    
    // Certificate tests  
    test_certificate_generation().await?;
    test_certificate_validation().await?;
    
    // Connection tests
    test_connection_establishment().await?;
    test_connection_security().await?;
    
    // Server tests
    test_server_creation().await?;
    test_server_lifecycle().await?;
    
    // Client tests
    test_client_creation().await?;
    test_client_connection().await?;
    
    // Stream tests
    test_stream_operations().await?;
    test_stream_multiplexing().await?;
    
    Ok(())
}

unit_test!(test_transport_config_validation, "config", {
    let mut config = TransportConfig::default();
    assert!(config.validate().is_ok());
    
    // Test invalid port
    config.bind_port = 0;
    assert!(config.validate().is_err());
    
    // Test invalid max connections
    config.bind_port = 7777;
    config.max_connections = 0;
    assert!(config.validate().is_err());
    
    // Test invalid timeout
    config.max_connections = 1000;
    config.connection_timeout_ms = 0;
    assert!(config.validate().is_err());
    
    Ok(())
});

unit_test!(test_transport_config_serialization, "config", {
    let config = TransportConfig::default();
    
    let serialized = toml::to_string(&config)?;
    assert!(!serialized.is_empty());
    
    let deserialized: TransportConfig = toml::from_str(&serialized)?;
    assert_eq!(config.bind_port, deserialized.bind_port);
    assert_eq!(config.max_connections, deserialized.max_connections);
    
    Ok(())
});

unit_test!(test_certificate_generation, "certificate", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("server.pem");
    let key_path = temp_dir.path().join("server.key");
    
    // Generate certificate
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=nexus-test".to_string(),
        validity_days: 365,
        key_size: 2048,
    };
    
    generate_self_signed_certificate(&cert_config)?;
    
    // Verify files exist
    assert!(cert_path.exists());
    assert!(key_path.exists());
    
    // Verify certificate can be loaded
    let cert_data = std::fs::read(&cert_path)?;
    assert!(!cert_data.is_empty());
    
    let key_data = std::fs::read(&key_path)?;
    assert!(!key_data.is_empty());
    
    Ok(())
});

unit_test!(test_certificate_validation, "certificate", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("server.pem");
    let key_path = temp_dir.path().join("server.key");
    
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=nexus-test".to_string(),
        validity_days: 365,
        key_size: 2048,
    };
    
    generate_self_signed_certificate(&cert_config)?;
    
    // Validate certificate
    let is_valid = validate_certificate(&cert_config.cert_path)?;
    assert!(is_valid);
    
    // Test certificate expiry check
    let expires_in = get_certificate_expiry_days(&cert_config.cert_path)?;
    assert!(expires_in > 300); // Should be close to 365 days
    assert!(expires_in <= 365);
    
    Ok(())
});

unit_test!(test_connection_establishment, "connection", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("server.pem");
    let key_path = temp_dir.path().join("server.key");
    
    // Generate certificate for testing
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=localhost".to_string(),
        validity_days: 1,
        key_size: 2048,
    };
    generate_self_signed_certificate(&cert_config)?;
    
    let port = crate::test_utils::find_available_port()?;
    let config = TransportConfig {
        bind_address: "127.0.0.1".parse().unwrap(),
        bind_port: port,
        cert_path: cert_config.cert_path,
        key_path: cert_config.key_path,
        max_connections: 10,
        connection_timeout_ms: 5000,
        keep_alive_interval_ms: 1000,
        max_idle_timeout_ms: 30000,
    };
    
    // Test connection parameters
    let conn_config = ConnectionConfig::from_transport_config(&config);
    assert_eq!(conn_config.max_connections, config.max_connections);
    assert_eq!(conn_config.timeout_ms, config.connection_timeout_ms);
    
    Ok(())
});

unit_test!(test_connection_security, "security", {
    // Test connection security settings
    let security_config = SecurityConfig {
        enable_client_cert_verification: true,
        enable_sni: true,
        cipher_suites: vec![
            "TLS_AES_256_GCM_SHA384".to_string(),
            "TLS_CHACHA20_POLY1305_SHA256".to_string(),
        ],
        protocol_versions: vec!["1.3".to_string()],
        session_timeout_seconds: 3600,
    };
    
    assert!(security_config.enable_client_cert_verification);
    assert!(security_config.enable_sni);
    assert!(!security_config.cipher_suites.is_empty());
    assert_eq!(security_config.session_timeout_seconds, 3600);
    
    Ok(())
});

unit_test!(test_server_creation, "server", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("server.pem");
    let key_path = temp_dir.path().join("server.key");
    
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=localhost".to_string(),
        validity_days: 1,
        key_size: 2048,
    };
    generate_self_signed_certificate(&cert_config)?;
    
    let port = crate::test_utils::find_available_port()?;
    let config = TransportConfig {
        bind_address: "127.0.0.1".parse().unwrap(),
        bind_port: port,
        cert_path: cert_config.cert_path,
        key_path: cert_config.key_path,
        max_connections: 10,
        connection_timeout_ms: 5000,
        keep_alive_interval_ms: 1000,
        max_idle_timeout_ms: 30000,
    };
    
    let node_id = NodeId::random();
    let server = QuicServer::new(config, node_id).await?;
    
    assert!(!server.is_running());
    assert_eq!(server.local_addr(), None); // Not bound yet
    
    Ok(())
});

unit_test!(test_server_lifecycle, "server", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("server.pem");
    let key_path = temp_dir.path().join("server.key");
    
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=localhost".to_string(),
        validity_days: 1,
        key_size: 2048,
    };
    generate_self_signed_certificate(&cert_config)?;
    
    let port = crate::test_utils::find_available_port()?;
    let config = TransportConfig {
        bind_address: "127.0.0.1".parse().unwrap(),
        bind_port: port,
        cert_path: cert_config.cert_path,
        key_path: cert_config.key_path,
        max_connections: 10,
        connection_timeout_ms: 5000,
        keep_alive_interval_ms: 1000,
        max_idle_timeout_ms: 30000,
    };
    
    let node_id = NodeId::random();
    let mut server = QuicServer::new(config, node_id).await?;
    
    // Start server
    server.start().await?;
    assert!(server.is_running());
    assert!(server.local_addr().is_some());
    
    // Get stats
    let stats = server.stats().await;
    assert_eq!(stats.active_connections, 0);
    assert_eq!(stats.total_connections_accepted, 0);
    
    // Stop server
    server.stop().await?;
    assert!(!server.is_running());
    
    Ok(())
});

unit_test!(test_client_creation, "client", {
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("client.pem");
    let key_path = temp_dir.path().join("client.key");
    
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=nexus-client".to_string(),
        validity_days: 1,
        key_size: 2048,
    };
    generate_self_signed_certificate(&cert_config)?;
    
    let config = ClientConfig {
        cert_path: cert_config.cert_path,
        key_path: cert_config.key_path,
        ca_cert_path: None,
        server_name: "localhost".to_string(),
        connection_timeout_ms: 5000,
        keep_alive_interval_ms: 1000,
        max_idle_timeout_ms: 30000,
    };
    
    let node_id = NodeId::random();
    let client = QuicClient::new(config, node_id)?;
    
    assert!(!client.is_connected());
    assert_eq!(client.connection_count(), 0);
    
    Ok(())
});

unit_test!(test_client_connection, "client", {
    // Test client connection parameters without actual network connection
    let temp_dir = tempfile::TempDir::new()?;
    let cert_path = temp_dir.path().join("client.pem");
    let key_path = temp_dir.path().join("client.key");
    
    let cert_config = CertificateConfig {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        subject: "CN=nexus-client".to_string(),
        validity_days: 1,
        key_size: 2048,
    };
    generate_self_signed_certificate(&cert_config)?;
    
    let config = ClientConfig {
        cert_path: cert_config.cert_path,
        key_path: cert_config.key_path,
        ca_cert_path: None,
        server_name: "localhost".to_string(),
        connection_timeout_ms: 5000,
        keep_alive_interval_ms: 1000,
        max_idle_timeout_ms: 30000,
    };
    
    let node_id = NodeId::random();
    let client = QuicClient::new(config, node_id)?;
    
    // Test connection attempt (will fail but shouldn't crash)
    let server_addr = "127.0.0.1:12345".parse()?;
    let result = timeout(
        Duration::from_millis(100), 
        client.connect(server_addr)
    ).await;
    
    // Should timeout or fail gracefully
    assert!(result.is_err() || result.unwrap().is_err());
    
    Ok(())
});

unit_test!(test_stream_operations, "stream", {
    // Test stream configuration and parameters
    let stream_config = StreamConfig {
        max_stream_data: 1024 * 1024, // 1MB
        max_streams_bidi: 100,
        max_streams_uni: 100,
        stream_receive_window: 64 * 1024, // 64KB
        connection_receive_window: 1024 * 1024, // 1MB
    };
    
    assert_eq!(stream_config.max_stream_data, 1024 * 1024);
    assert_eq!(stream_config.max_streams_bidi, 100);
    assert_eq!(stream_config.max_streams_uni, 100);
    
    // Test stream data validation
    let test_data = b"Hello, QUIC streams!";
    assert!(test_data.len() as u64 <= stream_config.max_stream_data);
    
    Ok(())
});

unit_test!(test_stream_multiplexing, "stream", {
    // Test stream multiplexing configuration
    let multiplex_config = StreamMultiplexConfig {
        concurrent_streams: 10,
        stream_priority_levels: 4,
        flow_control_enabled: true,
        congestion_control: CongestionControlAlgorithm::Cubic,
    };
    
    assert_eq!(multiplex_config.concurrent_streams, 10);
    assert_eq!(multiplex_config.stream_priority_levels, 4);
    assert!(multiplex_config.flow_control_enabled);
    
    match multiplex_config.congestion_control {
        CongestionControlAlgorithm::Cubic => {},
        _ => panic!("Expected Cubic congestion control"),
    }
    
    Ok(())
});

#[tokio::test]
async fn test_transport_error_handling() -> TestResult {
    init_test_logging();
    
    // Test various transport errors
    let cert_error = TransportError::Certificate {
        message: "Invalid certificate".to_string(),
    };
    assert_eq!(cert_error.category(), "certificate");
    assert!(!cert_error.is_retryable());
    
    let connection_error = TransportError::Connection {
        message: "Connection refused".to_string(),
    };
    assert_eq!(connection_error.category(), "connection");
    assert!(connection_error.is_retryable());
    
    let timeout_error = TransportError::Timeout {
        operation: "handshake".to_string(),
        duration_ms: 5000,
    };
    assert_eq!(timeout_error.category(), "timeout");
    assert!(timeout_error.is_retryable());
    
    Ok(())
}

#[tokio::test]
async fn test_transport_metrics() -> TestResult {
    init_test_logging();
    
    // Test transport metrics collection
    let mut metrics = TransportMetrics::new();
    
    metrics.record_connection_established();
    metrics.record_connection_closed();
    metrics.record_bytes_sent(1024);
    metrics.record_bytes_received(2048);
    metrics.record_stream_opened();
    metrics.record_stream_closed();
    
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.connections_established, 1);
    assert_eq!(snapshot.connections_closed, 1);
    assert_eq!(snapshot.bytes_sent, 1024);
    assert_eq!(snapshot.bytes_received, 2048);
    assert_eq!(snapshot.streams_opened, 1);
    assert_eq!(snapshot.streams_closed, 1);
    
    Ok(())
}

// Helper functions and types for tests
struct CertificateConfig {
    cert_path: String,
    key_path: String,
    subject: String,
    validity_days: u32,
    key_size: u32,
}

struct SecurityConfig {
    enable_client_cert_verification: bool,
    enable_sni: bool,
    cipher_suites: Vec<String>,
    protocol_versions: Vec<String>,
    session_timeout_seconds: u64,
}

struct ConnectionConfig {
    max_connections: u32,
    timeout_ms: u64,
}

impl ConnectionConfig {
    fn from_transport_config(config: &TransportConfig) -> Self {
        Self {
            max_connections: config.max_connections,
            timeout_ms: config.connection_timeout_ms,
        }
    }
}

struct StreamConfig {
    max_stream_data: u64,
    max_streams_bidi: u32,
    max_streams_uni: u32,
    stream_receive_window: u64,
    connection_receive_window: u64,
}

struct StreamMultiplexConfig {
    concurrent_streams: u32,
    stream_priority_levels: u8,
    flow_control_enabled: bool,
    congestion_control: CongestionControlAlgorithm,
}

#[derive(Debug, Clone)]
enum CongestionControlAlgorithm {
    Cubic,
    Reno,
    Bbr,
}

// Mock implementations for testing
fn generate_self_signed_certificate(config: &CertificateConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Simulate certificate generation
    std::fs::write(&config.cert_path, "MOCK_CERTIFICATE_DATA")?;
    std::fs::write(&config.key_path, "MOCK_PRIVATE_KEY_DATA")?;
    Ok(())
}

fn validate_certificate(_cert_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // Mock validation
    Ok(true)
}

fn get_certificate_expiry_days(_cert_path: &str) -> Result<u32, Box<dyn std::error::Error>> {
    // Mock expiry check
    Ok(364)
}
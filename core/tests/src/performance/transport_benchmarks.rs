//! Transport layer performance benchmarks
//! Tests QUIC performance, connection pooling, and message throughput

use crate::{TestResult, init_test_logging};
use crate::performance::{PerfTestConfig, PerfTestRunner};
use nexus_transport::{CertificateManager, TransportBuilder, TransportConfig};
use nexus_shared::NodeId;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, error};

/// Test transport layer end-to-end performance
pub async fn run_transport_benchmarks() -> crate::TestResult {
    init_test_logging();
    info!("🚀 Running comprehensive QUIC transport benchmarks");
    
    // Run different benchmark scenarios
    run_connection_establishment_benchmark().await?;
    run_message_throughput_benchmark().await?;
    run_concurrent_connections_benchmark().await?;
    run_large_message_benchmark().await?;
    
    info!("✅ All transport benchmarks completed");
    Ok(())
}

/// Benchmark connection establishment latency
async fn run_connection_establishment_benchmark() -> TestResult {
    info!("📊 Benchmarking connection establishment latency");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    // Setup server
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "benchmark-server".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert.clone())
        .build_server()
        .await?;
        
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Benchmark configuration targeting <10ms connection establishment
    let mut config = PerfTestConfig::default();
    config.duration_seconds = 10;
    config.concurrent_operations = 50;
    config.target_throughput = Some(100); // 100 connections/sec
    config.max_latency_ms = Some(10); // <10ms target
    
    let mut runner = PerfTestRunner::new(config);
    
    let test_results = runner.run_test(|| {
        let server_addr = server_addr.clone();
        let cert_manager = server_cert.clone();
        
        async move {
            let client_config = TransportConfig::default();
            let client = TransportBuilder::new()
                .with_config(client_config)
                .with_certificate_manager(cert_manager)
                .build_client()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            let connection = client.connect(&server_addr.parse()?)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            // Verify connection is working
            let _stream = connection.open_stream()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            Ok(())
        }
    }).await;
    
    test_results.print_summary();
    
    if test_results.meets_targets(&runner.config) {
        info!("✅ Connection establishment benchmark PASSED");
    } else {
        error!("❌ Connection establishment benchmark FAILED to meet targets");
    }
    
    Ok(())
}

/// Benchmark message throughput
async fn run_message_throughput_benchmark() -> TestResult {
    info!("📊 Benchmarking message throughput");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    // Create certificates
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "throughput-server".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    let client_cert = Arc::new(
        CertificateManager::new_self_signed(
            "throughput-client".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Setup server
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert)
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create persistent client connection
    let client_config = TransportConfig::default();
    let client = TransportBuilder::new()
        .with_config(client_config)
        .with_certificate_manager(client_cert)
        .build_client()
        .await?;
        
    let connection = client.connect(&server_addr.parse()?).await?;
    
    // Benchmark configuration targeting high throughput
    let mut config = PerfTestConfig::default();
    config.duration_seconds = 15;
    config.concurrent_operations = 10;
    config.target_throughput = Some(10000); // 10k messages/sec
    config.max_latency_ms = Some(5); // <5ms per message
    
    let mut runner = PerfTestRunner::new(config);
    
    let test_message = b"benchmark_message_payload_1024_bytes".repeat(32); // ~1KB message
    
    let test_results = runner.run_test(|| {
        let connection = connection.clone();
        let message = test_message.clone();
        
        async move {
            let stream = connection.open_stream()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            stream.write_message(&message)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            // Read echo response (in real test, server would echo)
            let _response = stream.read_message()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            Ok(())
        }
    }).await;
    
    test_results.print_summary();
    info!("📈 Achieved throughput: {:.2} messages/sec", test_results.throughput_ops_sec);
    info!("📏 Message size: {} bytes", test_message.len());
    info!("📊 Data rate: {:.2} MB/sec", 
          (test_results.throughput_ops_sec * test_message.len() as f64) / 1_000_000.0);
    
    if test_results.meets_targets(&runner.config) {
        info!("✅ Message throughput benchmark PASSED");
    } else {
        error!("❌ Message throughput benchmark FAILED to meet targets");
    }
    
    Ok(())
}

/// Benchmark concurrent connection handling
async fn run_concurrent_connections_benchmark() -> TestResult {
    info!("📊 Benchmarking concurrent connection handling");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "concurrent-server".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Setup server
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    server_config.max_concurrent_streams = 1000;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert.clone())
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Test with 500 concurrent connections
    let mut config = PerfTestConfig::default();
    config.duration_seconds = 20;
    config.concurrent_operations = 500;
    config.target_throughput = Some(50); // 50 ops/sec with high concurrency
    config.max_latency_ms = Some(100); // Allow higher latency with high concurrency
    
    let mut runner = PerfTestRunner::new(config);
    
    let test_results = runner.run_test(|| {
        let server_addr = server_addr.clone();
        let cert_manager = server_cert.clone();
        
        async move {
            let client_config = TransportConfig::default();
            let client = TransportBuilder::new()
                .with_config(client_config)
                .with_certificate_manager(cert_manager)
                .build_client()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            let connection = client.connect(&server_addr.parse()?)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            // Open multiple streams on this connection
            for _ in 0..5 {
                let stream = connection.open_stream()
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    
                let _ = stream.write_message(b"concurrent_test")
                    .await;
            }
                
            Ok(())
        }
    }).await;
    
    test_results.print_summary();
    info!("🔢 Concurrent connections tested: {}", config.concurrent_operations);
    
    if test_results.meets_targets(&runner.config) {
        info!("✅ Concurrent connections benchmark PASSED");
    } else {
        error!("❌ Concurrent connections benchmark FAILED to meet targets");
    }
    
    Ok(())
}

/// Benchmark large message handling
async fn run_large_message_benchmark() -> TestResult {
    info!("📊 Benchmarking large message handling (10MB messages)");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "large-msg-server".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Setup server with larger buffers
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    server_config.max_message_size = 50 * 1024 * 1024; // 50MB max
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert.clone())
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let client_config = TransportConfig::default();
    let client = TransportBuilder::new()
        .with_config(client_config) 
        .with_certificate_manager(server_cert)
        .build_client()
        .await?;
        
    let connection = client.connect(&server_addr.parse()?).await?;
    
    // Test large message performance
    let mut config = PerfTestConfig::default();
    config.duration_seconds = 30;
    config.concurrent_operations = 5; // Lower concurrency for large messages
    config.target_throughput = Some(1); // 1 large message per second
    config.max_latency_ms = Some(5000); // 5 second max for 10MB message
    
    let mut runner = PerfTestRunner::new(config);
    
    let large_message = vec![0u8; 10 * 1024 * 1024]; // 10MB message
    
    let test_results = runner.run_test(|| {
        let connection = connection.clone();
        let message = large_message.clone();
        
        async move {
            let stream = connection.open_stream()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            stream.write_message(&message)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
            Ok(())
        }
    }).await;
    
    test_results.print_summary();
    info!("📦 Message size: {:.2} MB", large_message.len() as f64 / 1_000_000.0);
    info!("📊 Data throughput: {:.2} MB/sec", 
          (test_results.throughput_ops_sec * large_message.len() as f64) / 1_000_000.0);
    
    if test_results.meets_targets(&runner.config) {
        info!("✅ Large message benchmark PASSED");
    } else {
        error!("❌ Large message benchmark FAILED to meet targets");
    }
    
    Ok(())
}

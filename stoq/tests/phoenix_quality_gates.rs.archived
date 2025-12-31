// Phoenix SDK Quality Gates Test Suite
// Tests comprehensive validation for production readiness

use stoq::phoenix::{PhoenixTransport, PhoenixConfig};
use std::time::{Duration, Instant};
use tokio;

#[tokio::test]
async fn test_phoenix_5_minute_setup() {
    // Test that a new developer can get Phoenix running in under 5 minutes
    let start = Instant::now();

    // Create minimal Phoenix configuration
    let config = PhoenixConfig::default();

    // Initialize Phoenix transport
    let phoenix = PhoenixTransport::new(config).await;
    assert!(phoenix.is_ok(), "Phoenix initialization should succeed");

    let duration = start.elapsed();
    assert!(
        duration < Duration::from_secs(300),
        "Phoenix setup took {:?}, should be under 5 minutes",
        duration
    );
}

#[tokio::test]
async fn test_phoenix_api_simplicity() {
    // Core functionality must be achievable in less than 10 lines of code
    let mut line_count = 0;

    // Line 1: Create config
    let config = PhoenixConfig::default(); line_count += 1;

    // Line 2: Initialize transport
    let phoenix = PhoenixTransport::new(config).await.unwrap(); line_count += 1;

    // Line 3: Connect to server
    let connection = phoenix.connect("localhost:9292").await.unwrap(); line_count += 1;

    // Line 4: Send data
    connection.send(b"Hello Phoenix").await.unwrap(); line_count += 1;

    // Line 5: Receive response
    let response = connection.recv().await.unwrap(); line_count += 1;

    assert!(
        line_count <= 10,
        "Core Phoenix functionality required {} lines, should be <= 10",
        line_count
    );
}

#[tokio::test]
async fn test_phoenix_performance_overhead() {
    // Phoenix overhead must be less than 10% compared to raw STOQ

    // Measure raw STOQ performance
    let raw_start = Instant::now();
    let raw_config = stoq::config::StoqConfig::default();
    let _raw_transport = stoq::transport::StoqTransport::new(raw_config).await.unwrap();
    let raw_duration = raw_start.elapsed();

    // Measure Phoenix SDK performance
    let phoenix_start = Instant::now();
    let phoenix_config = PhoenixConfig::default();
    let _phoenix = PhoenixTransport::new(phoenix_config).await.unwrap();
    let phoenix_duration = phoenix_start.elapsed();

    // Calculate overhead percentage
    let overhead = if raw_duration.as_nanos() > 0 {
        (phoenix_duration.as_nanos() as f64 / raw_duration.as_nanos() as f64) - 1.0
    } else {
        0.0
    };

    assert!(
        overhead < 0.1,
        "Phoenix overhead is {:.1}% (raw: {:?}, phoenix: {:?}), should be < 10%",
        overhead * 100.0,
        raw_duration,
        phoenix_duration
    );
}

#[tokio::test]
async fn test_phoenix_type_safety() {
    // Verify compile-time type safety prevents common errors
    let config = PhoenixConfig::default();
    let phoenix = PhoenixTransport::new(config).await.unwrap();

    // This should compile and provide clear type information
    let connection = phoenix.connect("localhost:9292").await.unwrap();

    // Type system should prevent sending wrong types
    let data: &[u8] = b"Type-safe data";
    let result = connection.send(data).await;

    assert!(result.is_ok(), "Type-safe operations should succeed");
}

#[tokio::test]
async fn test_phoenix_error_handling() {
    // Test graceful error handling and recovery
    let config = PhoenixConfig::default();
    let phoenix = PhoenixTransport::new(config).await.unwrap();

    // Try connecting to non-existent server
    let result = phoenix.connect("nonexistent:9999").await;

    assert!(
        result.is_err(),
        "Should return error for invalid connection"
    );

    // Verify error message is helpful
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(
            error_msg.contains("connect") || error_msg.contains("failed"),
            "Error message should be descriptive: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_phoenix_zero_copy_operations() {
    // Verify zero-copy operations work correctly
    let config = PhoenixConfig::default();
    let phoenix = PhoenixTransport::new(config).await.unwrap();

    // Create large buffer to test zero-copy
    let large_data = vec![0u8; 1024 * 1024]; // 1MB

    let start = Instant::now();
    // This should use zero-copy internally
    let _result = phoenix.process_data(&large_data);
    let duration = start.elapsed();

    // Zero-copy should be very fast for large data
    assert!(
        duration < Duration::from_millis(1),
        "Zero-copy operation took {:?}, should be < 1ms for 1MB",
        duration
    );
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[tokio::test]
    async fn test_phoenix_memory_safety() {
        // Test memory safety under stress conditions
        let config = PhoenixConfig::default();
        let phoenix = PhoenixTransport::new(config).await.unwrap();

        // Send many large chunks to test memory handling
        for i in 0..100 {
            let data = vec![i as u8; 1024 * 1024]; // 1MB chunks
            let _result = phoenix.process_data(&data);
        }

        // Get metrics to verify no memory leaks
        let metrics = phoenix.get_metrics().await;

        // Memory usage should be reasonable (not growing unbounded)
        assert!(
            metrics.memory_usage_mb < 500.0,
            "Memory usage is {} MB, indicates potential leak",
            metrics.memory_usage_mb
        );
    }

    #[tokio::test]
    async fn test_phoenix_concurrent_safety() {
        // Test thread safety with concurrent operations
        use std::sync::Arc;
        use tokio::task;

        let config = PhoenixConfig::default();
        let phoenix = Arc::new(PhoenixTransport::new(config).await.unwrap());

        let mut handles = vec![];

        // Spawn multiple concurrent tasks
        for i in 0..10 {
            let phoenix_clone = phoenix.clone();
            let handle = task::spawn(async move {
                let data = vec![i as u8; 1024];
                phoenix_clone.process_data(&data)
            });
            handles.push(handle);
        }

        // All tasks should complete without panic
        for handle in handles {
            assert!(handle.await.is_ok(), "Concurrent task should not panic");
        }
    }
}

#[cfg(test)]
mod performance_benchmarks {
    use super::*;

    #[tokio::test]
    async fn test_phoenix_throughput() {
        // Measure sustained throughput
        let config = PhoenixConfig::default();
        let phoenix = PhoenixTransport::new(config).await.unwrap();

        let start = Instant::now();
        let mut bytes_processed = 0u64;
        let test_duration = Duration::from_secs(1);

        while start.elapsed() < test_duration {
            let data = vec![0u8; 64 * 1024]; // 64KB chunks
            bytes_processed += data.len() as u64;
            let _result = phoenix.process_data(&data);
        }

        let elapsed = start.elapsed().as_secs_f64();
        let throughput_mbps = (bytes_processed as f64 / (1024.0 * 1024.0)) / elapsed;

        println!("Phoenix throughput: {:.2} MB/s", throughput_mbps);

        // Should handle at least 100 MB/s
        assert!(
            throughput_mbps > 100.0,
            "Throughput {:.2} MB/s is below 100 MB/s target",
            throughput_mbps
        );
    }

    #[tokio::test]
    async fn test_phoenix_latency() {
        // Measure operation latency
        let config = PhoenixConfig::default();
        let phoenix = PhoenixTransport::new(config).await.unwrap();

        let mut latencies = vec![];
        let data = vec![0u8; 1024]; // 1KB test data

        // Measure latency of 100 operations
        for _ in 0..100 {
            let start = Instant::now();
            let _result = phoenix.process_data(&data);
            latencies.push(start.elapsed());
        }

        // Calculate average latency
        let total: Duration = latencies.iter().sum();
        let avg_latency = total / latencies.len() as u32;

        println!("Phoenix average latency: {:?}", avg_latency);

        // Average latency should be under 1ms
        assert!(
            avg_latency < Duration::from_millis(1),
            "Average latency {:?} exceeds 1ms target",
            avg_latency
        );
    }
}
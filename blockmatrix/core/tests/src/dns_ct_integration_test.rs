//! DNS/CT eBPF Integration Test
//! 
//! Comprehensive integration test for the DNS/CT eBPF test suite
//! validating all components work together correctly.

#[cfg(test)]
mod dns_ct_integration_tests {
    use crate::dns_ct;
    use std::time::Duration;

    /// Test that all DNS/CT test modules can be initialized
    #[tokio::test]
    async fn test_dns_ct_modules_initialization() {
        // Test individual module initialization
        
        // DNS Resolution Tests
        let dns_config = dns_ct::DnsTestConfig::default();
        let dns_tests = dns_ct::DnsResolutionTests::new(dns_config).await;
        assert!(dns_tests.is_ok(), "DNS resolution tests should initialize successfully");

        // CT Validation Tests  
        let ct_config = dns_ct::CtTestConfig::default();
        let ct_tests = dns_ct::CtValidationTests::new(ct_config).await;
        assert!(ct_tests.is_ok(), "CT validation tests should initialize successfully");

        // Byzantine Fault Tests
        let byzantine_config = dns_ct::ByzantineFaultTestConfig::default();
        let byzantine_tests = dns_ct::ByzantineFaultTests::new(byzantine_config).await;
        assert!(byzantine_tests.is_ok(), "Byzantine fault tests should initialize successfully");

        // STOQ Integration Tests
        let stoq_config = dns_ct::StoqTestConfig::default();
        let stoq_tests = dns_ct::StoqIntegrationTests::new(stoq_config).await;
        assert!(stoq_tests.is_ok(), "STOQ integration tests should initialize successfully");

        // Performance Benchmarks
        let perf_config = dns_ct::PerformanceBenchmarkConfig::default();
        let perf_tests = dns_ct::PerformanceBenchmarks::new(perf_config).await;
        assert!(perf_tests.is_ok(), "Performance benchmarks should initialize successfully");

        println!("✅ All DNS/CT test modules initialized successfully");
    }

    /// Test comprehensive test suite initialization and basic functionality
    #[tokio::test]
    async fn test_comprehensive_test_suite_initialization() {
        let test_suite = dns_ct::DnsCtTestSuite::new().await;
        assert!(test_suite.is_ok(), "Comprehensive test suite should initialize successfully");
        
        println!("✅ Comprehensive DNS/CT test suite initialized successfully");
    }

    /// Integration test for DNS resolution functionality
    #[tokio::test]
    async fn test_dns_resolution_integration() {
        let config = dns_ct::DnsTestConfig {
            target_resolution_us: 5000, // 5ms for testing
            concurrent_resolutions: 10, // Lower load for testing
            cache_hit_threshold: 0.8, // 80% cache hit rate
            max_memory_mb: 128, // 128MB limit
        };

        let mut dns_tests = dns_ct::DnsResolutionTests::new(config).await
            .expect("DNS tests should initialize");

        // Test basic functionality
        let result = dns_tests.test_basic_dns_resolution().await;
        assert!(result.is_ok(), "Basic DNS resolution should work: {:?}", result);

        println!("✅ DNS resolution integration test passed");
    }

    /// Integration test for CT validation functionality  
    #[tokio::test]
    async fn test_ct_validation_integration() {
        let config = dns_ct::CtTestConfig {
            max_validation_time_ms: 200, // 200ms for testing
            ct_log_servers: 3, // Fewer servers for testing
            byzantine_threshold: 0.66,
            anomaly_sensitivity: 0.7,
        };

        let mut ct_tests = dns_ct::CtValidationTests::new(config).await
            .expect("CT tests should initialize");

        // Test basic functionality
        let result = ct_tests.test_basic_ct_validation().await;
        assert!(result.is_ok(), "Basic CT validation should work: {:?}", result);

        println!("✅ CT validation integration test passed");
    }

    /// Integration test for Byzantine fault tolerance
    #[tokio::test]
    async fn test_byzantine_fault_tolerance_integration() {
        let config = dns_ct::ByzantineFaultTestConfig {
            total_validators: 9, // Smaller network for testing
            max_byzantine_validators: 2, // Less than 1/3
            consensus_threshold: 0.66,
            fault_detection_sensitivity: 0.8,
            recovery_time_target_ms: 2000, // 2 second target
        };

        let mut byzantine_tests = dns_ct::ByzantineFaultTests::new(config).await
            .expect("Byzantine tests should initialize");

        // Test consensus functionality
        let result = byzantine_tests.test_byzantine_dns_consensus().await;
        assert!(result.is_ok(), "Byzantine DNS consensus should work: {:?}", result);

        println!("✅ Byzantine fault tolerance integration test passed");
    }

    /// Integration test for STOQ statistical analysis
    #[tokio::test]
    async fn test_stoq_integration() {
        let config = dns_ct::StoqTestConfig {
            data_retention_period_secs: 3600, // 1 hour for testing
            analysis_window_secs: 60, // 1 minute window
            anomaly_threshold: 2.0,
            ml_accuracy_target: 0.8,
            analytics_latency_target_ms: 200, // 200ms target
        };

        let mut stoq_tests = dns_ct::StoqIntegrationTests::new(config).await
            .expect("STOQ tests should initialize");

        // Test query pattern analysis
        let result = stoq_tests.test_dns_query_pattern_analysis().await;
        assert!(result.is_ok(), "STOQ query pattern analysis should work: {:?}", result);

        println!("✅ STOQ statistical analysis integration test passed");
    }

    /// Integration test for performance benchmarks
    #[tokio::test]
    async fn test_performance_benchmarks_integration() {
        let config = dns_ct::PerformanceBenchmarkConfig {
            target_throughput_pps: 100_000, // 100K PPS for testing
            target_throughput_gbps: 1.0, // 1 Gbps for testing
            target_dns_latency_us: 2000, // 2ms DNS target
            target_ct_latency_us: 10000, // 10ms CT target
            max_memory_usage_mb: 512, // 512MB limit
            max_cpu_utilization_pct: 90.0, // 90% CPU limit
            benchmark_duration_secs: 5, // 5 second test
            concurrent_connections: 100, // 100 concurrent connections
        };

        let mut perf_tests = dns_ct::PerformanceBenchmarks::new(config).await
            .expect("Performance tests should initialize");

        // Test DNS performance
        let result = perf_tests.benchmark_dns_performance().await;
        assert!(result.is_ok(), "DNS performance benchmark should work: {:?}", result);

        if let Ok(metrics) = result {
            assert!(metrics.throughput_pps > 0, "Should achieve non-zero throughput");
            assert!(metrics.avg_dns_latency_us > 0, "Should measure DNS latency");
            println!("  DNS Performance: {} QPS, {}μs avg latency", 
                    metrics.throughput_pps, metrics.avg_dns_latency_us);
        }

        println!("✅ Performance benchmarks integration test passed");
    }

    /// Test quick validation functions
    #[tokio::test]
    async fn test_quick_validation_functions() {
        // Test DNS quick validation
        let result = dns_ct::run_dns_quick_test().await;
        assert!(result.is_ok(), "DNS quick test should pass: {:?}", result);

        // Test CT quick validation
        let result = dns_ct::run_ct_quick_test().await;
        assert!(result.is_ok(), "CT quick test should pass: {:?}", result);

        // Test performance quick validation
        let result = dns_ct::run_performance_quick_test().await;
        assert!(result.is_ok(), "Performance quick test should pass: {:?}", result);

        println!("✅ All quick validation functions passed");
    }

    /// Comprehensive integration test (full suite execution)
    #[tokio::test]
    #[ignore] // Ignore by default as this is a long-running test
    async fn test_dns_ct_complete_suite() {
        println!("🚀 Starting comprehensive DNS/CT eBPF test suite validation");

        let mut test_suite = dns_ct::DnsCtTestSuite::new().await
            .expect("Test suite should initialize successfully");

        let results = test_suite.run_complete_test_suite().await
            .expect("Test suite should execute successfully");

        // Validate results
        assert!(results.overall_success, "Overall test suite should pass");
        assert!(results.dns_results.basic_resolution_passed, "DNS basic resolution should pass");
        assert!(results.ct_results.basic_validation_passed, "CT basic validation should pass");
        assert!(results.byzantine_results.consensus_tests_passed, "Byzantine consensus should pass");
        assert!(results.stoq_results.query_pattern_analysis_passed, "STOQ analysis should pass");

        // Validate performance metrics
        assert!(results.dns_results.avg_resolution_time_us < 10000, 
               "DNS resolution should be under 10ms");
        assert!(results.ct_results.validation_success_rate >= 0.8, 
               "CT validation success rate should be >= 80%");
        assert!(results.byzantine_results.fault_detection_accuracy >= 0.7,
               "Byzantine fault detection should be >= 70% accurate");
        assert!(results.stoq_results.ml_accuracy >= 0.8,
               "STOQ ML accuracy should be >= 80%");

        println!("🎉 DNS/CT eBPF breakthrough technology comprehensively validated!");
        println!("   Duration: {:.2}s", results.total_duration_secs);
        println!("   DNS Resolution: {}μs avg", results.dns_results.avg_resolution_time_us);
        println!("   CT Validation: {:.1}% success rate", results.ct_results.validation_success_rate * 100.0);
        println!("   Byzantine Detection: {:.1}% accuracy", results.byzantine_results.fault_detection_accuracy * 100.0);
        println!("   STOQ ML: {:.1}% accuracy", results.stoq_results.ml_accuracy * 100.0);
        println!("   Performance: {:.1} Gbps throughput", results.performance_results.throughput_gbps);
    }

    /// Test error handling and edge cases
    #[tokio::test]
    async fn test_error_handling_and_edge_cases() {
        println!("Testing error handling and edge cases...");

        // Test initialization with invalid configurations
        let invalid_dns_config = dns_ct::DnsTestConfig {
            target_resolution_us: 0, // Invalid target
            concurrent_resolutions: 0, // Invalid concurrency
            cache_hit_threshold: 2.0, // Invalid threshold (>1.0)
            max_memory_mb: 0, // Invalid memory limit
        };

        // The system should handle invalid configs gracefully
        let dns_tests = dns_ct::DnsResolutionTests::new(invalid_dns_config).await;
        // Should either succeed with corrected values or fail gracefully
        match dns_tests {
            Ok(_) => println!("  Invalid DNS config handled gracefully"),
            Err(e) => println!("  Invalid DNS config rejected: {}", e),
        }

        let invalid_ct_config = dns_ct::CtTestConfig {
            max_validation_time_ms: 0, // Invalid timeout
            ct_log_servers: 0, // No servers
            byzantine_threshold: 1.5, // Invalid threshold
            anomaly_sensitivity: -0.5, // Invalid sensitivity
        };

        let ct_tests = dns_ct::CtValidationTests::new(invalid_ct_config).await;
        match ct_tests {
            Ok(_) => println!("  Invalid CT config handled gracefully"),
            Err(e) => println!("  Invalid CT config rejected: {}", e),
        }

        println!("✅ Error handling and edge cases tested");
    }

    /// Performance regression test
    #[tokio::test]
    async fn test_performance_regression() {
        println!("Running performance regression test...");

        let config = dns_ct::PerformanceBenchmarkConfig {
            target_throughput_pps: 50_000, // Reasonable target
            target_dns_latency_us: 5000, // 5ms target
            benchmark_duration_secs: 3, // Quick test
            concurrent_connections: 50, // Moderate load
            ..Default::default()
        };

        let mut benchmarks = dns_ct::PerformanceBenchmarks::new(config).await
            .expect("Performance benchmarks should initialize");

        let metrics = benchmarks.benchmark_dns_performance().await
            .expect("DNS performance benchmark should succeed");

        // Performance regression checks
        assert!(metrics.throughput_pps >= 1000, 
               "Throughput regression: {} QPS is too low", metrics.throughput_pps);
        
        assert!(metrics.avg_dns_latency_us <= 50000, 
               "Latency regression: {}μs is too high", metrics.avg_dns_latency_us);

        assert!(metrics.error_rate <= 0.1, 
               "Error rate regression: {:.2}% is too high", metrics.error_rate * 100.0);

        println!("  Throughput: {} QPS (✅)", metrics.throughput_pps);
        println!("  Latency: {}μs avg (✅)", metrics.avg_dns_latency_us);
        println!("  Error rate: {:.2}% (✅)", metrics.error_rate * 100.0);

        println!("✅ Performance regression test passed");
    }
}
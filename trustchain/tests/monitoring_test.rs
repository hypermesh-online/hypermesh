//! Integration tests for native monitoring system

use trustchain::monitoring::{
    MonitoringSystem, MonitoringConfig, MetricsExporter,
    export::{JsonExporter, PrometheusExporter},
};

#[tokio::test]
async fn test_native_monitoring_system() {
    // Create monitoring system with default config
    let config = MonitoringConfig::default();
    let monitoring = MonitoringSystem::new(config).await
        .expect("Failed to create monitoring system");

    // Start the monitoring system
    monitoring.start().await
        .expect("Failed to start monitoring");

    // Record some test metrics
    monitoring.record_cert_issuance(35, true).await;
    monitoring.record_cert_issuance(40, true).await;
    monitoring.record_cert_issuance(50, false).await;

    monitoring.record_dns_resolution(10, true).await;
    monitoring.record_dns_resolution(12, true).await;

    monitoring.record_ct_log_entry(5, true).await;

    monitoring.record_consensus_validation(100, true).await;
    monitoring.record_consensus_validation(120, false).await;

    // Get metrics snapshot
    let metrics = monitoring.get_metrics().await;

    // Verify CA metrics
    assert!(metrics.components.contains_key("ca"));
    let ca_metrics = metrics.components.get("ca").unwrap();
    assert_eq!(ca_metrics.total_operations, 3);
    assert_eq!(ca_metrics.successful_operations, 2);
    assert_eq!(ca_metrics.failed_operations, 1);

    // Verify DNS metrics
    assert!(metrics.components.contains_key("dns"));
    let dns_metrics = metrics.components.get("dns").unwrap();
    assert_eq!(dns_metrics.total_operations, 2);
    assert_eq!(dns_metrics.successful_operations, 2);

    // Verify CT metrics
    assert!(metrics.components.contains_key("ct"));
    let ct_metrics = metrics.components.get("ct").unwrap();
    assert_eq!(ct_metrics.total_operations, 1);

    // Verify consensus metrics
    assert!(metrics.components.contains_key("consensus"));
    let consensus_metrics = metrics.components.get("consensus").unwrap();
    assert_eq!(consensus_metrics.total_operations, 2);

    println!("Native monitoring system test passed!");
}

#[tokio::test]
async fn test_health_check_system() {
    let config = MonitoringConfig::default();
    let monitoring = MonitoringSystem::new(config).await
        .expect("Failed to create monitoring system");

    // Get initial health status
    let health = monitoring.get_health().await;

    // All components should be healthy initially
    assert!(health.is_healthy);
    assert_eq!(health.components.len(), 6); // ca, ct, dns, consensus, stoq, api

    for (name, component_health) in &health.components {
        assert!(component_health.is_healthy, "{} should be healthy", name);
    }

    println!("Health check system test passed!");
}

#[tokio::test]
async fn test_metrics_export_formats() {
    let config = MonitoringConfig::default();
    let monitoring = MonitoringSystem::new(config).await
        .expect("Failed to create monitoring system");

    // Record some metrics
    monitoring.record_cert_issuance(35, true).await;
    monitoring.record_dns_resolution(10, true).await;

    let metrics = monitoring.get_metrics().await;

    // Test JSON export
    let json_exporter = JsonExporter;
    let json_output = json_exporter.export(&metrics).await
        .expect("Failed to export JSON metrics");
    assert!(json_output.contains("\"ca\""));
    assert!(json_output.contains("\"dns\""));

    // Test Prometheus export
    let prom_exporter = PrometheusExporter::new("trustchain");
    let prom_output = prom_exporter.export(&metrics).await
        .expect("Failed to export Prometheus metrics");
    assert!(prom_output.contains("# HELP"));
    assert!(prom_output.contains("# TYPE"));
    assert!(prom_output.contains("trustchain_ca_total"));

    println!("Metrics export formats test passed!");
}

#[tokio::test]
async fn test_monitoring_without_external_dependencies() {
    // This test verifies that monitoring works without any external services
    let config = MonitoringConfig {
        enabled: true,
        collection_interval: 1, // Fast collection for testing
        health_check_interval: 1,
        enable_export: true,
        export_format: trustchain::monitoring::export::ExportFormat::Json,
        retention_seconds: 60,
        alert_thresholds: Default::default(),
    };

    let monitoring = MonitoringSystem::new(config).await
        .expect("Failed to create monitoring system");

    monitoring.start().await
        .expect("Failed to start monitoring");

    // Wait a bit for the monitoring to collect data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify system is running
    let health = monitoring.get_health().await;
    assert!(health.is_healthy);

    let metrics = monitoring.get_metrics().await;
    assert!(metrics.timestamp > std::time::SystemTime::UNIX_EPOCH);

    // Get system info
    let info = monitoring.get_system_info().await;
    assert!(!info.version.is_empty());
    assert!(info.uptime_seconds >= 2);

    println!("Monitoring without external dependencies test passed!");
}
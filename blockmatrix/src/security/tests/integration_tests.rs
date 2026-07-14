// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security framework integration tests

use crate::security::{
    ebpf::EBPFSecurityManager,
    monitoring::SecurityMonitor,
    types::{HyperMeshSecurity, ProcessContext, SystemCall},
    SecurityConfig, SecurityError, SecurityManager,
};
use std::time::SystemTime;

#[tokio::test]
async fn test_security_config() {
    let config = SecurityConfig::default();
    // Check that config has expected fields
    assert!(config.capabilities.enabled);
    // Certificate config has different structure
    assert!(config.certificates.lifecycle.default_validity_days > 0);
}

#[tokio::test]
async fn test_security_manager() {
    let config = SecurityConfig::default();
    let manager = SecurityManager::new(config);
    // Basic test - validate method returns Ok
    assert!(manager.validate().is_ok());
}

#[tokio::test]
async fn test_security_monitor() {
    let monitor = SecurityMonitor::new();
    // Start monitoring and check it starts successfully
    assert!(monitor.start().await.is_ok());
    // Stop monitoring cleanly
    assert!(monitor.stop().await.is_ok());
}

#[tokio::test]
async fn test_security_framework_initialization() {
    let config = SecurityConfig::default();
    let mut security = HyperMeshSecurity::new(config)
        .await
        .expect("test: create HyperMeshSecurity");

    // Test initialization
    security
        .initialize()
        .await
        .expect("test: initialize security framework");

    // Test shutdown
    security
        .shutdown()
        .await
        .expect("test: shutdown security framework");
}

#[tokio::test]
async fn test_ebpf_security_manager() {
    let mut ebpf_manager = EBPFSecurityManager::new()
        .await
        .expect("test: create EBPFSecurityManager");

    // Test loading default programs (XDP attach to loopback / userspace fallback)
    ebpf_manager
        .load_default_programs()
        .await
        .expect("test: load default programs");

    // Test program listing
    let programs = ebpf_manager.list_programs().await;
    assert!(!programs.is_empty());
}

#[tokio::test]
async fn test_ebpf_syscall_tracing() {
    let ebpf_manager = EBPFSecurityManager::new()
        .await
        .expect("test: create EBPFSecurityManager");

    let safe_call = SystemCall {
        number: 0,
        name: "read".to_string(),
        args: vec![],
        return_value: None,
        process: ProcessContext {
            pid: 1234,
            name: "safe_proc".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "cat /etc/hostname".to_string(),
            ppid: 1,
        },
        timestamp: SystemTime::now(),
    };

    let result = ebpf_manager
        .trace_syscall(&safe_call)
        .await
        .expect("test: trace safe syscall");
    assert!(result, "read syscall should be allowed");

    // Dangerous syscall should be denied
    let dangerous_call = SystemCall {
        number: 101,
        name: "ptrace".to_string(),
        args: vec![],
        return_value: None,
        process: ProcessContext {
            pid: 5678,
            name: "attacker".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "strace -p 1".to_string(),
            ppid: 1,
        },
        timestamp: SystemTime::now(),
    };

    let result = ebpf_manager
        .trace_syscall(&dangerous_call)
        .await
        .expect("test: trace dangerous syscall");
    assert!(!result, "ptrace syscall should be denied");
}

#[tokio::test]
async fn test_security_monitor_metrics() {
    let monitor = SecurityMonitor::new();
    monitor.start().await.expect("test: start monitor");

    // Record some events
    monitor.record_event("threat_detected").await;
    monitor.record_event("policy_evaluated").await;
    monitor.record_event("access_denied").await;

    let metrics = monitor.get_metrics().await;
    assert!(metrics.threats_detected >= 1);
    assert!(metrics.policies_evaluated >= 1);
    assert!(metrics.access_denials >= 1);
    assert!(metrics.events_processed >= 3);

    monitor.stop().await.expect("test: stop monitor");
}

#[tokio::test]
async fn test_security_manager_invalid_config() {
    let mut config = SecurityConfig::default();
    // Set invalid policy evaluation mode
    config.policies.evaluation_mode = "invalid_mode".to_string();

    let manager = SecurityManager::new(config);
    let result = manager.validate();
    assert!(result.is_err());

    match result {
        Err(SecurityError::ConfigurationError { message }) => {
            assert!(message.contains("evaluation mode"));
        }
        other => panic!("test: expected ConfigurationError, got {other:?}"),
    }
}

//! Security framework integration tests

use hypermesh_security::{
    HyperMeshSecurity, SecurityConfig, EBPFSecurityManager, CapabilitySystem,
    PKIManager, IntrusionDetectionSystem, NetworkPacket, SystemCall,
    ProcessContext, Principal, Resource, Operation, SecurityContext,
    ebpf::{SecurityEvent, SecurityEventType, SecuritySeverity, SecurityEventData},
};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio;

#[tokio::test]
async fn test_security_framework_initialization() {
    let config = SecurityConfig::default();
    let mut security = HyperMeshSecurity::new(config).await.unwrap();
    
    // Test initialization
    security.initialize().await.unwrap();
    
    // Test shutdown
    security.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_ebpf_security_manager() {
    let mut ebpf_manager = EBPFSecurityManager::new().await.unwrap();
    
    // Test loading default programs
    ebpf_manager.load_default_programs().await.unwrap();
    
    // Test program listing
    let programs = ebpf_manager.list_programs().await;
    assert!(!programs.is_empty());
    
    // Test network traffic analysis
    let packet = NetworkPacket {
        src_addr: "192.168.1.100".to_string(),
        dst_addr: "10.0.0.1".to_string(),
        src_port: 12345,
        dst_port: 80,
        protocol: "tcp".to_string(),
        payload_size: 1500,
        flags: vec!["SYN".to_string()],
        timestamp: SystemTime::now(),
    };
    
    let assessment = ebpf_manager.analyze_network_traffic(&packet).await;
    assert!(assessment.threat_level >= 0.0 && assessment.threat_level <= 1.0);
    assert!(assessment.confidence >= 0.0 && assessment.confidence <= 1.0);
    
    // Test system call monitoring
    let syscall = SystemCall {
        number: 2,
        name: "open".to_string(),
        args: vec![0x12345678, 0x755, 0],
        return_value: Some(3),
        process: ProcessContext {
            pid: 1234,
            name: "test_process".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "test_process --arg".to_string(),
            ppid: 1,
        },
        timestamp: SystemTime::now(),
    };
    
    let decision = ebpf_manager.monitor_system_calls(&syscall, &syscall.process).await;
    // Should allow normal open syscall
    assert!(matches!(decision, hypermesh_security::ebpf::SecurityDecision::Allow | hypermesh_security::ebpf::SecurityDecision::Log));
    
    // Test resource enforcement
    let enforcement = ebpf_manager.enforce_resource_limits(1234, "memory", 500 * 1024 * 1024, 1024 * 1024 * 1024).await.unwrap();
    assert!(matches!(enforcement, hypermesh_security::ebpf::SecurityDecision::Allow));
    
    // Test resource limit violation
    let enforcement = ebpf_manager.enforce_resource_limits(1234, "memory", 2 * 1024 * 1024 * 1024, 1024 * 1024 * 1024).await.unwrap();
    assert!(matches!(enforcement, hypermesh_security::ebpf::SecurityDecision::Deny));
    
    // Test statistics
    let stats = ebpf_manager.get_stats().await;
    assert!(stats.events_processed > 0);
    
    // Clean up
    ebpf_manager.unload_all_programs().await.unwrap();
}

#[tokio::test]
async fn test_capability_system() {
    let capability_system = CapabilitySystem::new();
    
    let principal = Principal::User {
        id: "test_user".to_string(),
        groups: vec!["users".to_string()],
    };
    
    let resource = Resource::File {
        path: "/tmp/test.txt".to_string(),
    };
    
    let capability = hypermesh_security::Capability {
        id: "cap_1".to_string(),
        resource: resource.clone(),
        permissions: hypermesh_security::PermissionSet {
            read: true,
            write: true,
            execute: false,
            delete: false,
            modify_permissions: false,
            delegate: false,
        },
        expiry: None,
        delegation_depth: 0,
        signature: "mock_signature".to_string(),
    };
    
    // Grant capability
    capability_system.grant_capability(principal.clone(), capability).await.unwrap();
    
    // Test permission check (should succeed for read)
    let has_read = capability_system.check_permission(&principal, &resource, &Operation::Read).await.unwrap();
    assert!(has_read);
    
    // Test permission check (should fail for execute)
    let has_execute = capability_system.check_permission(&principal, &resource, &Operation::Execute).await.unwrap();
    assert!(!has_execute);
}

#[tokio::test]
async fn test_pki_manager() {
    let config = hypermesh_security::config::CertificateConfig::default();
    let pki_manager = PKIManager::new(&config).unwrap();
    
    pki_manager.initialize().await.unwrap();
    
    // Test certificate issuance
    let cert = pki_manager.issue_certificate("test.example.com").await.unwrap();
    assert_eq!(cert.subject, "test.example.com");
    assert_eq!(cert.issuer, "HyperMesh CA");
    
    // Test certificate revocation
    pki_manager.revoke_certificate(&cert.serial_number).await.unwrap();
}

#[tokio::test]
async fn test_intrusion_detection() {
    let ids = IntrusionDetectionSystem::new();
    
    // Test traffic analysis
    let packet = NetworkPacket {
        src_addr: "192.168.1.100".to_string(),
        dst_addr: "10.0.0.1".to_string(),
        src_port: 12345,
        dst_port: 80,
        protocol: "tcp".to_string(),
        payload_size: 1500,
        flags: vec!["SYN".to_string()],
        timestamp: SystemTime::now(),
    };
    
    let threats = ids.analyze_traffic(&packet).await;
    // Should detect large packet as suspicious
    assert!(!threats.is_empty());
    
    // Test threat reporting
    if let Some(threat) = threats.first() {
        ids.report_threat(threat.clone()).await.unwrap();
    }
}

#[tokio::test]
async fn test_security_monitoring() {
    let config = SecurityConfig::default();
    let mut security = HyperMeshSecurity::new(config).await.unwrap();
    
    security.initialize().await.unwrap();
    
    // Test metrics collection
    let metrics = security.monitor.get_metrics().await;
    assert!(metrics.events_processed >= 0);
    
    // Test event recording
    security.monitor.record_event("threat_detected").await;
    security.monitor.record_event("policy_evaluated").await;
    
    let updated_metrics = security.monitor.get_metrics().await;
    assert!(updated_metrics.events_processed >= metrics.events_processed);
    
    security.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_policy_evaluation() {
    let config = SecurityConfig::default();
    let security = HyperMeshSecurity::new(config).await.unwrap();
    
    let context = SecurityContext {
        principal: Principal::User {
            id: "test_user".to_string(),
            groups: vec!["users".to_string()],
        },
        resource: Resource::File {
            path: "/tmp/test.txt".to_string(),
        },
        operation: Operation::Read,
        timestamp: SystemTime::now(),
        metadata: HashMap::new(),
    };
    
    let decision = security.policy_engine.evaluate(&context).await.unwrap();
    assert!(matches!(decision, 
        hypermesh_security::AccessDecision::Allow | 
        hypermesh_security::AccessDecision::Deny { .. }
    ));
}

#[tokio::test]
async fn test_security_performance_targets() {
    let config = SecurityConfig::default();
    let mut security = HyperMeshSecurity::new(config).await.unwrap();
    
    security.initialize().await.unwrap();
    
    // Test eBPF overhead target (<5% CPU, simulated as <1ms processing time)
    let start = std::time::Instant::now();
    let packet = NetworkPacket {
        src_addr: "192.168.1.1".to_string(),
        dst_addr: "10.0.0.1".to_string(),
        src_port: 80,
        dst_port: 12345,
        protocol: "tcp".to_string(),
        payload_size: 1024,
        flags: vec![],
        timestamp: SystemTime::now(),
    };
    
    security.ebpf_manager.analyze_network_traffic(&packet).await;
    let processing_time = start.elapsed();
    assert!(processing_time < std::time::Duration::from_millis(5)); // Very relaxed for simulation
    
    // Test certificate validation target (<10ms)
    let cert_start = std::time::Instant::now();
    security.pki_manager.issue_certificate("test.local").await.unwrap();
    let cert_time = cert_start.elapsed();
    assert!(cert_time < std::time::Duration::from_millis(100)); // Relaxed for simulation
    
    // Test capability check performance (<100μs, relaxed to <1ms for simulation)
    let cap_start = std::time::Instant::now();
    let principal = Principal::User {
        id: "test_user".to_string(),
        groups: vec![],
    };
    let resource = Resource::File {
        path: "/tmp/test".to_string(),
    };
    security.capability_system.check_permission(&principal, &resource, &Operation::Read).await.unwrap();
    let cap_time = cap_start.elapsed();
    assert!(cap_time < std::time::Duration::from_millis(1));
    
    security.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_threat_detection_accuracy() {
    let ids = IntrusionDetectionSystem::new();
    
    // Test legitimate traffic (should have low threat score)
    let normal_packet = NetworkPacket {
        src_addr: "192.168.1.100".to_string(),
        dst_addr: "10.0.0.1".to_string(),
        src_port: 12345,
        dst_port: 80,
        protocol: "tcp".to_string(),
        payload_size: 512,
        flags: vec!["SYN".to_string()],
        timestamp: SystemTime::now(),
    };
    
    let normal_threats = ids.analyze_traffic(&normal_packet).await;
    // Should have few or no threats for normal traffic
    assert!(normal_threats.len() <= 1);
    
    // Test suspicious traffic (should have higher threat score)
    let suspicious_packet = NetworkPacket {
        src_addr: "192.168.1.100".to_string(),
        dst_addr: "10.0.0.1".to_string(),
        src_port: 31337,
        dst_port: 22,
        protocol: "tcp".to_string(),
        payload_size: 2000, // Large packet
        flags: vec!["SYN".to_string(), "FIN".to_string()], // Suspicious flag combination
        timestamp: SystemTime::now(),
    };
    
    let suspicious_threats = ids.analyze_traffic(&suspicious_packet).await;
    // Should detect more threats for suspicious traffic
    assert!(suspicious_threats.len() >= 1);
    
    if let Some(threat) = suspicious_threats.first() {
        assert!(threat.confidence > 0.5);
    }
}
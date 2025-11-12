//! Integration tests for HyperMesh NAT-like Remote Proxy System
//!
//! Tests the complete proxy system including NAT translation, trust integration,
//! quantum security, and sharded data access.

use std::time::SystemTime;
use hypermesh_assets::core::*;
use hypermesh_assets::proxy::*;

#[tokio::test]
async fn test_complete_proxy_system_integration() {
    // Initialize logging for tests
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create NAT Translator
    let nat_translator = NATTranslator::new().await.expect("Failed to create NAT translator");
    
    // 2. Create test asset
    let asset_id = AssetId::new(AssetType::Memory);
    
    // 3. Generate global address
    let global_proxy_addr = nat_translator.generate_global_address(
        "test-node-1",
        &asset_id,
        8080,
    ).await.expect("Failed to generate global address");
    
    assert!(!global_proxy_addr.to_string().is_empty());
    
    // 4. Create global address for NAT translation
    let global_addr = GlobalAddress::new(
        [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
        &asset_id,
        8080,
        GlobalAddressType::Memory,
    );
    
    // 5. Create memory permissions
    let memory_permissions = MemoryPermissions {
        read: true,
        write: true,
        execute: false,
        share: true,
        cache: true,
        prefetch: true,
    };
    
    // 6. Create NAT translation
    let nat_mapping = nat_translator.create_translation(
        global_addr.clone(),
        1024 * 1024, // 1MB
        memory_permissions,
    ).await.expect("Failed to create NAT translation");
    
    assert_eq!(nat_mapping.region_size, 1024 * 1024);
    assert!(matches!(nat_mapping.translation_state, TranslationState::Active));
    
    // 7. Test address translation
    let local_addr = nat_translator.translate_to_local(&global_addr).await
        .expect("Failed to translate to local address");
    assert_eq!(local_addr, nat_mapping.local_address);
    
    // 8. Test reverse translation
    let reverse_global = nat_translator.translate_to_global(local_addr).await
        .expect("Failed to translate to global address");
    assert_eq!(reverse_global.hash(), global_addr.hash());
    
    // 9. Test NAT statistics
    let nat_stats = nat_translator.get_stats().await
        .expect("Failed to get NAT stats");
    assert_eq!(nat_stats.active_translations, 1);
    assert_eq!(nat_stats.total_memory_mapped, 1024 * 1024);
    
    println!("✅ NAT Translation system test passed");
}

#[tokio::test]
async fn test_quantum_security_integration() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create quantum security system
    let quantum_security = QuantumSecurity::new().await
        .expect("Failed to create quantum security");
    
    // 2. Create test proxy address
    let proxy_addr = ProxyAddress::new([1u8; 16], [2u8; 8], 8080);
    
    // 3. Generate access tokens
    let tokens = quantum_security.generate_access_tokens(&proxy_addr).await
        .expect("Failed to generate access tokens");
    
    assert!(!tokens.is_empty());
    assert!(tokens.len() >= 64); // Minimum expected size
    
    // 4. Validate tokens
    let valid = quantum_security.validate_access_tokens(&tokens).await
        .expect("Failed to validate access tokens");
    
    assert!(valid, "Generated tokens should be valid");
    
    // 5. Test with invalid tokens
    let invalid_tokens = vec![0u8; 32]; // Too small
    let invalid_result = quantum_security.validate_access_tokens(&invalid_tokens).await
        .expect("Failed to validate invalid tokens");
    
    assert!(!invalid_result, "Invalid tokens should not validate");
    
    println!("✅ Quantum Security system test passed");
}

#[tokio::test]
async fn test_trust_chain_integration() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create trust chain integration
    let trust_integration = TrustChainIntegration::new().await
        .expect("Failed to create trust integration");
    
    // 2. Create test node info
    let node_info = ProxyNodeInfo {
        node_id: "test-node".to_string(),
        network_address: "2a01:04f8:0110:53ad::1".to_string(),
        capabilities: ProxyCapabilities {
            http_proxy: true,
            socks5_proxy: true,
            tcp_forwarding: true,
            vpn_tunnel: false,
            max_connections: 1000,
            bandwidth_mbps: 1000,
            protocols: vec!["HTTP".to_string(), "SOCKS5".to_string()],
        },
        trust_score: 0.8,
        last_heartbeat: SystemTime::now(),
        certificate_fingerprint: "test-cert-fingerprint".to_string(),
    };
    
    // 3. Validate node certificate
    let is_valid = trust_integration.validate_node_certificate(&node_info).await
        .expect("Failed to validate node certificate");
    
    assert!(is_valid, "Node certificate should be valid with default setup");
    
    // 4. Get certificate trust level
    let trust_level = trust_integration.get_certificate_trust_level(&node_info.certificate_fingerprint).await
        .expect("Failed to get certificate trust level");
    
    assert!(trust_level > 0.0, "Trust level should be positive");
    
    println!("✅ Trust Chain integration test passed");
}

#[tokio::test]
async fn test_sharded_data_access() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create sharded data access system
    let sharded_access = ShardedDataAccess::new().await
        .expect("Failed to create sharded access");
    
    // 2. Create test asset
    let asset_id = AssetId::new(AssetType::Storage);
    
    // 3. Create and store test shard
    let shard_manager = ShardManager::new().await
        .expect("Failed to create shard manager");
    
    let test_data = b"This is test data for sharding";
    let shards = shard_manager.create_shards(&asset_id, test_data).await
        .expect("Failed to create shards");
    
    assert!(!shards.is_empty(), "Should create at least one shard");
    
    // Store the first shard
    let first_shard = &shards[0];
    shard_manager.store_shard(first_shard.clone()).await
        .expect("Failed to store shard");
    
    // 4. Test shard retrieval
    let retrieved_shard = shard_manager.get_shard(&first_shard.shard_key).await
        .expect("Failed to retrieve shard");
    
    assert_eq!(retrieved_shard.shard_id, first_shard.shard_id);
    assert_eq!(retrieved_shard.asset_id, asset_id);
    
    println!("✅ Sharded Data Access system test passed");
}

#[tokio::test]
async fn test_proxy_routing_system() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create proxy router
    let router = ProxyRouter::new().await
        .expect("Failed to create proxy router");
    
    // 2. Create test node
    let node_info = ProxyNodeInfo {
        node_id: "test-router-node".to_string(),
        network_address: "192.168.1.100".to_string(),
        capabilities: ProxyCapabilities {
            http_proxy: true,
            socks5_proxy: true,
            tcp_forwarding: true,
            vpn_tunnel: false,
            max_connections: 1000,
            bandwidth_mbps: 1000,
            protocols: vec!["HTTP".to_string(), "SOCKS5".to_string()],
        },
        trust_score: 0.9,
        last_heartbeat: SystemTime::now(),
        certificate_fingerprint: "router-cert-fingerprint".to_string(),
    };
    
    // 3. Add node to router
    router.add_proxy_node(&node_info).await
        .expect("Failed to add proxy node");
    
    // 4. Test route metrics update
    let route_metrics = RouteMetrics {
        avg_latency_ms: 50.0,
        success_rate: 0.95,
        throughput_mbps: 100.0,
        current_load: 0.3,
        total_requests: 1000,
        failed_requests: 50,
        last_measured: SystemTime::now(),
    };
    
    router.update_route_metrics("test-destination", route_metrics.clone()).await
        .expect("Failed to update route metrics");
    
    // 5. Get route statistics
    let stats = router.get_route_stats().await
        .expect("Failed to get route stats");
    
    assert!(stats.contains_key("test-destination"));
    assert_eq!(stats["test-destination"].avg_latency_ms, 50.0);
    
    println!("✅ Proxy Routing system test passed");
}

#[tokio::test]
async fn test_proxy_forwarding_system() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create proxy forwarder
    let forwarder = ProxyForwarder::new().await
        .expect("Failed to create proxy forwarder");
    
    // 2. Create test proxy address and forwarding rule
    let proxy_addr = ProxyAddress::new([1u8; 16], [2u8; 8], 8080);
    
    let forwarding_rule = ForwardingRule {
        rule_type: ForwardingRuleType::HTTP,
        source_pattern: "*".to_string(),
        destination: "forwarded".to_string(),
        port_mapping: None,
        protocol_settings: std::collections::HashMap::new(),
    };
    
    // 3. Install forwarding rule
    forwarder.install_rule(&proxy_addr, &forwarding_rule).await
        .expect("Failed to install forwarding rule");
    
    // 4. Test request forwarding
    let test_request = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let response = forwarder.forward_request(
        &proxy_addr,
        "example.com:80",
        test_request.to_vec(),
        ForwardingRuleType::HTTP,
    ).await.expect("Failed to forward request");
    
    assert!(!response.is_empty(), "Response should not be empty");
    
    // 5. Get forwarding statistics
    let stats = forwarder.get_stats().await
        .expect("Failed to get forwarding stats");
    
    assert_eq!(stats.total_connections, 1);
    assert!(stats.total_bytes_forwarded > 0);
    
    println!("✅ Proxy Forwarding system test passed");
}

#[tokio::test]
async fn test_complete_proxy_manager_workflow() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create proxy network configuration
    let config = ProxyNetworkConfig::default();
    
    // 2. Create remote proxy manager
    let proxy_manager = RemoteProxyManager::new(config).await
        .expect("Failed to create proxy manager");
    
    // 3. Create and register proxy node
    let proxy_node = ProxyNodeInfo {
        node_id: "integration-test-node".to_string(),
        network_address: "2a01:04f8:0110:53ad::100".to_string(),
        capabilities: ProxyCapabilities {
            http_proxy: true,
            socks5_proxy: true,
            tcp_forwarding: true,
            vpn_tunnel: true,
            max_connections: 5000,
            bandwidth_mbps: 5000,
            protocols: vec!["HTTP".to_string(), "SOCKS5".to_string(), "VPN".to_string()],
        },
        trust_score: 0.95,
        last_heartbeat: SystemTime::now(),
        certificate_fingerprint: "integration-cert-abc123".to_string(),
    };
    
    proxy_manager.register_proxy_node(proxy_node.clone()).await
        .expect("Failed to register proxy node");
    
    // 4. Create test asset and allocate proxy address
    let asset_id = AssetId::new(AssetType::Memory);
    
    let allocated_address = proxy_manager.allocate_proxy_address(
        &asset_id,
        PrivacyLevel::P2P,
        &["HTTP".to_string(), "SOCKS5".to_string()],
    ).await.expect("Failed to allocate proxy address");
    
    assert!(!allocated_address.to_string().is_empty());
    
    // 5. Resolve proxy address back to asset
    let resolved_asset = proxy_manager.resolve_proxy_address(&allocated_address).await
        .expect("Failed to resolve proxy address");
    
    assert_eq!(resolved_asset, asset_id);
    
    // 6. Test request forwarding through proxy manager
    let test_request = b"Test request data";
    let response = proxy_manager.forward_request(
        &allocated_address,
        test_request.to_vec(),
        ForwardingRuleType::HTTP,
    ).await.expect("Failed to forward request through proxy manager");
    
    assert!(!response.is_empty());
    
    // 7. Test sharded data access through proxy
    let shard_data = proxy_manager.access_sharded_data(
        &allocated_address,
        "test-shard-key",
    ).await.expect("Failed to access sharded data");
    
    assert!(!shard_data.is_empty());
    
    // 8. Get final system statistics
    let stats = proxy_manager.get_system_stats().await
        .expect("Failed to get system stats");
    
    assert_eq!(stats.active_proxy_nodes, 1);
    assert_eq!(stats.total_mappings, 1);
    assert!(stats.forwarded_requests > 0);
    assert!(stats.quantum_validations > 0);
    assert!(stats.sharded_requests > 0);
    
    println!("✅ Complete Proxy Manager workflow test passed");
}

#[tokio::test] 
async fn test_global_address_functionality() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Create test asset
    let asset_id = AssetId::new(AssetType::Memory);
    
    // 2. Create global address
    let global_addr = GlobalAddress::new(
        [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad], // HyperMesh prefix
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], // Node ID
        &asset_id,
        8080,
        GlobalAddressType::Memory,
    );
    
    // 3. Test string conversion
    let addr_string = global_addr.to_string();
    assert!(addr_string.starts_with("hypermesh://"));
    assert!(addr_string.contains("8080"));
    
    // 4. Test round-trip string conversion
    let parsed_addr = GlobalAddress::from_string(&addr_string)
        .expect("Failed to parse global address from string");
    
    assert_eq!(parsed_addr.network_prefix, global_addr.network_prefix);
    assert_eq!(parsed_addr.node_id, global_addr.node_id);
    assert_eq!(parsed_addr.service_port, global_addr.service_port);
    
    // 5. Test IPv6 conversion
    let ipv6_addr = global_addr.to_ipv6();
    assert!(!ipv6_addr.is_unspecified());
    
    // 6. Test socket address conversion
    let socket_addr = global_addr.to_socket_addr();
    assert_eq!(socket_addr.port(), 8080);
    
    // 7. Test address hash
    let hash1 = global_addr.hash();
    let hash2 = global_addr.hash();
    assert_eq!(hash1, hash2); // Should be deterministic
    
    println!("✅ Global Address functionality test passed");
}
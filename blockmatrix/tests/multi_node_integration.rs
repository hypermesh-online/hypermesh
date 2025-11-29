//! Integration tests for HyperMesh multi-node asset system

use blockmatrix::assets::{
    AssetManager, AssetId, AssetType, ConsensusProof,
    MultiNodeCoordinator, NodeInfo, NodeCapabilities,
    ConsensusManager, NetworkTopology,
};
use blockmatrix::assets::multi_node::{
    NodeId, AllocationDecision, ResourceSharingRequest,
    ResourceAmount, PrivacyLevel, MultiNodeEvent,
    ConsensusProposal, ProposalType, ProposalData,
    MigrationPlan, MigrationPriority,
};
use blockmatrix::assets::proxy::{
    RemoteMemoryTransport, TransportConfig, GlobalAddress,
    MemoryOperationType, MemoryPermissions,
};
use std::time::{Duration, SystemTime};
use tokio;

/// Create test node ID
fn create_test_node(id: u8) -> NodeId {
    NodeId {
        id: [id; 32],
        ipv6_address: format!("2001:db8::{}",id).parse().unwrap(),
        public_key: vec![id; 64],
        trust_score: 0.95,
    }
}

#[tokio::test]
async fn test_multi_node_coordinator_initialization() {
    use blockmatrix::assets::multi_node::coordinator::{
        MultiNodeCoordinator, CoordinatorConfig,
    };

    let config = CoordinatorConfig::default();
    let mut coordinator = MultiNodeCoordinator::new(config);

    let local_node = create_test_node(1);
    coordinator.initialize(local_node.clone()).await.unwrap();

    // Join the network
    coordinator.join_network().await.unwrap();

    // Verify we can get topology
    let topology = coordinator.get_topology().await.unwrap();
    assert_eq!(topology.nodes.len(), 0); // Initially empty
}

#[tokio::test]
async fn test_consensus_manager() {
    use blockmatrix::assets::multi_node::consensus::{
        ConsensusManager, ConsensusConfig, ConsensusProposal,
        ProposalType, ProposalData, Vote, VoteValue,
    };

    let node = create_test_node(1);
    let config = ConsensusConfig::default();
    let manager = ConsensusManager::new(node.clone(), config);

    // Create a proposal
    let proposal = ConsensusProposal {
        proposal_id: "test-proposal".to_string(),
        proposal_type: ProposalType::AssetAllocation,
        proposer: node.clone(),
        data: ProposalData::Configuration {
            key: "test-key".to_string(),
            value: "test-value".to_string(),
        },
        timestamp: SystemTime::now(),
        signature: vec![1, 2, 3, 4],
    };

    // Submit proposal
    let round_id = manager.submit_proposal(proposal).await.unwrap();
    assert!(!round_id.is_empty());

    // Submit a vote
    let vote = Vote {
        voter: node.clone(),
        value: VoteValue::Accept,
        timestamp: SystemTime::now(),
        signature: vec![5, 6, 7, 8],
        justification: Some("Test vote".to_string()),
    };

    manager.submit_vote(&round_id, vote).await.unwrap();
}

#[tokio::test]
async fn test_asset_migration() {
    use blockmatrix::assets::multi_node::migration::{
        AssetMigrator, MigrationConfig, MigrationPriority,
    };

    let config = MigrationConfig::default();
    let migrator = AssetMigrator::new(config);

    let asset_id = AssetId::new(AssetType::Memory);
    let source = create_test_node(1);
    let target = create_test_node(2);

    // Plan migration
    let plan = migrator.plan_migration(
        asset_id.clone(),
        source,
        target,
        MigrationPriority::Normal,
    ).await.unwrap();

    assert_eq!(plan.asset_id, asset_id);
    assert!(plan.estimated_duration > Duration::from_secs(0));

    // Execute migration
    let status = migrator.execute_migration(&plan).await.unwrap();
    assert_eq!(status.progress, 100.0);
}

#[tokio::test]
async fn test_node_discovery() {
    use blockmatrix::assets::multi_node::discovery::{
        NodeDiscovery, DiscoveryProtocol, DiscoveryConfig,
        ServiceAnnouncement, DiscoveredNode,
    };

    let local_node = create_test_node(1);
    let config = DiscoveryConfig::default();
    let discovery = NodeDiscovery::new(
        local_node.clone(),
        DiscoveryProtocol::Hybrid,
        config,
    );

    // Start discovery
    discovery.start().await.unwrap();

    // Announce a service
    let announcement = ServiceAnnouncement {
        service_name: "hypermesh-asset".to_string(),
        version: "1.0.0".to_string(),
        provider: local_node.clone(),
        endpoint: "http3://[2001:db8::1]:8080".to_string(),
        metadata: std::collections::HashMap::new(),
        announced_at: SystemTime::now(),
        ttl: Duration::from_secs(3600),
    };

    discovery.announce_service(announcement).await.unwrap();

    // Discover services
    let services = discovery.discover_services("hypermesh-asset").await;
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].service_name, "hypermesh-asset");
}

#[tokio::test]
async fn test_load_balancer() {
    use blockmatrix::assets::multi_node::load_balancer::{
        LoadBalancer, BalancingStrategy, LoadBalancerConfig,
        ResourceMetrics,
    };

    let config = LoadBalancerConfig::default();
    let balancer = LoadBalancer::new(BalancingStrategy::ResourceAware, config);

    // Update metrics for multiple nodes
    for i in 1..=3 {
        let metrics = ResourceMetrics {
            node_id: create_test_node(i),
            cpu_utilization: 0.3 + (i as f64 * 0.2),
            memory_utilization: 0.2 + (i as f64 * 0.1),
            network_utilization: 0.1,
            storage_utilization: 0.4,
            active_connections: 10 * i as u64,
            request_rate: 100.0 * i as f64,
            avg_response_time: 10.0 + i as f64,
            timestamp: SystemTime::now(),
        };
        balancer.update_metrics(metrics).await;
    }

    // Select best node
    let selected_node = balancer.select_node(AssetType::Memory).await.unwrap();
    assert_eq!(selected_node.id[0], 1); // Node 1 should have lowest load
}

#[tokio::test]
async fn test_byzantine_detection() {
    use blockmatrix::assets::multi_node::fault_tolerance::{
        ByzantineDetector, ByzantineConfig, SuspiciousEvent,
    };

    let config = ByzantineConfig::default();
    let detector = ByzantineDetector::new(config);

    let byzantine_node = create_test_node(99);

    // Report multiple suspicious events
    for i in 0..10 {
        let event = SuspiciousEvent::ExcessiveFailures {
            failure_rate: 0.8 + (i as f64 * 0.02),
        };
        detector.report_suspicious_behavior(byzantine_node.clone(), event).await;
    }

    // Check if node is marked as Byzantine
    let is_byzantine = detector.is_byzantine(&byzantine_node).await;
    assert!(is_byzantine);

    let byzantine_nodes = detector.get_byzantine_nodes().await;
    assert_eq!(byzantine_nodes.len(), 1);
    assert_eq!(byzantine_nodes[0].id[0], 99);
}

#[tokio::test]
async fn test_resource_sharing() {
    use blockmatrix::assets::multi_node::resource_sharing::{
        ResourceSharing, SharingProtocol, PricingModel,
        SharingConfig, ResourceOffer, ResourceRequest,
        ServiceLevelAgreement, DataLocalityRequirement,
    };

    let config = SharingConfig::default();
    let sharing = ResourceSharing::new(
        SharingProtocol::Market,
        PricingModel::Dynamic,
        config,
    );

    let provider = create_test_node(1);
    let consumer = create_test_node(2);

    // Submit offer
    let offer = ResourceOffer {
        offer_id: "offer-1".to_string(),
        provider: provider.clone(),
        resource_type: AssetType::Memory,
        available_amount: ResourceAmount::MemoryBytes(8 * 1024 * 1024 * 1024),
        price_per_hour: 0.10,
        min_commitment: Duration::from_secs(3600),
        max_commitment: Duration::from_secs(86400),
        sla: ServiceLevelAgreement {
            uptime_guarantee: 99.9,
            max_latency_ms: 10,
            min_bandwidth_mbps: 1000,
            data_locality: DataLocalityRequirement::SameRegion,
            penalty_rate: 0.1,
        },
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        privacy_requirements: PrivacyLevel::PublicNetwork,
    };

    sharing.submit_offer(offer).await.unwrap();

    // Submit request
    let request = ResourceRequest {
        request_id: "request-1".to_string(),
        consumer,
        resource_type: AssetType::Memory,
        requested_amount: ResourceAmount::MemoryBytes(4 * 1024 * 1024 * 1024),
        max_price_per_hour: 0.15,
        duration: Duration::from_secs(7200),
        required_sla: ServiceLevelAgreement {
            uptime_guarantee: 99.0,
            max_latency_ms: 20,
            min_bandwidth_mbps: 500,
            data_locality: DataLocalityRequirement::SameRegion,
            penalty_rate: 0.05,
        },
        expires_at: SystemTime::now() + Duration::from_secs(1800),
        privacy_requirements: PrivacyLevel::PublicNetwork,
    };

    sharing.submit_request(request).await.unwrap();

    // Check if agreement was created
    let agreements = sharing.get_active_agreements().await;
    assert_eq!(agreements.len(), 1);
    assert_eq!(agreements[0].provider.id[0], 1);
    assert_eq!(agreements[0].consumer.id[0], 2);
}

#[tokio::test]
async fn test_remote_memory_transport() {
    use quinn::{Endpoint, ServerConfig, ClientConfig};
    use std::sync::Arc;
    use bytes::Bytes;

    // Create dummy endpoint for testing
    let server_config = ServerConfig::with_crypto(Arc::new(
        rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![], rustls::PrivateKey(vec![]))
            .unwrap()
    ));

    let endpoint = Endpoint::server(
        server_config,
        "127.0.0.1:0".parse().unwrap(),
    ).unwrap();

    let config = TransportConfig::default();
    let transport = RemoteMemoryTransport::new(endpoint, config).await.unwrap();

    // Create global address
    let global_addr = GlobalAddress {
        network_prefix: [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
        node_id: [1u8; 8],
        asset_id: [2u8; 16],
        service_port: 8080,
        address_type: hypermesh::assets::proxy::nat_translation::GlobalAddressType::Memory,
        created_at: SystemTime::now(),
    };

    // Map remote memory
    let permissions = MemoryPermissions {
        read: true,
        write: true,
        execute: false,
        share: true,
        cache: true,
        prefetch: false,
    };

    // Note: Actual remote operations would require a connected peer
    // This test verifies the API structure
    assert!(transport.get_metrics().await.total_operations == 0);
}

#[tokio::test]
async fn test_end_to_end_multi_node_flow() {
    use blockmatrix::assets::multi_node::coordinator::{
        MultiNodeCoordinator, CoordinatorConfig,
    };
    use blockmatrix::assets::multi_node::{MultiNodeCoordinatorTrait};

    // Create coordinator
    let config = CoordinatorConfig::default();
    let mut coordinator = MultiNodeCoordinator::new(config);

    // Initialize with local node
    let local_node = create_test_node(1);
    coordinator.initialize(local_node.clone()).await.unwrap();

    // Join network
    coordinator.join_network().await.unwrap();

    // Allocate an asset
    let asset_id = AssetId::new(AssetType::Memory);
    let decision = coordinator.allocate_asset(asset_id.clone()).await.unwrap();

    assert_eq!(decision.asset_id, asset_id);
    assert!(decision.score > 0.0);

    // Handle a node failure event
    let failed_node = create_test_node(2);
    coordinator.handle_node_failure(failed_node).await.unwrap();

    // Get Byzantine nodes (should be empty)
    let byzantine_nodes = coordinator.detect_byzantine_nodes().await.unwrap();
    assert_eq!(byzantine_nodes.len(), 0);
}

#[test]
fn test_resource_amount_types() {
    let cpu = ResourceAmount::CpuCores(8);
    let memory = ResourceAmount::MemoryBytes(16 * 1024 * 1024 * 1024);
    let gpu = ResourceAmount::GpuUnits(2);
    let storage = ResourceAmount::StorageBytes(1024 * 1024 * 1024 * 1024);
    let bandwidth = ResourceAmount::BandwidthMbps(10000);

    match cpu {
        ResourceAmount::CpuCores(cores) => assert_eq!(cores, 8),
        _ => panic!("Wrong type"),
    }

    match memory {
        ResourceAmount::MemoryBytes(bytes) => assert_eq!(bytes, 16 * 1024 * 1024 * 1024),
        _ => panic!("Wrong type"),
    }

    match gpu {
        ResourceAmount::GpuUnits(units) => assert_eq!(units, 2),
        _ => panic!("Wrong type"),
    }

    match storage {
        ResourceAmount::StorageBytes(bytes) => assert_eq!(bytes, 1024 * 1024 * 1024 * 1024),
        _ => panic!("Wrong type"),
    }

    match bandwidth {
        ResourceAmount::BandwidthMbps(mbps) => assert_eq!(mbps, 10000),
        _ => panic!("Wrong type"),
    }
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS-as-Asset Integration Tests
//!
//! Sprint 3.3: Test scenarios for Nike (mixed), Bank (portal+private), Gov (fully federated)

use blockmatrix::dns::*;
use blockmatrix::consensus::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
};
use blockmatrix::consensus::ConsensusProof;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Create test consensus proof
fn create_test_proof() -> ConsensusProof {
    let stake = StakeProof::new(
        "test-holder".to_string(),
        "holder-id".to_string(),
        1000,
    );
    let time = TimeProof::new(Duration::from_secs(10));
    let space = SpaceProof::new(
        "test-node".to_string(),
        "/test/storage".to_string(),
        1024 * 1024,
    );
    let work = WorkProof::new(
        "test-owner".to_string(),
        "test-workload".to_string(),
        12345,
        100,
        WorkloadType::Compute,
        WorkState::Completed,
    );

    ConsensusProof::new(stake, time, space, work)
}

/// Create test DNS record
fn create_dns_record(domain: &str, ipv6: Ipv6Addr, owner: &str) -> DnsRecord {
    DnsRecord::new(
        domain.to_string(),
        DnsRecordType::AAAA,
        DnsRecordData::AAAA(ipv6),
        300,
        owner.to_string(),
    )
}

/// Setup DNS system for testing
async fn setup_dns_system() -> (
    Arc<DnsPoolManager>,
    Arc<DnsValidator>,
    Arc<DnsCache>,
    Arc<DnsRegistrar>,
    DnsResolver,
) {
    let pool_manager = Arc::new(DnsPoolManager::new());
    let validator = Arc::new(DnsValidator::new(false)); // Non-strict for testing
    let cache = Arc::new(DnsCache::new(1000));
    let registrar = Arc::new(DnsRegistrar::new(
        pool_manager.clone(),
        validator.clone(),
    ));
    let resolver = DnsResolver::new(
        pool_manager.clone(),
        validator.clone(),
        cache.clone(),
    );

    (pool_manager, validator, cache, registrar, resolver)
}

#[tokio::test]
async fn test_nike_scenario_mixed_public_and_federated() {
    // Nike scenario: Public storefront + federated admin/warehouse/supplier
    let (_pool_manager, _validator, _cache, registrar, resolver) = setup_dns_system().await;

    println!("\n==== Nike Scenario: Mixed Public/Federated ====");

    // 1. Register public storefront
    let nike_domain = Domain::parse("nike").unwrap();
    let nike_record = create_dns_record("nike", Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 1), "nike-node");
    let proof = create_test_proof();

    let registration = registrar
        .register_public(nike_domain.clone(), nike_record, proof.clone())
        .await
        .unwrap();
    println!("✅ Registered public: nike -> {}", registration.tx_hash.unwrap());

    // 2. Register federated admin
    let admin_domain = Domain::parse("admin.nike").unwrap();
    let admin_record = create_dns_record("admin.nike", Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 2), "nike-node");

    let registration = registrar
        .register_federated(
            admin_domain.clone(),
            "nike-internal".to_string(),
            admin_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: admin.nike (nike-internal) -> {}", registration.tx_hash.unwrap());

    // 3. Register federated warehouse
    let warehouse_domain = Domain::parse("warehouse.nike").unwrap();
    let warehouse_record = create_dns_record("warehouse.nike", Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 3), "nike-node");

    let registration = registrar
        .register_federated(
            warehouse_domain.clone(),
            "nike-internal".to_string(),
            warehouse_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: warehouse.nike (nike-internal) -> {}", registration.tx_hash.unwrap());

    // 4. Register federated supplier
    let supplier_domain = Domain::parse("supplier.nike").unwrap();
    let supplier_record = create_dns_record("supplier.nike", Ipv6Addr::new(0x2001, 0xdb8, 1, 0, 0, 0, 0, 4), "nike-node");

    let registration = registrar
        .register_federated(
            supplier_domain.clone(),
            "nike-supply-chain".to_string(),
            supplier_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: supplier.nike (nike-supply-chain) -> {}", registration.tx_hash.unwrap());

    // 5. Query public storefront (anyone can access)
    let query = DnsQuery {
        domain: nike_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: None,
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert_eq!(response.tier, DnsResolutionTier::Public);
    assert_eq!(response.records.len(), 1);
    println!("✅ Public query: nike -> {} records", response.records.len());

    // 6. Query federated admin (requires nike-internal network)
    let query = DnsQuery {
        domain: admin_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("nike-internal".to_string()),
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::Federated { .. }));
    assert_eq!(response.records.len(), 1);
    println!("✅ Federated query: admin.nike (nike-internal) -> {} records", response.records.len());

    // 7. Query federated supplier (requires nike-supply-chain network)
    let query = DnsQuery {
        domain: supplier_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("nike-supply-chain".to_string()),
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::Federated { .. }));
    assert_eq!(response.records.len(), 1);
    println!("✅ Federated query: supplier.nike (nike-supply-chain) -> {} records", response.records.len());

    println!("✅ Nike scenario complete: Mixed public/federated DNS\n");
}

#[tokio::test]
async fn test_bank_scenario_portal_and_private() {
    // Bank scenario: Public website + federated internal/swift/compliance
    let (_pool_manager, _validator, _cache, registrar, resolver) = setup_dns_system().await;

    println!("\n==== Bank Scenario: Portal + Private Networks ====");

    // 1. Register public website
    let bank_domain = Domain::parse("bank").unwrap();
    let bank_record = create_dns_record("bank", Ipv6Addr::new(0x2001, 0xdb8, 2, 0, 0, 0, 0, 1), "bank-node");
    let proof = create_test_proof();

    let registration = registrar
        .register_public(bank_domain.clone(), bank_record, proof.clone())
        .await
        .unwrap();
    println!("✅ Registered public: bank -> {}", registration.tx_hash.unwrap());

    // 2. Register federated internal
    let internal_domain = Domain::parse("internal.bank").unwrap();
    let internal_record = create_dns_record("internal.bank", Ipv6Addr::new(0x2001, 0xdb8, 2, 0, 0, 0, 0, 2), "bank-node");

    let registration = registrar
        .register_federated(
            internal_domain.clone(),
            "bank-employees".to_string(),
            internal_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: internal.bank (bank-employees) -> {}", registration.tx_hash.unwrap());

    // 3. Register federated SWIFT
    let swift_domain = Domain::parse("swift.bank").unwrap();
    let swift_record = create_dns_record("swift.bank", Ipv6Addr::new(0x2001, 0xdb8, 2, 0, 0, 0, 0, 3), "bank-node");

    let registration = registrar
        .register_federated(
            swift_domain.clone(),
            "bank-swift".to_string(),
            swift_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: swift.bank (bank-swift) -> {}", registration.tx_hash.unwrap());

    // 4. Register federated compliance
    let compliance_domain = Domain::parse("compliance.bank").unwrap();
    let compliance_record = create_dns_record("compliance.bank", Ipv6Addr::new(0x2001, 0xdb8, 2, 0, 0, 0, 0, 4), "bank-node");

    let registration = registrar
        .register_federated(
            compliance_domain.clone(),
            "bank-compliance".to_string(),
            compliance_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered federated: compliance.bank (bank-compliance) -> {}", registration.tx_hash.unwrap());

    // 5. Query public website
    let query = DnsQuery {
        domain: bank_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: None,
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert_eq!(response.tier, DnsResolutionTier::Public);
    println!("✅ Public query: bank -> {} records", response.records.len());

    // 6. Query federated internal (requires bank-employees network)
    let query = DnsQuery {
        domain: internal_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("bank-employees".to_string()),
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::Federated { .. }));
    println!("✅ Federated query: internal.bank (bank-employees) -> {} records", response.records.len());

    // 7. Query federated SWIFT (requires bank-swift network)
    let query = DnsQuery {
        domain: swift_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("bank-swift".to_string()),
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::Federated { .. }));
    println!("✅ Federated query: swift.bank (bank-swift) -> {} records", response.records.len());

    println!("✅ Bank scenario complete: Portal + private networks\n");
}

#[tokio::test]
async fn test_government_scenario_fully_federated() {
    // Government scenario: Fully federated (no public DNS records)
    let (_pool_manager, _validator, _cache, registrar, resolver) = setup_dns_system().await;

    println!("\n==== Government Scenario: Fully Federated (No Public) ====");

    let proof = create_test_proof();

    // 1. Register fully federated classified
    let classified_domain = Domain::parse("classified.internal.gov").unwrap();
    let classified_record = create_dns_record(
        "classified.internal.gov",
        Ipv6Addr::new(0x2001, 0xdb8, 3, 0, 0, 0, 0, 1),
        "gov-node",
    );

    let registration = registrar
        .register_federated(
            classified_domain.clone(),
            "gov-classified".to_string(),
            classified_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered fully federated: classified.internal.gov (gov-classified) -> {}", registration.tx_hash.unwrap());

    // 2. Register federated intel
    let intel_domain = Domain::parse("intel.internal.gov").unwrap();
    let intel_record = create_dns_record(
        "intel.internal.gov",
        Ipv6Addr::new(0x2001, 0xdb8, 3, 0, 0, 0, 0, 2),
        "gov-node",
    );

    let registration = registrar
        .register_federated(
            intel_domain.clone(),
            "gov-intelligence".to_string(),
            intel_record,
            proof.clone(),
        )
        .await
        .unwrap();
    println!("✅ Registered fully federated: intel.internal.gov (gov-intelligence) -> {}", registration.tx_hash.unwrap());

    // 3. Attempt to query without network membership (should fail or require proof)
    let query = DnsQuery {
        domain: classified_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: None,
        proof: None,
        timestamp: SystemTime::now(),
    };

    // Should fail due to lack of network membership
    let result = resolver.resolve(query).await;
    assert!(result.is_err(), "Should fail without network membership");
    println!("✅ Access denied: classified.internal.gov (no network membership)");

    // 4. Query with proper network membership
    let query = DnsQuery {
        domain: classified_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("gov-classified".to_string()),
        proof: Some(proof.clone()),
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::FullyFederated { .. }));
    println!("✅ Fully federated query: classified.internal.gov (gov-classified) -> {} records", response.records.len());

    // 5. Query intel with proper network
    let query = DnsQuery {
        domain: intel_domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: Some("gov-intelligence".to_string()),
        proof: Some(proof.clone()),
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert!(matches!(response.tier, DnsResolutionTier::FullyFederated { .. }));
    println!("✅ Fully federated query: intel.internal.gov (gov-intelligence) -> {} records", response.records.len());

    println!("✅ Government scenario complete: Fully federated DNS\n");
}

#[tokio::test]
async fn test_privacy_boundary_enforcement() {
    // Test privacy boundaries between public and federated pools
    let (pool_manager, validator, cache, _registrar, _resolver) = setup_dns_system().await;

    println!("\n==== Privacy Boundary Enforcement ====");

    // Setup: Register records in different pools
    let public_record = create_dns_record("public-domain", Ipv6Addr::LOCALHOST, "node-1");
    pool_manager.register_public(public_record).await.unwrap();

    let federated_record = create_dns_record("private-domain", Ipv6Addr::LOCALHOST, "node-2");
    pool_manager
        .register_federated("private-network".to_string(), federated_record)
        .await
        .unwrap();

    // Test 1: Public can query public pool
    assert!(
        pool_manager
            .can_access(None, &DnsPoolType::Public)
            .await
    );
    println!("✅ Public pool accessible without network membership");

    // Test 2: Federated requires matching network
    let federated_type = DnsPoolType::Federated {
        network_id: "private-network".to_string(),
    };
    assert!(
        pool_manager
            .can_access(Some("private-network"), &federated_type)
            .await
    );
    println!("✅ Federated pool accessible with matching network");

    // Test 3: Federated denies wrong network
    assert!(
        !pool_manager
            .can_access(Some("wrong-network"), &federated_type)
            .await
    );
    println!("✅ Federated pool denies wrong network");

    // Test 4: Federated denies no network
    assert!(
        !pool_manager
            .can_access(None, &federated_type)
            .await
    );
    println!("✅ Federated pool denies no network membership");

    println!("✅ Privacy boundary enforcement complete\n");
}

#[tokio::test]
async fn test_dns_cache_performance() {
    // Test DNS caching with TTL
    let (_pool_manager, _validator, _cache, registrar, resolver) = setup_dns_system().await;

    println!("\n==== DNS Cache Performance ====");

    // Register domain
    let domain = Domain::parse("cached-domain").unwrap();
    let record = create_dns_record("cached-domain", Ipv6Addr::LOCALHOST, "node-1");
    let proof = create_test_proof();

    registrar
        .register_public(domain.clone(), record, proof)
        .await
        .unwrap();

    // First query (cache miss)
    let query = DnsQuery {
        domain: domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: None,
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response1 = resolver.resolve(query.clone()).await.unwrap();
    assert!(!response1.from_cache);
    println!("✅ First query: cache miss");

    // Second query (cache hit)
    let response2 = resolver.resolve(query.clone()).await.unwrap();
    assert!(response2.from_cache);
    println!("✅ Second query: cache hit");

    // Verify same data
    assert_eq!(response1.records.len(), response2.records.len());
    println!("✅ Cache data integrity verified");

    println!("✅ DNS cache performance test complete\n");
}

#[tokio::test]
async fn test_p2p_direct_bypass() {
    // Test P2P direct connection bypass (no DNS)
    let (_pool_manager, _validator, _cache, _registrar, resolver) = setup_dns_system().await;

    println!("\n==== P2P Direct Bypass Test ====");

    // Query P2P peer ID
    let domain = Domain::parse("peer-12345abcde").unwrap();
    let query = DnsQuery {
        domain: domain.clone(),
        record_type: DnsRecordType::AAAA,
        requester_network: None,
        proof: None,
        timestamp: SystemTime::now(),
    };

    let response = resolver.resolve(query).await.unwrap();
    assert_eq!(response.tier, DnsResolutionTier::P2PDirect);
    assert_eq!(response.records.len(), 0); // P2P returns empty, app handles direct connection
    println!("✅ P2P direct: no DNS resolution (app handles connection)");

    println!("✅ P2P direct bypass test complete\n");
}

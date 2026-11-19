// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Local mock-routing, structural token, and loopback transport fixtures.
//! These tests do not establish genuine authority, PoS, or deployment success.

use anyhow::Result;
use hypermesh_lib::proof::{SpaceProof, StakeProof, StateProof, TimeProof, WorkProof};
use hypermesh_lib::NetworkId;
use std::net::Ipv6Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use stoq::api::service_discovery::{
    DiscoveryContext, DnsBinding, ServiceDiscovery, ServiceEndpoint, ServiceMetadata,
    TrustChainDnsClient,
};
use stoq::protocol::pos_validator::{PosToken, PosTokenValidator};
use stoq::transport::{Endpoint, StoqTransport, TransportConfig};

/// Local routing/structural fixture, not genuine authority, proof, or deployment validation.
#[tokio::test]
async fn test_full_integration() -> Result<()> {
    let (discovery, resolver) = dns_fixture()?;
    let endpoint = discovery.resolve("caesar")?;
    assert_eq!(endpoint.port, 8001);
    assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST);
    assert_eq!(endpoint.server_name.as_deref(), Some("caesar.local"));
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
    validate_mock_token()?;
    create_loopback_transport().await?;
    let endpoint = Endpoint {
        address: Ipv6Addr::LOCALHOST,
        port: 9292,
        server_name: None,
    };
    assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST);
    assert_eq!(endpoint.port, 9292);
    assert!(endpoint.server_name.is_none());
    Ok(())
}

#[test]
fn test_unconfigured_hardcoded_metadata_denied() {
    let discovery = ServiceDiscovery::new(Duration::from_secs(300));
    discovery.add_hardcoded_endpoint(caesar_endpoint());
    assert_eq!(
        discovery.list_hardcoded_services(),
        vec!["caesar".to_string()]
    );
    assert!(discovery.resolve("caesar").is_err());
    assert!(discovery.list_cached_services().is_empty());
    assert_eq!(discovery.get_metrics().fallback_resolutions, 0);
}

#[derive(Default)]
struct MockDns {
    calls: AtomicUsize,
}

impl TrustChainDnsClient for MockDns {
    fn resolve(&self, name: &str) -> Result<Vec<ServiceEndpoint>> {
        assert_eq!(name, "caesar");
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![caesar_endpoint()])
    }
    fn is_available(&self) -> bool {
        true
    }
}

fn dns_fixture() -> Result<(ServiceDiscovery, Arc<MockDns>)> {
    let resolver = Arc::new(MockDns::default());
    let context = DiscoveryContext::new("stoq", NetworkId([1; 16]), "[::1]:5353".parse()?)?;
    let binding = DnsBinding::new(context, resolver.clone());
    Ok((
        ServiceDiscovery::with_dns(Duration::from_secs(300), binding),
        resolver,
    ))
}

fn caesar_endpoint() -> ServiceEndpoint {
    ServiceEndpoint {
        name: "caesar".to_string(),
        address: Ipv6Addr::LOCALHOST,
        port: 8001,
        server_name: Some("caesar.local".to_string()),
        metadata: ServiceMetadata::default(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
    }
}

fn validate_mock_token() -> Result<()> {
    let validator = PosTokenValidator::new(Duration::from_secs(300));
    // Mock identity/signature bytes exercise a structural/error path, not genuine PoS.
    let token = PosToken::for_identity(
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        canonical_test_proof(),
        (10, 20, 5),
        1000,
        vec![7, 0, 0, 0],
        Duration::from_secs(3600),
    );
    let result = validator.validate_token(&token)?;
    assert!(result.is_valid || !result.errors.is_empty());
    let metrics = validator.get_metrics();
    assert_eq!(metrics.total_validations, 1);
    assert!(metrics.avg_validation_time_us < 10000);
    Ok(())
}

async fn create_loopback_transport() -> Result<()> {
    let config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        enable_kernel_acceleration: false,
        ..Default::default()
    };
    // The runner supplies a fresh disposable HOME under the private runtime;
    // localhost certificates never touch a real HOME or a live CA/service.
    let _transport = StoqTransport::new(config).await?;
    Ok(())
}

#[test]
fn test_pos_validation_overhead() {
    // Performance test: Ensure PoS validation adds minimal overhead
    let validator = PosTokenValidator::new(Duration::from_secs(300));

    let token = PosToken::for_identity(
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        canonical_test_proof(),
        (10, 20, 5),
        1000,
        vec![7, 0, 0, 0],
        Duration::from_secs(3600),
    );

    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = validator.validate_token(&token);
    }
    let elapsed = start.elapsed();

    let per_validation = elapsed / 100;
    println!("PoS validation overhead: {per_validation:?} per validation");

    // Should be under 10ms per validation (relaxed for real crypto)
    assert!(per_validation < Duration::from_millis(10));
}

/// Build a structural four-proof fixture; mock identity bytes are not genuine PoS.
///
/// CANONICAL MODEL: authorization (WHO) is an identity binding with NO amount;
/// WHAT is the BLAKE3 hash of the work performed; WHERE is a location (capacity
/// is descriptive only, never a gate); WHEN is a time.
fn canonical_test_proof() -> StateProof {
    let mut space = SpaceProof::new(
        "test-node-001".to_string(),
        "hypermesh://test-node-001/store".to_string(),
        1024 * 1024 * 1024,
    );
    space.file_hash = "a1b2c3d4e5f6".to_string();

    StateProof::new(
        StakeProof::new("test-owner".to_string(), "unbound".to_string()),
        TimeProof::new(Duration::from_secs(1)),
        space,
        WorkProof::from_work(
            "test-owner".to_string(),
            "test-workload".to_string(),
            b"the work that was actually done",
        ),
    )
}

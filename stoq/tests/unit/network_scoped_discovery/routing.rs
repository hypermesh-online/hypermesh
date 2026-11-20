use super::fixtures::*;
use super::*;
use std::net::{SocketAddr, SocketAddrV6};
use std::time::Instant;

#[test]
fn supplied_resolvers_and_network_caches_are_isolated() {
    let left = Arc::new(MockDns::new(vec![endpoint("fd00::1", 9295)]));
    let right = Arc::new(MockDns::new(vec![endpoint("fd00::2", 9395)]));
    let a = ServiceDiscovery::with_dns(Duration::from_secs(300), binding(1, left.clone()));
    let b = ServiceDiscovery::with_dns(Duration::from_secs(300), binding(2, right.clone()));
    for _ in 0..2 {
        assert_eq!(a.resolve("catalog").unwrap().port, 9295);
        assert_eq!(b.resolve("catalog").unwrap().port, 9395);
    }
    assert_eq!(left.calls.load(Ordering::SeqCst), 1);
    assert_eq!(right.calls.load(Ordering::SeqCst), 1);
    assert_eq!(a.get_metrics().cache_hits, 1);
    assert_eq!(b.get_metrics().fallback_resolutions, 0);
}

#[test]
fn missing_empty_unavailable_and_errors_never_fallback() {
    let mock = Arc::new(MockDns::new(vec![]));
    let discovery = configured(mock.clone());
    discovery.add_hardcoded_endpoint(endpoint("::1", 9295));
    assert!(discovery.resolve("catalog").is_err());
    mock.available.store(false, Ordering::SeqCst);
    assert!(discovery.resolve("catalog").is_err());
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    struct Failing;
    impl TrustChainDnsClient for Failing {
        fn resolve(&self, _: &str) -> Result<Vec<ServiceEndpoint>> {
            Err(anyhow!("unavailable"))
        }
        fn is_available(&self) -> bool {
            true
        }
    }
    let failure = ServiceDiscovery::with_dns(
        Duration::from_secs(1),
        DnsBinding::new(context(1), Arc::new(Failing)),
    );
    assert!(failure.resolve("catalog").is_err());
    assert_eq!(discovery.get_metrics().fallback_resolutions, 0);
}

#[test]
fn context_validates_native_ipv6_without_network_id_mapping() {
    let network = hypermesh_lib::NetworkId([0; 16]);
    for address in ["127.0.0.1:53", "[::ffff:127.0.0.1]:53", "[::1]:0"] {
        assert!(DiscoveryContext::new("stoq", network, address.parse().unwrap()).is_err());
    }
    let scoped = SocketAddr::V6(SocketAddrV6::new("fe80::1".parse().unwrap(), 53, 0, 4));
    assert!(DiscoveryContext::new("stoq", network, scoped).is_err());
    assert!(DiscoveryContext::new("  ", network, "[::1]:53".parse().unwrap()).is_err());
    for address in ["[::1]:53", "[fd00::1]:53", "[2001:db8::1]:53"] {
        let context = DiscoveryContext::new("stoq", network, address.parse().unwrap()).unwrap();
        assert_eq!(context.network_id(), &network);
        assert_eq!(context.authority().to_string(), address);
        assert_eq!(context.protocol(), "stoq");
    }
}

#[test]
fn mapped_zero_port_and_expired_endpoints_are_denied() {
    let mut expired = endpoint("::1", 9295);
    expired.expires_at = SystemTime::UNIX_EPOCH;
    for record in [
        endpoint("::ffff:127.0.0.1", 9295),
        endpoint("::1", 0),
        expired,
    ] {
        let discovery = configured(Arc::new(MockDns::new(vec![record])));
        assert!(discovery.resolve("catalog").is_err());
        assert!(discovery.list_cached_services().is_empty());
    }
    for address in ["::1", "fd00::1", "2001:db8::1"] {
        let discovery = configured(Arc::new(MockDns::new(vec![endpoint(address, 9295)])));
        assert_eq!(
            discovery.resolve("catalog").unwrap().address.to_string(),
            address
        );
    }
}

#[test]
fn binding_cannot_replace_or_follow_first_resolution() {
    let mock = Arc::new(MockDns::new(vec![endpoint("::1", 9295)]));
    let discovery = ServiceDiscovery::new(Duration::from_secs(300));
    discovery.set_dns_binding(binding(1, mock.clone())).unwrap();
    assert!(discovery.set_dns_binding(binding(2, mock.clone())).is_err());
    discovery.resolve("catalog").unwrap();
    assert!(discovery.set_dns_binding(binding(1, mock.clone())).is_err());
    let mut unconfigured = ServiceDiscovery::new(Duration::from_secs(300));
    assert!(unconfigured.set_trustchain_dns(mock.clone()).is_err());
    assert!(unconfigured.resolve("catalog").is_err());
    assert!(unconfigured.set_dns_binding(binding(1, mock)).is_err());
}

#[test]
fn cache_expiry_is_minimum_of_ttl_and_record_expiry() {
    for (ttl, horizon) in [(300, 61), (1, 2)] {
        let mock = Arc::new(MockDns::new(vec![endpoint("::1", 9295)]));
        let discovery =
            ServiceDiscovery::with_dns(Duration::from_secs(ttl), binding(1, mock.clone()));
        discovery.resolve("catalog").unwrap();
        let future = Duration::from_secs(horizon);
        assert!(discovery
            .state
            .lock()
            .cached_at(
                "catalog",
                Instant::now() + future,
                SystemTime::now() + future
            )
            .is_none());
        discovery.resolve("catalog").unwrap();
        assert_eq!(mock.calls.load(Ordering::SeqCst), 2);
    }
    let zero = ServiceDiscovery::with_dns(
        Duration::ZERO,
        binding(1, Arc::new(MockDns::new(vec![endpoint("::1", 9295)]))),
    );
    assert!(zero.resolve("catalog").is_err());
}

#[test]
fn registration_is_metadata_not_dns_authority() {
    let discovery = ServiceDiscovery::new(Duration::from_secs(300));
    discovery
        .register_service(
            ServiceType::Catalog,
            Ipv6Addr::LOCALHOST,
            9295,
            ServiceMetadata::default(),
        )
        .unwrap();
    assert_eq!(
        discovery.registered_service_types(),
        vec![ServiceType::Catalog]
    );
    assert_eq!(
        discovery.list_hardcoded_services(),
        vec!["catalog".to_owned()]
    );
    assert!(discovery.resolve("catalog").is_err());
    discovery.unregister_service("catalog");
    assert!(discovery.registered_service_types().is_empty());
}

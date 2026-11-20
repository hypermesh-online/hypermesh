use super::*;
#[path = "network_scoped_discovery/fixtures.rs"]
mod fixtures;
#[path = "network_scoped_discovery/invalidation.rs"]
mod invalidation;
#[path = "network_scoped_discovery/routing.rs"]
mod routing;

#[test]
fn unconfigured_catalog_denies() {
    let discovery = ServiceDiscovery::new(Duration::from_secs(300));
    assert!(
        discovery.resolve("catalog").is_err(),
        "unconfigured discovery must deny catalog"
    );
}

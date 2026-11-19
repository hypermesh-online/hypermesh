use super::fixtures::*;
use super::*;
use std::sync::Barrier;

#[test]
fn mutation_cannot_publish_stale_inflight_resolution() {
    for mutation in 0..3 {
        stale_after_mutation(mutation);
    }
}

fn stale_after_mutation(mutation: u8) {
    let entered = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let (callback_entered, callback_resume) = (entered.clone(), resume.clone());
    let mut mock = MockDns::new(vec![endpoint("::1", 9295)]);
    mock.hook = Some(Arc::new(move || {
        callback_entered.wait();
        callback_resume.wait();
    }));
    let discovery = Arc::new(configured(Arc::new(mock)));
    let resolving = discovery.clone();
    let worker = std::thread::spawn(move || resolving.resolve("catalog"));
    entered.wait();
    match mutation {
        0 => discovery.clear_cache(),
        1 => discovery.add_hardcoded_endpoint(endpoint("fd00::2", 9395)),
        _ => discovery.unregister_service("catalog"),
    }
    resume.wait();
    assert!(worker.join().unwrap().is_err());
    assert!(discovery.list_cached_services().is_empty());
    assert_eq!(discovery.get_metrics().dns_resolutions, 0);
}

#[test]
fn callbacks_can_reenter_discovery_without_any_guard() {
    let holder = Arc::new(Mutex::new(std::sync::Weak::<ServiceDiscovery>::new()));
    let callback_holder = holder.clone();
    let mut mock = MockDns::new(vec![endpoint("::1", 9295)]);
    mock.hook = Some(Arc::new(move || {
        let discovery = callback_holder.lock().upgrade().unwrap();
        discovery.clear_cache();
        assert_eq!(discovery.get_metrics().cache_size, 0);
    }));
    let discovery = Arc::new(configured(Arc::new(mock)));
    *holder.lock() = Arc::downgrade(&discovery);
    assert!(discovery.resolve("catalog").is_err());
    assert!(discovery.list_cached_services().is_empty());
}

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize};

pub(super) struct MockDns {
    pub endpoints: Mutex<Vec<ServiceEndpoint>>,
    pub calls: AtomicUsize,
    pub available: AtomicBool,
    pub hook: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl MockDns {
    pub fn new(endpoints: Vec<ServiceEndpoint>) -> Self {
        Self {
            endpoints: Mutex::new(endpoints),
            calls: AtomicUsize::new(0),
            available: AtomicBool::new(true),
            hook: None,
        }
    }
}

impl TrustChainDnsClient for MockDns {
    fn resolve(&self, _: &str) -> Result<Vec<ServiceEndpoint>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(hook) = &self.hook {
            hook();
        }
        Ok(self.endpoints.lock().clone())
    }
    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }
}

pub(super) fn endpoint(address: &str, port: u16) -> ServiceEndpoint {
    ServiceEndpoint {
        name: "catalog".to_owned(),
        address: address.parse().unwrap(),
        port,
        server_name: None,
        metadata: ServiceMetadata::default(),
        expires_at: SystemTime::now() + Duration::from_secs(60),
    }
}

pub(super) fn context(network: u8) -> DiscoveryContext {
    DiscoveryContext::new(
        "stoq",
        hypermesh_lib::NetworkId([network; 16]),
        "[::1]:5353".parse().unwrap(),
    )
    .unwrap()
}

pub(super) fn binding(network: u8, client: Arc<MockDns>) -> DnsBinding {
    DnsBinding::new(context(network), client)
}

pub(super) fn configured(client: Arc<MockDns>) -> ServiceDiscovery {
    ServiceDiscovery::with_dns(Duration::from_secs(300), binding(1, client))
}

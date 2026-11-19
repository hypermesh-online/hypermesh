use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub(super) struct DiscoveryMetrics {
    pub total_resolutions: AtomicU64,
    pub cache_hits: AtomicU64,
    pub dns_resolutions: AtomicU64,
    pub failed_resolutions: AtomicU64,
}

/// Discovery counts; fallback_resolutions remains zero (no fallback exists).
#[derive(Debug)]
pub struct DiscoveryStats {
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub dns_resolutions: u64,
    pub fallback_resolutions: u64,
    pub failed_resolutions: u64,
    pub cache_size: usize,
}

impl DiscoveryMetrics {
    pub fn snapshot(&self, cache_size: usize) -> DiscoveryStats {
        DiscoveryStats {
            total_resolutions: self.total_resolutions.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            dns_resolutions: self.dns_resolutions.load(Ordering::Relaxed),
            fallback_resolutions: 0,
            failed_resolutions: self.failed_resolutions.load(Ordering::Relaxed),
            cache_size,
        }
    }
}

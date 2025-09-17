//! Real-time Certificate Fingerprint Tracker
//! 
//! Tracks certificate fingerprints for real-time monitoring and
//! duplicate detection with efficient in-memory and persistent storage.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, Instant};
use tracing::{debug, info, warn, error};
use sha2::{Sha256, Digest};

use crate::errors::{CTError, Result as TrustChainResult};

/// Real-time fingerprint tracker
pub struct FingerprintTracker {
    /// Enable real-time tracking
    enabled: bool,
    /// Fingerprint cache (fingerprint -> metadata)
    fingerprint_cache: Arc<DashMap<[u8; 32], FingerprintMetadata>>,
    /// Domain tracking (domain -> fingerprints)
    domain_tracking: Arc<DashMap<String, Vec<[u8; 32]>>>,
    /// Recent fingerprints queue (for duplicate detection)
    recent_fingerprints: Arc<RwLock<VecDeque<TimestampedFingerprint>>>,
    /// Statistics
    stats: Arc<RwLock<FingerprintStats>>,
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// Fingerprint metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintMetadata {
    /// Certificate fingerprint
    pub fingerprint: [u8; 32],
    /// Common name from certificate
    pub common_name: String,
    /// First seen timestamp
    pub first_seen: SystemTime,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Number of times seen
    pub seen_count: u64,
    /// Associated domains
    pub domains: Vec<String>,
    /// Fingerprint status
    pub status: FingerprintStatus,
}

/// Fingerprint status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FingerprintStatus {
    /// Normal certificate
    Normal,
    /// Duplicate fingerprint detected
    Duplicate,
    /// Suspicious activity detected
    Suspicious { reason: String },
    /// Certificate revoked
    Revoked { reason: String },
}

/// Timestamped fingerprint for recent tracking
#[derive(Clone, Debug)]
struct TimestampedFingerprint {
    fingerprint: [u8; 32],
    timestamp: Instant,
    common_name: String,
}

/// Fingerprint tracker statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintStats {
    /// Total fingerprints tracked
    pub total_tracked: u64,
    /// Unique domains tracked
    pub unique_domains: u64,
    /// Duplicate detections
    pub duplicate_count: u64,
    /// Suspicious activity count
    pub suspicious_count: u64,
    /// Last cleanup timestamp
    pub last_cleanup: SystemTime,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Memory usage (approximate bytes)
    pub memory_usage_bytes: u64,
}

impl Default for FingerprintStats {
    fn default() -> Self {
        Self {
            total_tracked: 0,
            unique_domains: 0,
            duplicate_count: 0,
            suspicious_count: 0,
            last_cleanup: SystemTime::now(),
            cache_hit_rate: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

/// Fingerprint tracking configuration
#[derive(Clone, Debug)]
pub struct FingerprintConfig {
    /// Maximum fingerprints to keep in recent queue
    pub max_recent_fingerprints: usize,
    /// Recent fingerprint retention duration
    pub recent_retention: Duration,
    /// Duplicate detection window
    pub duplicate_detection_window: Duration,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Suspicious activity threshold (fingerprints per domain per hour)
    pub suspicious_threshold: u64,
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            max_recent_fingerprints: 10000,
            recent_retention: Duration::from_secs(3600), // 1 hour
            duplicate_detection_window: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(1800), // 30 minutes
            suspicious_threshold: 100, // 100 certs per domain per hour
        }
    }
}

impl FingerprintTracker {
    /// Create new fingerprint tracker
    pub async fn new(enabled: bool) -> TrustChainResult<Self> {
        info!("Initializing fingerprint tracker (enabled: {})", enabled);

        let tracker = Self {
            enabled,
            fingerprint_cache: Arc::new(DashMap::new()),
            domain_tracking: Arc::new(DashMap::new()),
            recent_fingerprints: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(FingerprintStats::default())),
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        if enabled {
            tracker.start_background_tasks().await?;
        }

        info!("Fingerprint tracker initialized successfully");
        Ok(tracker)
    }

    /// Track a certificate fingerprint
    pub async fn track_certificate(
        &self,
        fingerprint: [u8; 32],
        common_name: String,
        timestamp: SystemTime,
    ) -> TrustChainResult<FingerprintStatus> {
        if !self.enabled {
            return Ok(FingerprintStatus::Normal);
        }

        debug!("Tracking certificate fingerprint: {}", hex::encode(fingerprint));

        // Check for duplicates in recent fingerprints
        let is_duplicate = self.check_for_duplicate(&fingerprint).await?;
        
        // Extract domain from common name
        let domain = self.extract_domain(&common_name);

        // Update or create fingerprint metadata
        let status = if let Some(mut metadata) = self.fingerprint_cache.get_mut(&fingerprint) {
            // Update existing fingerprint
            metadata.last_seen = timestamp;
            metadata.seen_count += 1;
            
            if !metadata.domains.contains(&domain) {
                metadata.domains.push(domain.clone());
            }

            if is_duplicate && metadata.seen_count > 1 {
                metadata.status = FingerprintStatus::Duplicate;
            }

            metadata.status.clone()
        } else {
            // New fingerprint
            let status = if is_duplicate {
                FingerprintStatus::Duplicate
            } else {
                FingerprintStatus::Normal
            };

            let metadata = FingerprintMetadata {
                fingerprint,
                common_name: common_name.clone(),
                first_seen: timestamp,
                last_seen: timestamp,
                seen_count: 1,
                domains: vec![domain.clone()],
                status: status.clone(),
            };

            self.fingerprint_cache.insert(fingerprint, metadata);
            status
        };

        // Update domain tracking
        self.domain_tracking.entry(domain.clone())
            .or_insert_with(Vec::new)
            .push(fingerprint);

        // Add to recent fingerprints
        self.add_to_recent_fingerprints(fingerprint, common_name).await;

        // Check for suspicious activity
        let suspicious_status = self.check_suspicious_activity(&domain).await?;
        if let FingerprintStatus::Suspicious { .. } = suspicious_status {
            // Update fingerprint status to suspicious
            if let Some(mut metadata) = self.fingerprint_cache.get_mut(&fingerprint) {
                metadata.status = suspicious_status.clone();
            }
            return Ok(suspicious_status);
        }

        // Update statistics
        self.update_stats().await;

        Ok(status)
    }

    /// Get fingerprint metadata
    pub async fn get_fingerprint_metadata(&self, fingerprint: &[u8; 32]) -> Option<FingerprintMetadata> {
        self.fingerprint_cache.get(fingerprint).map(|entry| entry.clone())
    }

    /// Get domain fingerprints
    pub async fn get_domain_fingerprints(&self, domain: &str) -> Vec<[u8; 32]> {
        self.domain_tracking.get(domain)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    /// Get tracker statistics
    pub async fn get_stats(&self) -> FingerprintStats {
        self.stats.read().await.clone()
    }

    /// Check if fingerprint is suspicious
    pub async fn is_suspicious(&self, fingerprint: &[u8; 32]) -> bool {
        self.fingerprint_cache.get(fingerprint)
            .map(|metadata| matches!(metadata.status, FingerprintStatus::Suspicious { .. }))
            .unwrap_or(false)
    }

    /// Mark fingerprint as revoked
    pub async fn mark_revoked(&self, fingerprint: &[u8; 32], reason: String) -> TrustChainResult<()> {
        if let Some(mut metadata) = self.fingerprint_cache.get_mut(fingerprint) {
            metadata.status = FingerprintStatus::Revoked { reason };
            info!("Marked fingerprint as revoked: {}", hex::encode(fingerprint));
        } else {
            warn!("Attempted to mark unknown fingerprint as revoked: {}", hex::encode(fingerprint));
        }
        Ok(())
    }

    /// Clean up old fingerprints
    pub async fn cleanup(&self) -> TrustChainResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Starting fingerprint tracker cleanup");

        let config = FingerprintConfig::default();
        let cutoff_time = Instant::now() - config.recent_retention;

        // Clean up recent fingerprints
        {
            let mut recent = self.recent_fingerprints.write().await;
            while let Some(front) = recent.front() {
                if front.timestamp > cutoff_time {
                    break;
                }
                recent.pop_front();
            }
        }

        // Clean up domain tracking (remove empty domains)
        let mut empty_domains = Vec::new();
        for item in self.domain_tracking.iter() {
            if item.value().is_empty() {
                empty_domains.push(item.key().clone());
            }
        }

        for domain in empty_domains {
            self.domain_tracking.remove(&domain);
        }

        // Update statistics
        self.update_stats().await;

        debug!("Fingerprint tracker cleanup completed");
        Ok(())
    }

    /// Shutdown tracker
    pub async fn shutdown(&self) -> TrustChainResult<()> {
        if self.enabled {
            info!("Shutting down fingerprint tracker");

            // Cancel background tasks
            let mut handles = self.task_handles.lock().await;
            for handle in handles.drain(..) {
                handle.abort();
            }

            info!("Fingerprint tracker shutdown completed");
        }
        Ok(())
    }

    // Internal helper methods

    async fn start_background_tasks(&self) -> TrustChainResult<()> {
        let mut handles = self.task_handles.lock().await;
        let config = FingerprintConfig::default();

        // Cleanup task
        let tracker_clone = self.clone_for_task();
        let cleanup_interval = config.cleanup_interval;
        let handle = tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            loop {
                interval.tick().await;
                if let Err(e) = tracker_clone.cleanup().await {
                    error!("Fingerprint tracker cleanup failed: {}", e);
                }
            }
        });
        handles.push(handle);

        info!("Fingerprint tracker background tasks started");
        Ok(())
    }

    fn clone_for_task(&self) -> Self {
        Self {
            enabled: self.enabled,
            fingerprint_cache: Arc::clone(&self.fingerprint_cache),
            domain_tracking: Arc::clone(&self.domain_tracking),
            recent_fingerprints: Arc::clone(&self.recent_fingerprints),
            stats: Arc::clone(&self.stats),
            task_handles: Arc::clone(&self.task_handles),
        }
    }

    async fn check_for_duplicate(&self, fingerprint: &[u8; 32]) -> TrustChainResult<bool> {
        let config = FingerprintConfig::default();
        let cutoff_time = Instant::now() - config.duplicate_detection_window;

        let recent = self.recent_fingerprints.read().await;
        let is_duplicate = recent.iter()
            .filter(|fp| fp.timestamp > cutoff_time)
            .any(|fp| fp.fingerprint == *fingerprint);

        Ok(is_duplicate)
    }

    async fn add_to_recent_fingerprints(&self, fingerprint: [u8; 32], common_name: String) {
        let config = FingerprintConfig::default();
        let mut recent = self.recent_fingerprints.write().await;

        // Add new fingerprint
        recent.push_back(TimestampedFingerprint {
            fingerprint,
            timestamp: Instant::now(),
            common_name,
        });

        // Maintain size limit
        while recent.len() > config.max_recent_fingerprints {
            recent.pop_front();
        }
    }

    async fn check_suspicious_activity(&self, domain: &str) -> TrustChainResult<FingerprintStatus> {
        let config = FingerprintConfig::default();
        let one_hour_ago = Instant::now() - Duration::from_secs(3600);

        // Count recent certificates for this domain
        let recent = self.recent_fingerprints.read().await;
        let recent_count = recent.iter()
            .filter(|fp| fp.timestamp > one_hour_ago)
            .filter(|fp| self.extract_domain(&fp.common_name) == domain)
            .count() as u64;

        if recent_count > config.suspicious_threshold {
            warn!("Suspicious activity detected for domain {}: {} certificates in 1 hour", domain, recent_count);
            Ok(FingerprintStatus::Suspicious {
                reason: format!("High certificate issuance rate: {} certs/hour", recent_count),
            })
        } else {
            Ok(FingerprintStatus::Normal)
        }
    }

    fn extract_domain(&self, common_name: &str) -> String {
        // Simple domain extraction - in production, this would be more sophisticated
        if let Some(domain) = common_name.strip_prefix("*.") {
            domain.to_string()
        } else {
            common_name.to_string()
        }
    }

    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;
        
        stats.total_tracked = self.fingerprint_cache.len() as u64;
        stats.unique_domains = self.domain_tracking.len() as u64;
        
        // Count duplicates and suspicious
        let mut duplicate_count = 0;
        let mut suspicious_count = 0;
        
        for item in self.fingerprint_cache.iter() {
            match &item.status {
                FingerprintStatus::Duplicate => duplicate_count += 1,
                FingerprintStatus::Suspicious { .. } => suspicious_count += 1,
                _ => {}
            }
        }
        
        stats.duplicate_count = duplicate_count;
        stats.suspicious_count = suspicious_count;
        stats.last_cleanup = SystemTime::now();
        
        // Estimate memory usage
        stats.memory_usage_bytes = self.estimate_memory_usage();
    }

    fn estimate_memory_usage(&self) -> u64 {
        let fingerprint_count = self.fingerprint_cache.len() as u64;
        let domain_count = self.domain_tracking.len() as u64;
        
        // Rough estimate: 200 bytes per fingerprint + 100 bytes per domain
        fingerprint_count * 200 + domain_count * 100
    }
}

/// Fingerprint query parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintQuery {
    pub domain: Option<String>,
    pub status: Option<FingerprintStatus>,
    pub since: Option<SystemTime>,
    pub limit: Option<u32>,
}

/// Fingerprint search results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintSearchResult {
    pub fingerprints: Vec<FingerprintMetadata>,
    pub total_count: u64,
    pub has_more: bool,
}

impl FingerprintTracker {
    /// Search fingerprints with query parameters
    pub async fn search_fingerprints(&self, query: &FingerprintQuery) -> TrustChainResult<FingerprintSearchResult> {
        let mut matching_fingerprints = Vec::new();
        let limit = query.limit.unwrap_or(100) as usize;

        for item in self.fingerprint_cache.iter() {
            let metadata = item.value();
            
            // Apply filters
            if let Some(ref domain_filter) = query.domain {
                if !metadata.domains.iter().any(|d| d.contains(domain_filter)) {
                    continue;
                }
            }

            if let Some(ref status_filter) = query.status {
                if !std::mem::discriminant(&metadata.status)
                    .eq(&std::mem::discriminant(status_filter)) {
                    continue;
                }
            }

            if let Some(since) = query.since {
                if metadata.last_seen < since {
                    continue;
                }
            }

            matching_fingerprints.push(metadata.clone());

            if matching_fingerprints.len() >= limit {
                break;
            }
        }

        let total_count = matching_fingerprints.len() as u64;
        let has_more = self.fingerprint_cache.len() > matching_fingerprints.len();

        Ok(FingerprintSearchResult {
            fingerprints: matching_fingerprints,
            total_count,
            has_more,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_fingerprint_tracker_creation() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        let stats = tracker.get_stats().await;
        assert_eq!(stats.total_tracked, 0);
    }

    #[tokio::test]
    async fn test_certificate_tracking() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        let fingerprint = [1u8; 32];
        let common_name = "test.example.com".to_string();
        let timestamp = SystemTime::now();

        let status = tracker.track_certificate(fingerprint, common_name, timestamp).await.unwrap();
        assert!(matches!(status, FingerprintStatus::Normal));

        let metadata = tracker.get_fingerprint_metadata(&fingerprint).await.unwrap();
        assert_eq!(metadata.common_name, "test.example.com");
        assert_eq!(metadata.seen_count, 1);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        let fingerprint = [2u8; 32];
        let common_name = "test.example.com".to_string();
        let timestamp = SystemTime::now();

        // First tracking should be normal
        let status1 = tracker.track_certificate(fingerprint, common_name.clone(), timestamp).await.unwrap();
        assert!(matches!(status1, FingerprintStatus::Normal));

        // Immediate second tracking should detect duplicate
        let status2 = tracker.track_certificate(fingerprint, common_name, timestamp).await.unwrap();
        assert!(matches!(status2, FingerprintStatus::Duplicate));
    }

    #[tokio::test]
    async fn test_domain_fingerprints() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        let domain = "example.com";
        let fingerprint1 = [3u8; 32];
        let fingerprint2 = [4u8; 32];

        tracker.track_certificate(fingerprint1, format!("test1.{}", domain), SystemTime::now()).await.unwrap();
        tracker.track_certificate(fingerprint2, format!("test2.{}", domain), SystemTime::now()).await.unwrap();

        let domain_fingerprints = tracker.get_domain_fingerprints(domain).await;
        assert_eq!(domain_fingerprints.len(), 2);
        assert!(domain_fingerprints.contains(&fingerprint1));
        assert!(domain_fingerprints.contains(&fingerprint2));
    }

    #[tokio::test]
    async fn test_suspicious_activity_detection() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        // Generate many certificates for the same domain
        let domain = "suspicious.example.com";
        for i in 0..150u8 { // Above suspicious threshold
            let fingerprint = [i; 32];
            tracker.track_certificate(fingerprint, domain.to_string(), SystemTime::now()).await.unwrap();
        }

        // The last certificate should be marked as suspicious
        let last_fingerprint = [149u8; 32];
        let metadata = tracker.get_fingerprint_metadata(&last_fingerprint).await.unwrap();
        assert!(matches!(metadata.status, FingerprintStatus::Suspicious { .. }));
    }

    #[tokio::test]
    async fn test_revocation_marking() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        let fingerprint = [5u8; 32];
        let common_name = "revoked.example.com".to_string();

        // Track certificate
        tracker.track_certificate(fingerprint, common_name, SystemTime::now()).await.unwrap();

        // Mark as revoked
        tracker.mark_revoked(&fingerprint, "Private key compromised".to_string()).await.unwrap();

        let metadata = tracker.get_fingerprint_metadata(&fingerprint).await.unwrap();
        assert!(matches!(metadata.status, FingerprintStatus::Revoked { .. }));
    }

    #[tokio::test]
    async fn test_fingerprint_search() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        // Add test fingerprints
        for i in 0..10u8 {
            let fingerprint = [i; 32];
            let common_name = format!("test{}.example.com", i);
            tracker.track_certificate(fingerprint, common_name, SystemTime::now()).await.unwrap();
        }

        // Search by domain
        let query = FingerprintQuery {
            domain: Some("example.com".to_string()),
            status: None,
            since: None,
            limit: Some(5),
        };

        let results = tracker.search_fingerprints(&query).await.unwrap();
        assert_eq!(results.fingerprints.len(), 5);
        assert!(results.has_more);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let tracker = FingerprintTracker::new(true).await.unwrap();
        
        // Add some fingerprints
        for i in 0..5u8 {
            let fingerprint = [i; 32];
            tracker.track_certificate(fingerprint, format!("test{}.com", i), SystemTime::now()).await.unwrap();
        }

        let stats = tracker.get_stats().await;
        assert_eq!(stats.total_tracked, 5);
        assert!(stats.memory_usage_bytes > 0);
    }

    #[tokio::test]
    async fn test_disabled_tracker() {
        let tracker = FingerprintTracker::new(false).await.unwrap();
        
        let fingerprint = [6u8; 32];
        let status = tracker.track_certificate(fingerprint, "test.com".to_string(), SystemTime::now()).await.unwrap();
        
        assert!(matches!(status, FingerprintStatus::Normal));
        
        let metadata = tracker.get_fingerprint_metadata(&fingerprint).await;
        assert!(metadata.is_none()); // Should not track when disabled
    }
}
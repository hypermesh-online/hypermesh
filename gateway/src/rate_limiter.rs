// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, warn};

use crate::error::GatewayError;

/// Configuration for rate limiting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second per IP.
    pub requests_per_sec_per_ip: u32,
    /// Burst size (max tokens in bucket).
    pub burst_size: u32,
    /// Global rate limit (total requests per second).
    pub global_limit: u32,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: usize,
    /// Maximum concurrent connections per IP.
    pub max_connections_per_ip: u32,
    /// Cleanup interval for expired buckets.
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_sec_per_ip: 100,
            burst_size: 200,
            global_limit: 10_000,
            max_payload_bytes: 10 * 1024 * 1024, // 10 MB
            max_connections_per_ip: 50,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Token bucket for a single client.
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns true if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn is_stale(&self, max_idle: Duration) -> bool {
        self.last_refill.elapsed() > max_idle
    }
}

/// Snapshot of rate limiter statistics.
#[derive(Debug, Clone)]
pub struct RateLimitStatsSnapshot {
    pub allowed: u64,
    pub ip_limited: u64,
    pub identity_limited: u64,
    pub global_limited: u64,
    pub payload_rejected: u64,
    pub connection_limited: u64,
    pub tracked_ips: usize,
    pub tracked_identities: usize,
}

struct RateLimitStats {
    allowed: AtomicU64,
    ip_limited: AtomicU64,
    identity_limited: AtomicU64,
    global_limited: AtomicU64,
    payload_rejected: AtomicU64,
    connection_limited: AtomicU64,
}

impl RateLimitStats {
    fn new() -> Self {
        Self {
            allowed: AtomicU64::new(0),
            ip_limited: AtomicU64::new(0),
            identity_limited: AtomicU64::new(0),
            global_limited: AtomicU64::new(0),
            payload_rejected: AtomicU64::new(0),
            connection_limited: AtomicU64::new(0),
        }
    }
}

/// Rate limiter with per-IP, per-identity, and global limits.
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Per-IP token buckets.
    ip_buckets: Arc<DashMap<IpAddr, TokenBucket>>,
    /// Per-identity token buckets (for authenticated users).
    identity_buckets: Arc<DashMap<String, TokenBucket>>,
    /// Global request counter (reset per 1-second window).
    global_count: AtomicU64,
    global_window_start: std::sync::RwLock<Instant>,
    /// Connection count per IP.
    connection_counts: Arc<DashMap<IpAddr, AtomicUsize>>,
    /// Statistics.
    stats: Arc<RateLimitStats>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        debug!(
            rps_per_ip = config.requests_per_sec_per_ip,
            burst = config.burst_size,
            global = config.global_limit,
            "Rate limiter created"
        );
        Self {
            config,
            ip_buckets: Arc::new(DashMap::new()),
            identity_buckets: Arc::new(DashMap::new()),
            global_count: AtomicU64::new(0),
            global_window_start: std::sync::RwLock::new(Instant::now()),
            connection_counts: Arc::new(DashMap::new()),
            stats: Arc::new(RateLimitStats::new()),
        }
    }

    /// Check if a request from the given IP is allowed.
    pub fn check_ip(&self, ip: IpAddr) -> Result<(), GatewayError> {
        // Check global limit first
        self.check_global()?;

        // Check per-IP bucket
        let mut entry = self.ip_buckets.entry(ip).or_insert_with(|| {
            TokenBucket::new(
                self.config.burst_size as f64,
                self.config.requests_per_sec_per_ip as f64,
            )
        });

        if entry.try_consume() {
            self.stats.allowed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.stats.ip_limited.fetch_add(1, Ordering::Relaxed);
            warn!(ip = %ip, "IP rate limit exceeded");
            Err(GatewayError::RateLimitExceeded {
                client: ip.to_string(),
            })
        }
    }

    /// Check if a request from an authenticated identity is allowed.
    ///
    /// Authenticated users receive double the burst and refill rate.
    pub fn check_identity(&self, identity: &str) -> Result<(), GatewayError> {
        let mut entry =
            self.identity_buckets
                .entry(identity.to_string())
                .or_insert_with(|| {
                    TokenBucket::new(
                        self.config.burst_size as f64 * 2.0,
                        self.config.requests_per_sec_per_ip as f64 * 2.0,
                    )
                });

        if entry.try_consume() {
            Ok(())
        } else {
            self.stats
                .identity_limited
                .fetch_add(1, Ordering::Relaxed);
            warn!(identity = %identity, "Identity rate limit exceeded");
            Err(GatewayError::RateLimitExceeded {
                client: identity.to_string(),
            })
        }
    }

    /// Check global rate limit.
    fn check_global(&self) -> Result<(), GatewayError> {
        // Reset window if more than 1 second has elapsed.
        {
            let mut start = self
                .global_window_start
                .write()
                .expect("rate limiter global window lock poisoned");
            if start.elapsed() > Duration::from_secs(1) {
                self.global_count.store(0, Ordering::Relaxed);
                *start = Instant::now();
            }
        }

        let count = self.global_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.config.global_limit as u64 {
            self.stats.global_limited.fetch_add(1, Ordering::Relaxed);
            warn!(count = count, limit = self.config.global_limit, "Global rate limit exceeded");
            Err(GatewayError::RateLimitExceeded {
                client: "global".to_string(),
            })
        } else {
            Ok(())
        }
    }

    /// Check payload size against the configured maximum.
    pub fn check_payload_size(&self, size: usize) -> Result<(), GatewayError> {
        if size > self.config.max_payload_bytes {
            self.stats.payload_rejected.fetch_add(1, Ordering::Relaxed);
            warn!(
                size = size,
                max = self.config.max_payload_bytes,
                "Payload size exceeds limit"
            );
            Err(GatewayError::RateLimitExceeded {
                client: format!(
                    "payload too large: {} > {}",
                    size, self.config.max_payload_bytes
                ),
            })
        } else {
            Ok(())
        }
    }

    /// Track a new connection for an IP. Returns error if over limit.
    pub fn track_connection(&self, ip: IpAddr) -> Result<(), GatewayError> {
        let entry = self
            .connection_counts
            .entry(ip)
            .or_insert_with(|| AtomicUsize::new(0));
        let count = entry.fetch_add(1, Ordering::Relaxed);
        if count >= self.config.max_connections_per_ip as usize {
            entry.fetch_sub(1, Ordering::Relaxed);
            self.stats
                .connection_limited
                .fetch_add(1, Ordering::Relaxed);
            warn!(ip = %ip, count = count + 1, limit = self.config.max_connections_per_ip, "Connection limit exceeded");
            Err(GatewayError::RateLimitExceeded {
                client: format!("connections:{}", ip),
            })
        } else {
            Ok(())
        }
    }

    /// Release a connection count for an IP.
    pub fn release_connection(&self, ip: IpAddr) {
        if let Some(entry) = self.connection_counts.get(&ip) {
            let prev = entry.fetch_sub(1, Ordering::Relaxed);
            // Remove the entry entirely when the count reaches zero.
            if prev <= 1 {
                drop(entry);
                self.connection_counts.remove(&ip);
            }
        }
    }

    /// Remove stale buckets that have been idle longer than twice the
    /// cleanup interval. Returns the number of entries removed.
    pub fn cleanup(&self) -> usize {
        let max_idle = self.config.cleanup_interval * 2;
        let before_ip = self.ip_buckets.len();
        self.ip_buckets.retain(|_, b| !b.is_stale(max_idle));
        let after_ip = self.ip_buckets.len();

        let before_id = self.identity_buckets.len();
        self.identity_buckets.retain(|_, b| !b.is_stale(max_idle));
        let after_id = self.identity_buckets.len();

        let removed = (before_ip - after_ip) + (before_id - after_id);
        if removed > 0 {
            debug!(removed = removed, "Cleaned up stale rate limit buckets");
        }
        removed
    }

    /// Get a snapshot of rate limiter statistics.
    pub fn rate_limit_stats(&self) -> RateLimitStatsSnapshot {
        RateLimitStatsSnapshot {
            allowed: self.stats.allowed.load(Ordering::Relaxed),
            ip_limited: self.stats.ip_limited.load(Ordering::Relaxed),
            identity_limited: self.stats.identity_limited.load(Ordering::Relaxed),
            global_limited: self.stats.global_limited.load(Ordering::Relaxed),
            payload_rejected: self.stats.payload_rejected.load(Ordering::Relaxed),
            connection_limited: self.stats.connection_limited.load(Ordering::Relaxed),
            tracked_ips: self.ip_buckets.len(),
            tracked_identities: self.identity_buckets.len(),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn default_limiter() -> RateLimiter {
        RateLimiter::new(RateLimitConfig::default())
    }

    fn ip(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn default_config_values() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.requests_per_sec_per_ip, 100);
        assert_eq!(cfg.burst_size, 200);
        assert_eq!(cfg.global_limit, 10_000);
        assert_eq!(cfg.max_payload_bytes, 10 * 1024 * 1024);
        assert_eq!(cfg.max_connections_per_ip, 50);
        assert_eq!(cfg.cleanup_interval, Duration::from_secs(60));
    }

    #[test]
    fn ip_rate_limit_allows_within_burst() {
        let rl = default_limiter();
        let addr = ip(10, 0, 0, 1);
        // Should allow up to burst_size requests
        for _ in 0..200 {
            assert!(rl.check_ip(addr).is_ok());
        }
    }

    #[test]
    fn ip_rate_limit_denies_over_burst() {
        let rl = RateLimiter::new(RateLimitConfig {
            burst_size: 5,
            requests_per_sec_per_ip: 5,
            ..RateLimitConfig::default()
        });
        let addr = ip(10, 0, 0, 2);

        // Consume all 5 tokens
        for _ in 0..5 {
            assert!(rl.check_ip(addr).is_ok());
        }
        // 6th request should be denied
        let result = rl.check_ip(addr);
        assert!(result.is_err());
        match result.expect_err("test: should be rate limited") {
            GatewayError::RateLimitExceeded { client } => {
                assert_eq!(client, "10.0.0.2");
            }
            other => unreachable!("test: expected RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn global_limit_enforced() {
        let rl = RateLimiter::new(RateLimitConfig {
            global_limit: 3,
            burst_size: 1000,
            requests_per_sec_per_ip: 1000,
            ..RateLimitConfig::default()
        });

        // Use different IPs to avoid per-IP limit
        assert!(rl.check_ip(ip(1, 0, 0, 1)).is_ok());
        assert!(rl.check_ip(ip(1, 0, 0, 2)).is_ok());
        assert!(rl.check_ip(ip(1, 0, 0, 3)).is_ok());

        let result = rl.check_ip(ip(1, 0, 0, 4));
        assert!(result.is_err());
        match result.expect_err("test: should be global limited") {
            GatewayError::RateLimitExceeded { client } => {
                assert_eq!(client, "global");
            }
            other => unreachable!("test: expected global RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn payload_size_allowed_within_limit() {
        let rl = default_limiter();
        assert!(rl.check_payload_size(1024).is_ok());
        assert!(rl.check_payload_size(10 * 1024 * 1024).is_ok());
    }

    #[test]
    fn payload_size_rejected_over_limit() {
        let rl = RateLimiter::new(RateLimitConfig {
            max_payload_bytes: 1000,
            ..RateLimitConfig::default()
        });
        assert!(rl.check_payload_size(999).is_ok());
        assert!(rl.check_payload_size(1000).is_ok());
        let result = rl.check_payload_size(1001);
        assert!(result.is_err());
    }

    #[test]
    fn connection_tracking_allows_within_limit() {
        let rl = RateLimiter::new(RateLimitConfig {
            max_connections_per_ip: 3,
            ..RateLimitConfig::default()
        });
        let addr = ip(192, 168, 1, 1);

        assert!(rl.track_connection(addr).is_ok());
        assert!(rl.track_connection(addr).is_ok());
        assert!(rl.track_connection(addr).is_ok());
        // 4th should fail
        assert!(rl.track_connection(addr).is_err());
    }

    #[test]
    fn connection_release_frees_slot() {
        let rl = RateLimiter::new(RateLimitConfig {
            max_connections_per_ip: 2,
            ..RateLimitConfig::default()
        });
        let addr = ip(192, 168, 1, 2);

        assert!(rl.track_connection(addr).is_ok());
        assert!(rl.track_connection(addr).is_ok());
        assert!(rl.track_connection(addr).is_err());

        rl.release_connection(addr);
        // Should allow again after releasing
        assert!(rl.track_connection(addr).is_ok());
    }

    #[test]
    fn connection_release_unknown_ip_is_noop() {
        let rl = default_limiter();
        // Should not panic
        rl.release_connection(ip(99, 99, 99, 99));
    }

    #[test]
    fn identity_rate_limit_allows_within_burst() {
        let rl = RateLimiter::new(RateLimitConfig {
            burst_size: 5,
            requests_per_sec_per_ip: 5,
            ..RateLimitConfig::default()
        });

        // Identity gets 2x burst = 10
        for _ in 0..10 {
            assert!(rl.check_identity("alice").is_ok());
        }
        // 11th should be denied
        assert!(rl.check_identity("alice").is_err());
    }

    #[test]
    fn identity_rate_limit_separate_from_ip() {
        let rl = RateLimiter::new(RateLimitConfig {
            burst_size: 3,
            requests_per_sec_per_ip: 3,
            ..RateLimitConfig::default()
        });

        // Exhaust IP limit
        let addr = ip(10, 0, 0, 5);
        for _ in 0..3 {
            assert!(rl.check_ip(addr).is_ok());
        }
        assert!(rl.check_ip(addr).is_err());

        // Identity limit should be independent (burst * 2 = 6)
        for _ in 0..6 {
            assert!(rl.check_identity("bob").is_ok());
        }
        assert!(rl.check_identity("bob").is_err());
    }

    #[test]
    fn cleanup_removes_stale_buckets() {
        let rl = RateLimiter::new(RateLimitConfig {
            cleanup_interval: Duration::from_millis(0),
            burst_size: 100,
            requests_per_sec_per_ip: 100,
            ..RateLimitConfig::default()
        });

        // Create some buckets
        let _ = rl.check_ip(ip(10, 0, 0, 1));
        let _ = rl.check_ip(ip(10, 0, 0, 2));
        let _ = rl.check_identity("alice");

        assert!(rl.ip_buckets.len() >= 2);
        assert_eq!(rl.identity_buckets.len(), 1);

        // With cleanup_interval=0ms, max_idle = 0ms. After a brief sleep
        // all buckets will be stale.
        std::thread::sleep(Duration::from_millis(2));
        let removed = rl.cleanup();
        assert!(removed >= 3, "expected at least 3 removals, got {removed}");
        assert_eq!(rl.ip_buckets.len(), 0);
        assert_eq!(rl.identity_buckets.len(), 0);
    }

    #[test]
    fn stats_reflect_operations() {
        let rl = RateLimiter::new(RateLimitConfig {
            burst_size: 2,
            requests_per_sec_per_ip: 2,
            global_limit: 100,
            max_payload_bytes: 500,
            max_connections_per_ip: 1,
            ..RateLimitConfig::default()
        });

        let addr = ip(10, 0, 0, 10);
        let _ = rl.check_ip(addr); // allowed
        let _ = rl.check_ip(addr); // allowed
        let _ = rl.check_ip(addr); // ip_limited

        let _ = rl.check_payload_size(9999); // payload_rejected

        let _ = rl.track_connection(addr); // ok
        let _ = rl.track_connection(addr); // connection_limited

        let _ = rl.check_identity("x"); // uses global counter, identity ok
        let _ = rl.check_identity("x"); // identity ok
        let _ = rl.check_identity("x"); // identity ok
        let _ = rl.check_identity("x"); // identity ok
        let _ = rl.check_identity("x"); // identity_limited (burst*2 = 4)

        let stats = rl.rate_limit_stats();
        assert_eq!(stats.allowed, 2, "expected 2 allowed IP requests");
        assert_eq!(stats.ip_limited, 1, "expected 1 IP limited");
        assert_eq!(stats.payload_rejected, 1, "expected 1 payload rejected");
        assert_eq!(stats.connection_limited, 1, "expected 1 connection limited");
        assert!(stats.tracked_ips >= 1, "expected at least 1 tracked IP");
    }

    #[test]
    fn different_ips_have_separate_buckets() {
        let rl = RateLimiter::new(RateLimitConfig {
            burst_size: 2,
            requests_per_sec_per_ip: 2,
            global_limit: 100,
            ..RateLimitConfig::default()
        });

        let a = ip(10, 0, 0, 1);
        let b = ip(10, 0, 0, 2);

        // Exhaust IP A
        assert!(rl.check_ip(a).is_ok());
        assert!(rl.check_ip(a).is_ok());
        assert!(rl.check_ip(a).is_err());

        // IP B should still have full burst
        assert!(rl.check_ip(b).is_ok());
        assert!(rl.check_ip(b).is_ok());
        assert!(rl.check_ip(b).is_err());
    }

    #[test]
    fn config_accessor() {
        let cfg = RateLimitConfig {
            burst_size: 42,
            ..RateLimitConfig::default()
        };
        let rl = RateLimiter::new(cfg);
        assert_eq!(rl.config().burst_size, 42);
    }
}

// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use gateway::error::GatewayError;
use gateway::rate_limiter::{RateLimitConfig, RateLimiter};

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

    for _ in 0..5 {
        assert!(rl.check_ip(addr).is_ok());
    }
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
    assert!(rl.track_connection(addr).is_ok());
}

#[test]
fn connection_release_unknown_ip_is_noop() {
    let rl = default_limiter();
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
    assert!(rl.check_identity("alice").is_err());
}

#[test]
fn identity_rate_limit_separate_from_ip() {
    let rl = RateLimiter::new(RateLimitConfig {
        burst_size: 3,
        requests_per_sec_per_ip: 3,
        ..RateLimitConfig::default()
    });

    let addr = ip(10, 0, 0, 5);
    for _ in 0..3 {
        assert!(rl.check_ip(addr).is_ok());
    }
    assert!(rl.check_ip(addr).is_err());

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

    let _ = rl.check_ip(ip(10, 0, 0, 1));
    let _ = rl.check_ip(ip(10, 0, 0, 2));
    let _ = rl.check_identity("alice");

    std::thread::sleep(Duration::from_millis(2));
    let removed = rl.cleanup();
    assert!(removed >= 3, "expected at least 3 removals, got {removed}");
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

    let _ = rl.check_identity("x");
    let _ = rl.check_identity("x");
    let _ = rl.check_identity("x");
    let _ = rl.check_identity("x");
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

    assert!(rl.check_ip(a).is_ok());
    assert!(rl.check_ip(a).is_ok());
    assert!(rl.check_ip(a).is_err());

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

//! Time utilities for Nexus components

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// High-precision timestamp with nanosecond accuracy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    nanos_since_epoch: u64,
}

impl Timestamp {
    /// Get current timestamp
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        
        Self {
            nanos_since_epoch: now.as_nanos() as u64,
        }
    }
    
    /// Create timestamp from nanoseconds since epoch
    pub fn from_nanos(nanos: u64) -> Self {
        Self {
            nanos_since_epoch: nanos,
        }
    }
    
    /// Create timestamp from seconds since epoch
    pub fn from_secs(secs: u64) -> Self {
        Self {
            nanos_since_epoch: secs * 1_000_000_000,
        }
    }
    
    /// Get nanoseconds since epoch
    pub fn as_nanos(&self) -> u64 {
        self.nanos_since_epoch
    }
    
    /// Get seconds since epoch
    pub fn as_secs(&self) -> u64 {
        self.nanos_since_epoch / 1_000_000_000
    }
    
    /// Get duration since this timestamp
    pub fn elapsed(&self) -> Duration {
        let now = Self::now();
        Duration::from_nanos(now.nanos_since_epoch - self.nanos_since_epoch)
    }
    
    /// Add duration to timestamp
    pub fn add(&self, duration: Duration) -> Self {
        Self {
            nanos_since_epoch: self.nanos_since_epoch + duration.as_nanos() as u64,
        }
    }
    
    /// Subtract duration from timestamp
    pub fn sub(&self, duration: Duration) -> Self {
        Self {
            nanos_since_epoch: self.nanos_since_epoch - duration.as_nanos() as u64,
        }
    }
    
    /// Convert to RFC3339 string
    pub fn to_rfc3339(&self) -> String {
        let secs = self.as_secs() as i64;
        let nanos = (self.nanos_since_epoch % 1_000_000_000) as u32;
        
        DateTime::<Utc>::from_timestamp(secs, nanos)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
            .to_rfc3339()
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_rfc3339())
    }
}

impl From<SystemTime> for Timestamp {
    fn from(system_time: SystemTime) -> Self {
        let duration = system_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        
        Self {
            nanos_since_epoch: duration.as_nanos() as u64,
        }
    }
}

impl From<Timestamp> for SystemTime {
    fn from(timestamp: Timestamp) -> Self {
        UNIX_EPOCH + Duration::from_nanos(timestamp.nanos_since_epoch)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Time window for validating message freshness
#[derive(Debug, Clone)]
pub struct TimeWindow {
    max_skew: Duration,
    max_age: Duration,
}

impl TimeWindow {
    /// Create a new time window
    pub fn new(max_skew: Duration, max_age: Duration) -> Self {
        Self { max_skew, max_age }
    }
    
    /// Default time window (5 minute skew, 10 minute age)
    pub fn default() -> Self {
        Self {
            max_skew: Duration::from_secs(300),  // 5 minutes
            max_age: Duration::from_secs(600),   // 10 minutes
        }
    }
    
    /// Check if timestamp is within valid window
    pub fn is_valid(&self, timestamp: Timestamp) -> bool {
        let now = Timestamp::now();
        let diff = if now >= timestamp {
            Duration::from_nanos(now.as_nanos() - timestamp.as_nanos())
        } else {
            Duration::from_nanos(timestamp.as_nanos() - now.as_nanos())
        };
        
        // Check for clock skew (future timestamps)
        if timestamp > now && diff > self.max_skew {
            return false;
        }
        
        // Check for message age (old timestamps)
        if now >= timestamp && diff > self.max_age {
            return false;
        }
        
        true
    }
}

/// Performance timer for measuring execution time
pub struct PerfTimer {
    start: std::time::Instant,
    label: String,
}

impl PerfTimer {
    /// Start a new performance timer
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            start: std::time::Instant::now(),
            label: label.into(),
        }
    }
    
    /// Get elapsed time without stopping timer
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    /// Stop timer and return elapsed time
    pub fn stop(self) -> Duration {
        let elapsed = self.start.elapsed();
        tracing::debug!("{} completed in {:?}", self.label, elapsed);
        elapsed
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: std::sync::atomic::AtomicU64,
    max_tokens: u64,
    refill_rate: u64, // tokens per second
    last_refill: std::sync::Mutex<std::time::Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            tokens: std::sync::atomic::AtomicU64::new(max_tokens),
            max_tokens,
            refill_rate,
            last_refill: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }
    
    /// Try to acquire tokens
    pub fn try_acquire(&self, tokens: u64) -> bool {
        use std::sync::atomic::Ordering;
        
        // Refill tokens based on elapsed time
        self.refill();
        
        // Try to acquire tokens atomically
        loop {
            let current = self.tokens.load(Ordering::Acquire);
            if current < tokens {
                return false;
            }
            
            let new_value = current - tokens;
            if self.tokens.compare_exchange_weak(
                current, 
                new_value, 
                Ordering::Release, 
                Ordering::Relaxed
            ).is_ok() {
                return true;
            }
        }
    }
    
    fn refill(&self) {
        use std::sync::atomic::Ordering;
        
        let mut last_refill = self.last_refill.lock().unwrap();
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*last_refill);
        
        if elapsed >= Duration::from_millis(100) { // Refill every 100ms
            let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate as f64) as u64;
            if tokens_to_add > 0 {
                let current = self.tokens.load(Ordering::Acquire);
                let new_value = (current + tokens_to_add).min(self.max_tokens);
                self.tokens.store(new_value, Ordering::Release);
                *last_refill = now;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_timestamp_creation() {
        let ts1 = Timestamp::now();
        thread::sleep(Duration::from_millis(10));
        let ts2 = Timestamp::now();
        
        assert!(ts2 > ts1);
        assert!(ts1.elapsed() >= Duration::from_millis(10));
    }
    
    #[test]
    fn test_timestamp_arithmetic() {
        let ts = Timestamp::from_secs(1000);
        let future = ts.add(Duration::from_secs(100));
        let past = ts.sub(Duration::from_secs(100));
        
        assert_eq!(future.as_secs(), 1100);
        assert_eq!(past.as_secs(), 900);
    }
    
    #[test]
    fn test_time_window() {
        let window = TimeWindow::new(
            Duration::from_secs(60),  // 1 minute skew
            Duration::from_secs(300), // 5 minute age
        );
        
        let now = Timestamp::now();
        assert!(window.is_valid(now));
        
        let future = now.add(Duration::from_secs(30));
        assert!(window.is_valid(future));
        
        let far_future = now.add(Duration::from_secs(120));
        assert!(!window.is_valid(far_future));
        
        let old = now.sub(Duration::from_secs(400));
        assert!(!window.is_valid(old));
    }
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, 5); // 10 tokens, 5/sec refill
        
        // Should be able to acquire initial tokens
        assert!(limiter.try_acquire(5));
        assert!(limiter.try_acquire(5));
        
        // Should fail to acquire more
        assert!(!limiter.try_acquire(1));
        
        // Wait for refill and try again
        thread::sleep(Duration::from_millis(1200));
        assert!(limiter.try_acquire(5));
    }
}
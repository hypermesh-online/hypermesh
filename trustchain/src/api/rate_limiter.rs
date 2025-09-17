//! Rate Limiter Implementation
//! 
//! Token bucket rate limiter for API endpoints with per-client tracking.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use tracing::{debug, warn};

use crate::errors::Result as TrustChainResult;

/// Token bucket rate limiter
pub struct RateLimiter {
    /// Rate limit per minute
    rate_limit: u32,
    /// Client buckets
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    /// Cleanup task handle
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Token bucket for individual clients
#[derive(Clone, Debug)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Maximum tokens (bucket capacity)  
    capacity: f64,
    /// Token refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    /// Create new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume tokens from bucket
    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    /// Check if bucket has been idle for too long
    fn is_stale(&self, max_idle: Duration) -> bool {
        self.last_refill.elapsed() > max_idle
    }
}

impl RateLimiter {
    /// Create new rate limiter
    pub async fn new(rate_limit_per_minute: u32) -> TrustChainResult<Self> {
        debug!("Initializing rate limiter: {} requests/minute", rate_limit_per_minute);

        let limiter = Self {
            rate_limit: rate_limit_per_minute,
            buckets: Arc::new(RwLock::new(HashMap::new())),
            cleanup_handle: Arc::new(Mutex::new(None)),
        };

        // Start cleanup task
        limiter.start_cleanup_task().await;

        debug!("Rate limiter initialized successfully");
        Ok(limiter)
    }

    /// Check if client can make a request
    pub async fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        
        // Get or create bucket for client
        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| {
                let capacity = self.rate_limit as f64;
                let refill_rate = capacity / 60.0; // Per second refill rate
                TokenBucket::new(capacity, refill_rate)
            });

        // Try to consume one token
        let allowed = bucket.try_consume(1.0);
        
        if !allowed {
            warn!("Rate limit exceeded for client: {}", client_id);
        }
        
        debug!("Rate limit check for {}: allowed={}, tokens={:.2}", 
               client_id, allowed, bucket.tokens);
        
        allowed
    }

    /// Check if client can make multiple requests
    pub async fn check_rate_limit_bulk(&self, client_id: &str, token_count: u32) -> bool {
        let mut buckets = self.buckets.write().await;
        
        // Get or create bucket for client
        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| {
                let capacity = self.rate_limit as f64;
                let refill_rate = capacity / 60.0;
                TokenBucket::new(capacity, refill_rate)
            });

        // Try to consume specified tokens
        let allowed = bucket.try_consume(token_count as f64);
        
        if !allowed {
            warn!("Bulk rate limit exceeded for client: {} (requested {} tokens)", 
                  client_id, token_count);
        }
        
        debug!("Bulk rate limit check for {}: allowed={}, tokens={:.2}, requested={}", 
               client_id, allowed, bucket.tokens, token_count);
        
        allowed
    }

    /// Get remaining tokens for client
    pub async fn get_remaining_tokens(&self, client_id: &str) -> f64 {
        let mut buckets = self.buckets.write().await;
        
        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| {
                let capacity = self.rate_limit as f64;
                let refill_rate = capacity / 60.0;
                TokenBucket::new(capacity, refill_rate)
            });

        bucket.refill();
        bucket.tokens
    }

    /// Get rate limiter statistics
    pub async fn get_stats(&self) -> RateLimiterStats {
        let buckets = self.buckets.read().await;
        let active_clients = buckets.len();
        
        let total_tokens: f64 = buckets.values().map(|bucket| bucket.tokens).sum();
        let average_tokens = if active_clients > 0 {
            total_tokens / active_clients as f64
        } else {
            0.0
        };

        RateLimiterStats {
            rate_limit_per_minute: self.rate_limit,
            active_clients,
            average_tokens_remaining: average_tokens,
            total_buckets: buckets.len(),
        }
    }

    /// Clear all rate limit buckets
    pub async fn clear_all(&self) -> TrustChainResult<()> {
        let mut buckets = self.buckets.write().await;
        let count = buckets.len();
        buckets.clear();
        debug!("Cleared {} rate limit buckets", count);
        Ok(())
    }

    /// Remove specific client bucket
    pub async fn clear_client(&self, client_id: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        let removed = buckets.remove(client_id).is_some();
        if removed {
            debug!("Cleared rate limit bucket for client: {}", client_id);
        }
        removed
    }

    // Internal helper methods

    async fn start_cleanup_task(&self) {
        let buckets_clone = Arc::clone(&self.buckets);
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            let max_idle = Duration::from_secs(3600); // 1 hour
            
            loop {
                interval.tick().await;
                
                let mut buckets = buckets_clone.write().await;
                let mut stale_clients = Vec::new();
                
                // Find stale buckets
                for (client_id, bucket) in buckets.iter() {
                    if bucket.is_stale(max_idle) {
                        stale_clients.push(client_id.clone());
                    }
                }
                
                // Remove stale buckets
                for client_id in &stale_clients {
                    buckets.remove(client_id);
                }
                
                if !stale_clients.is_empty() {
                    debug!("Cleaned up {} stale rate limit buckets", stale_clients.len());
                }
            }
        });

        let mut cleanup_handle = self.cleanup_handle.lock().await;
        *cleanup_handle = Some(handle);
    }
}

/// Rate limiter statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimiterStats {
    pub rate_limit_per_minute: u32,
    pub active_clients: usize,
    pub average_tokens_remaining: f64,
    pub total_buckets: usize,
}

/// Rate limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Burst capacity (tokens available immediately)
    pub burst_capacity: Option<u32>,
    /// Cleanup interval for stale buckets (seconds)
    pub cleanup_interval: u32,
    /// Maximum idle time before bucket cleanup (seconds)
    pub max_idle_time: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_capacity: None,
            cleanup_interval: 300, // 5 minutes
            max_idle_time: 3600,   // 1 hour
        }
    }
}

/// Advanced rate limiter with configuration
pub struct ConfigurableRateLimiter {
    limiter: RateLimiter,
    config: RateLimitConfig,
}

impl ConfigurableRateLimiter {
    /// Create new configurable rate limiter
    pub async fn new(config: RateLimitConfig) -> TrustChainResult<Self> {
        let limiter = RateLimiter::new(config.requests_per_minute).await?;
        
        Ok(Self {
            limiter,
            config,
        })
    }

    /// Check rate limit with configuration
    pub async fn check_rate_limit(&self, client_id: &str) -> bool {
        self.limiter.check_rate_limit(client_id).await
    }

    /// Get rate limiter configuration
    pub fn get_config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Get rate limiter statistics
    pub async fn get_stats(&self) -> RateLimiterStats {
        self.limiter.get_stats().await
    }
}

/// Rate limit error information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitError {
    pub client_id: String,
    pub current_limit: u32,
    pub retry_after_seconds: u32,
    pub remaining_tokens: f64,
}

impl RateLimitError {
    pub fn new(
        client_id: String,
        current_limit: u32,
        remaining_tokens: f64,
    ) -> Self {
        // Calculate retry after based on refill rate
        let refill_rate = current_limit as f64 / 60.0; // tokens per second
        let tokens_needed = 1.0 - remaining_tokens;
        let retry_after_seconds = if tokens_needed > 0.0 {
            (tokens_needed / refill_rate).ceil() as u32
        } else {
            1
        };

        Self {
            client_id,
            current_limit,
            retry_after_seconds,
            remaining_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(60).await.unwrap();
        assert_eq!(limiter.rate_limit, 60);
    }

    #[tokio::test]
    async fn test_rate_limit_allow() {
        let limiter = RateLimiter::new(60).await.unwrap();
        
        // First request should be allowed
        let allowed = limiter.check_rate_limit("client1").await;
        assert!(allowed);
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 1);
    }

    #[tokio::test]
    async fn test_rate_limit_exceed() {
        let limiter = RateLimiter::new(2).await.unwrap(); // Very low limit
        
        // First two requests should be allowed
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client1").await);
        
        // Third request should be blocked
        let blocked = limiter.check_rate_limit("client1").await;
        assert!(!blocked);
    }

    #[tokio::test]
    async fn test_rate_limit_refill() {
        let limiter = RateLimiter::new(60).await.unwrap(); // 1 token per second
        
        // Consume all tokens
        for _ in 0..60 {
            limiter.check_rate_limit("client1").await;
        }
        
        // Should be blocked now
        assert!(!limiter.check_rate_limit("client1").await);
        
        // Wait for refill (in real scenarios this would be automatic)
        sleep(Duration::from_secs(2)).await;
        
        // Should be allowed again after refill
        assert!(limiter.check_rate_limit("client1").await);
    }

    #[tokio::test]
    async fn test_multiple_clients() {
        let limiter = RateLimiter::new(5).await.unwrap();
        
        // Different clients should have separate buckets
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client2").await);
        assert!(limiter.check_rate_limit("client1").await);
        assert!(limiter.check_rate_limit("client2").await);
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 2);
    }

    #[tokio::test]
    async fn test_bulk_rate_limit() {
        let limiter = RateLimiter::new(10).await.unwrap();
        
        // Should allow bulk request within limit
        assert!(limiter.check_rate_limit_bulk("client1", 5).await);
        
        // Should block bulk request exceeding remaining tokens
        assert!(!limiter.check_rate_limit_bulk("client1", 10).await);
        
        // Should still allow smaller request
        assert!(limiter.check_rate_limit_bulk("client1", 3).await);
    }

    #[tokio::test]
    async fn test_remaining_tokens() {
        let limiter = RateLimiter::new(10).await.unwrap();
        
        let initial_tokens = limiter.get_remaining_tokens("client1").await;
        assert_eq!(initial_tokens, 10.0);
        
        // Consume some tokens
        limiter.check_rate_limit("client1").await;
        limiter.check_rate_limit("client1").await;
        
        let remaining = limiter.get_remaining_tokens("client1").await;
        assert_eq!(remaining, 8.0);
    }

    #[tokio::test]
    async fn test_clear_client() {
        let limiter = RateLimiter::new(5).await.unwrap();
        
        // Create bucket for client
        limiter.check_rate_limit("client1").await;
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 1);
        
        // Clear client bucket
        let cleared = limiter.clear_client("client1").await;
        assert!(cleared);
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 0);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let limiter = RateLimiter::new(5).await.unwrap();
        
        // Create buckets for multiple clients
        limiter.check_rate_limit("client1").await;
        limiter.check_rate_limit("client2").await;
        limiter.check_rate_limit("client3").await;
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 3);
        
        // Clear all buckets
        limiter.clear_all().await.unwrap();
        
        let stats = limiter.get_stats().await;
        assert_eq!(stats.active_clients, 0);
    }

    #[tokio::test]
    async fn test_configurable_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_minute: 30,
            burst_capacity: Some(10),
            cleanup_interval: 60,
            max_idle_time: 300,
        };
        
        let limiter = ConfigurableRateLimiter::new(config).await.unwrap();
        assert_eq!(limiter.get_config().requests_per_minute, 30);
        
        let allowed = limiter.check_rate_limit("client1").await;
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_error() {
        let error = RateLimitError::new("client1".to_string(), 60, 0.5);
        
        assert_eq!(error.client_id, "client1");
        assert_eq!(error.current_limit, 60);
        assert_eq!(error.remaining_tokens, 0.5);
        assert!(error.retry_after_seconds > 0);
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10.0, 1.0); // 1 token per second
        
        // Consume all tokens
        assert!(bucket.try_consume(10.0));
        assert!(!bucket.try_consume(1.0)); // Should fail
        
        // Simulate time passing
        sleep(Duration::from_millis(2000)).await;
        
        // Should have refilled some tokens
        bucket.refill();
        assert!(bucket.tokens > 0.0);
    }
}
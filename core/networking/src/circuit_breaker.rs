//! Circuit breaker implementation for fault tolerance

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    half_open_calls: Arc<RwLock<u32>>,
}

impl CircuitBreaker {
    pub fn new(config: &CircuitBreakerConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            half_open_calls: Arc::new(RwLock::new(0)),
        })
    }
    
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.config.timeout {
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.half_open_calls.write().await = 0;
                    } else {
                        return Err(crate::error::NetworkError::CircuitBreakerOpen.into());
                    }
                } else {
                    return Err(crate::error::NetworkError::CircuitBreakerOpen.into());
                }
            }
            CircuitState::HalfOpen => {
                let mut calls = self.half_open_calls.write().await;
                if *calls >= self.config.half_open_max_calls {
                    return Err(crate::error::NetworkError::CircuitBreakerOpen.into());
                }
                *calls += 1;
            }
            CircuitState::Closed => {}
        }
        
        match f() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
    
    async fn on_success(&self) {
        let mut success_count = self.success_count.write().await;
        *success_count += 1;
        
        let state = *self.state.read().await;
        if state == CircuitState::HalfOpen && *success_count >= self.config.success_threshold {
            *self.state.write().await = CircuitState::Closed;
            *self.failure_count.write().await = 0;
            *success_count = 0;
        }
    }
    
    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        *self.last_failure_time.write().await = Some(Instant::now());
        
        if *failure_count >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open;
            *self.success_count.write().await = 0;
        }
    }
    
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }

    pub async fn can_execute(&self) -> bool {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => {
                let calls = *self.half_open_calls.read().await;
                calls < self.config.half_open_max_calls
            }
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    last_failure.elapsed() >= self.config.timeout
                } else {
                    false
                }
            }
        }
    }

    pub async fn record_success(&self) {
        self.on_success().await;
    }

    pub async fn record_failure(&self) {
        self.on_failure().await;
    }
}
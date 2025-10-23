//! Load balancing module for service mesh

use crate::error::Result;
use crate::config::LoadBalancingConfig;
use nexus_shared::ServiceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
    WeightedRoundRobin,
    IPHash,
}

/// Backend server pool
#[derive(Debug)]
pub struct BackendPool {
    backends: Vec<SocketAddr>,
    strategy: LoadBalancingStrategy,
    current_index: usize,
}

impl BackendPool {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            backends: Vec::new(),
            strategy,
            current_index: 0,
        }
    }
    
    pub fn add_backend(&mut self, addr: SocketAddr) {
        self.backends.push(addr);
    }
    
    pub fn next(&mut self) -> Option<SocketAddr> {
        if self.backends.is_empty() {
            return None;
        }
        
        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let addr = self.backends[self.current_index];
                self.current_index = (self.current_index + 1) % self.backends.len();
                Some(addr)
            }
            LoadBalancingStrategy::Random => {
                use rand::Rng;
                let idx = rand::thread_rng().gen_range(0..self.backends.len());
                Some(self.backends[idx])
            }
            _ => Some(self.backends[0]), // Simplified for other strategies
        }
    }
}

/// Load balancer for service mesh
pub struct LoadBalancer {
    pools: Arc<RwLock<HashMap<ServiceId, BackendPool>>>,
    default_strategy: LoadBalancingStrategy,
}

impl LoadBalancer {
    pub fn new(config: &LoadBalancingConfig) -> Result<Self> {
        Ok(Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            default_strategy: config.strategy.clone(),
        })
    }
    
    pub async fn get_backend(&self, service_id: &ServiceId) -> Result<SocketAddr> {
        let mut pools = self.pools.write().await;
        let pool = pools.get_mut(service_id)
            .ok_or_else(|| crate::error::NetworkError::ServiceNotFound {
                service_id: service_id.clone(),
            })?;
        
        pool.next()
            .ok_or_else(|| crate::error::NetworkError::NoBackendsAvailable {
                service_id: service_id.clone(),
            })
    }
    
    pub async fn register_backend(
        &self,
        service_id: ServiceId,
        backend: SocketAddr,
        strategy: LoadBalancingStrategy,
    ) -> Result<()> {
        let mut pools = self.pools.write().await;
        let pool = pools.entry(service_id).or_insert_with(|| BackendPool::new(strategy));
        pool.add_backend(backend);
        Ok(())
    }

    pub async fn select_instance(&self, service_id: &ServiceId, _instances: &[SocketAddr]) -> Result<SocketAddr> {
        // Use the existing get_backend method for now
        self.get_backend(service_id).await
    }
}
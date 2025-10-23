//! Routing module for service mesh

use crate::error::Result;
use nexus_shared::ServiceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Routing rule for traffic management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub service_id: ServiceId,
    pub path_prefix: Option<String>,
    pub headers: HashMap<String, String>,
    pub weight: u32,
}

/// Traffic split configuration for canary deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplit {
    pub primary: ServiceId,
    pub primary_weight: u32,
    pub canary: ServiceId,
    pub canary_weight: u32,
}

impl TrafficSplit {
    pub fn new(primary: ServiceId, canary: ServiceId, canary_percentage: u32) -> Self {
        assert!(canary_percentage <= 100);
        Self {
            primary,
            primary_weight: 100 - canary_percentage,
            canary,
            canary_weight: canary_percentage,
        }
    }
    
    pub fn select_service(&self) -> ServiceId {
        use rand::Rng;
        let total = self.primary_weight + self.canary_weight;
        let roll = rand::thread_rng().gen_range(0..total);
        
        if roll < self.primary_weight {
            self.primary.clone()
        } else {
            self.canary.clone()
        }
    }
}

/// Router for service mesh traffic management
pub struct Router {
    rules: Arc<RwLock<Vec<RoutingRule>>>,
    traffic_splits: Arc<RwLock<HashMap<ServiceId, TrafficSplit>>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            traffic_splits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn add_rule(&self, rule: RoutingRule) -> Result<()> {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        Ok(())
    }
    
    pub async fn add_traffic_split(&self, service_id: ServiceId, split: TrafficSplit) -> Result<()> {
        let mut splits = self.traffic_splits.write().await;
        splits.insert(service_id, split);
        Ok(())
    }
    
    pub async fn route(&self, path: &str, headers: &HashMap<String, String>) -> Result<ServiceId> {
        let rules = self.rules.read().await;
        
        // Find matching rule
        for rule in rules.iter() {
            if let Some(prefix) = &rule.path_prefix {
                if !path.starts_with(prefix) {
                    continue;
                }
            }
            
            let headers_match = rule.headers.iter().all(|(k, v)| {
                headers.get(k).map(|hv| hv == v).unwrap_or(false)
            });
            
            if headers_match {
                // Check for traffic split
                let splits = self.traffic_splits.read().await;
                if let Some(split) = splits.get(&rule.service_id) {
                    return Ok(split.select_service());
                }
                return Ok(rule.service_id.clone());
            }
        }
        
        Err(crate::error::NetworkError::NoRouteFound {
            path: path.to_string(),
        }.into())
    }
}
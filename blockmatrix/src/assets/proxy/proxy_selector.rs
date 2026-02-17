// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Trust-Based Proxy Selection System
//!
//! Implements proxy selection based on trust levels, proximity, and performance

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetId, AssetResult, AssetError};
use super::trust_integration::{TrustChainIntegration, CertificateValidator};

/// Trust level for proxy nodes
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TrustLevel {
    /// No trust required
    None,
    /// Basic trust (self-signed OK)
    Basic,
    /// Medium trust (requires valid chain)
    Medium,
    /// High trust (requires known CA)
    High,
    /// Maximum trust (requires federated validation)
    Maximum,
}

impl TrustLevel {
    /// Convert to numeric score (0.0 - 1.0)
    pub fn to_score(&self) -> f32 {
        match self {
            TrustLevel::None => 0.0,
            TrustLevel::Basic => 0.25,
            TrustLevel::Medium => 0.5,
            TrustLevel::High => 0.75,
            TrustLevel::Maximum => 1.0,
        }
    }

    /// Check if trust level meets requirement
    pub fn meets_requirement(&self, required: &TrustLevel) -> bool {
        self.to_score() >= required.to_score()
    }
}

/// Proxy node information with trust and performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyNode {
    /// Node identifier
    pub node_id: String,

    /// Network address (IPv6)
    pub address: std::net::SocketAddrV6,

    /// Node certificate fingerprint
    pub certificate: String,

    /// Trust level
    pub trust_level: TrustLevel,

    /// Performance metrics
    pub performance: ProxyPerformance,

    /// Geographic location (optional)
    pub location: Option<GeoLocation>,

    /// Available capabilities
    pub capabilities: Vec<ProxyCapability>,

    /// Last health check
    pub last_health_check: SystemTime,

    /// Node status
    pub status: ProxyNodeStatus,
}

/// Proxy performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyPerformance {
    /// Average latency in microseconds
    pub avg_latency_us: u64,

    /// Bandwidth in Mbps
    pub bandwidth_mbps: f32,

    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,

    /// Current load (0.0 - 1.0)
    pub current_load: f32,

    /// Uptime percentage
    pub uptime_percentage: f32,
}

/// Geographic location
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude
    pub latitude: f64,

    /// Longitude
    pub longitude: f64,

    /// Country code
    pub country: String,

    /// City name
    pub city: Option<String>,
}

/// Proxy capabilities
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProxyCapability {
    /// Can forward memory requests
    MemoryForwarding,
    /// Can forward CPU requests
    CpuForwarding,
    /// Can forward GPU requests
    GpuForwarding,
    /// Can forward storage requests
    StorageForwarding,
    /// Supports encryption
    Encryption,
    /// Supports compression
    Compression,
    /// Supports caching
    Caching,
    /// Supports consensus validation
    ConsensusValidation,
}

/// Proxy node status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProxyNodeStatus {
    /// Node is healthy and available
    Healthy,
    /// Node is degraded but available
    Degraded,
    /// Node is overloaded
    Overloaded,
    /// Node is unavailable
    Unavailable,
    /// Node is blacklisted
    Blacklisted,
}

/// Proxy selector with trust validation
#[allow(dead_code)] // Fields used during proxy selection
pub struct ProxySelector {
    /// TrustChain integration
    trust_chain: Arc<TrustChainIntegration>,

    /// Certificate validator
    validator: Arc<CertificateValidator>,

    /// Available proxy nodes
    proxy_nodes: Arc<RwLock<HashMap<String, ProxyNode>>>,

    /// Selection cache
    selection_cache: Arc<RwLock<HashMap<String, Vec<ProxyNode>>>>,

    /// Selection configuration
    config: ProxySelectorConfig,
}

/// Proxy selector configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxySelectorConfig {
    /// Minimum required trust level
    pub min_trust_level: TrustLevel,

    /// Maximum acceptable latency (microseconds)
    pub max_latency_us: u64,

    /// Minimum required bandwidth (Mbps)
    pub min_bandwidth_mbps: f32,

    /// Minimum success rate
    pub min_success_rate: f32,

    /// Maximum load threshold
    pub max_load_threshold: f32,

    /// Prefer geographic proximity
    pub prefer_proximity: bool,

    /// Cache duration
    pub cache_duration: Duration,

    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for ProxySelectorConfig {
    fn default() -> Self {
        Self {
            min_trust_level: TrustLevel::Medium,
            max_latency_us: 10_000, // 10ms
            min_bandwidth_mbps: 100.0,
            min_success_rate: 0.95,
            max_load_threshold: 0.8,
            prefer_proximity: true,
            cache_duration: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl ProxySelector {
    /// Create new proxy selector
    pub fn new(
        trust_chain: Arc<TrustChainIntegration>,
        validator: Arc<CertificateValidator>,
        config: ProxySelectorConfig,
    ) -> Self {
        Self {
            trust_chain,
            validator,
            proxy_nodes: Arc::new(RwLock::new(HashMap::new())),
            selection_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Select proxy based on trust and proximity
    pub async fn select_proxy(
        &self,
        target_asset: &AssetId,
        required_trust_level: TrustLevel,
    ) -> AssetResult<ProxyNode> {
        // Check cache first
        let cache_key = format!("{}-{:?}", target_asset, required_trust_level);
        if let Some(cached) = self.get_cached_selection(&cache_key).await {
            if let Some(best) = cached.first() {
                return Ok(best.clone());
            }
        }

        // Get available proxies
        let available_proxies = self.discover_proxies(target_asset).await?;

        // Filter by trust level
        let mut trusted_proxies = Vec::new();
        for proxy in available_proxies {
            if self.validate_proxy_trust(&proxy, &required_trust_level).await {
                trusted_proxies.push(proxy);
            }
        }

        if trusted_proxies.is_empty() {
            return Err(AssetError::AdapterError {
                message: format!("No proxies found with required trust level: {:?}", required_trust_level)
            });
        }

        // Select best proxy by performance and proximity
        let best_proxy = self.select_best_proxy(trusted_proxies.clone()).await?;

        // Cache the selection
        self.cache_selection(&cache_key, trusted_proxies).await;

        Ok(best_proxy)
    }

    /// Validate proxy trust level
    async fn validate_proxy_trust(
        &self,
        proxy: &ProxyNode,
        required_level: &TrustLevel,
    ) -> bool {
        // Quick check against current trust level
        if !proxy.trust_level.meets_requirement(required_level) {
            return false;
        }

        // Validate certificate if high trust required
        if required_level.to_score() >= TrustLevel::High.to_score() {
            match self.validator.validate_certificate(&proxy.certificate).await {
                Ok(validation) => {
                    // Check if validation meets requirement
                    validation.trust_score >= required_level.to_score()
                }
                Err(_) => false,
            }
        } else {
            true
        }
    }

    /// Discover available proxies for asset
    async fn discover_proxies(&self, target_asset: &AssetId) -> AssetResult<Vec<ProxyNode>> {
        let nodes = self.proxy_nodes.read().await;

        // Filter nodes that can handle this asset type
        let mut available = Vec::new();
        for (_, node) in nodes.iter() {
            if node.status == ProxyNodeStatus::Healthy || node.status == ProxyNodeStatus::Degraded {
                // Check if node has required capability for asset type
                if self.node_supports_asset(node, target_asset) {
                    available.push(node.clone());
                }
            }
        }

        Ok(available)
    }

    /// Check if node supports asset type
    fn node_supports_asset(&self, node: &ProxyNode, _asset_id: &AssetId) -> bool {
        // For now, check if node has at least one forwarding capability
        node.capabilities.iter().any(|cap| matches!(cap,
            ProxyCapability::MemoryForwarding |
            ProxyCapability::CpuForwarding |
            ProxyCapability::GpuForwarding |
            ProxyCapability::StorageForwarding
        ))
    }

    /// Select best proxy from trusted list
    async fn select_best_proxy(&self, proxies: Vec<ProxyNode>) -> AssetResult<ProxyNode> {
        if proxies.is_empty() {
            return Err(AssetError::AdapterError {
                message: "No proxies available".to_string()
            });
        }

        // Score each proxy
        let mut scored_proxies: Vec<(ProxyNode, f32)> = Vec::new();
        for proxy in proxies {
            let score = self.calculate_proxy_score(&proxy);
            scored_proxies.push((proxy, score));
        }

        // Sort by score (highest first)
        scored_proxies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return best proxy
        scored_proxies.into_iter()
            .next()
            .map(|(proxy, _)| proxy)
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No suitable proxy nodes available for selection".to_string()
            })
    }

    /// Calculate proxy score based on multiple factors
    fn calculate_proxy_score(&self, proxy: &ProxyNode) -> f32 {
        let mut score = 0.0;

        // Trust level contributes 30%
        score += proxy.trust_level.to_score() * 0.3;

        // Performance contributes 40%
        let perf = &proxy.performance;

        // Latency score (inverse, lower is better)
        let latency_score = if perf.avg_latency_us <= self.config.max_latency_us {
            1.0 - (perf.avg_latency_us as f32 / self.config.max_latency_us as f32)
        } else {
            0.0
        };
        score += latency_score * 0.15;

        // Bandwidth score
        let bandwidth_score = (perf.bandwidth_mbps / 1000.0).min(1.0); // Normalize to 1Gbps
        score += bandwidth_score * 0.10;

        // Success rate
        score += perf.success_rate * 0.10;

        // Load score (inverse, lower is better)
        let load_score = 1.0 - perf.current_load;
        score += load_score * 0.05;

        // Availability contributes 20%
        score += (perf.uptime_percentage / 100.0) * 0.20;

        // Capability bonus (10%)
        let capability_score = proxy.capabilities.len() as f32 / 8.0; // Assume max 8 capabilities
        score += capability_score.min(1.0) * 0.10;

        score
    }

    /// Get cached selection
    async fn get_cached_selection(&self, key: &str) -> Option<Vec<ProxyNode>> {
        let cache = self.selection_cache.read().await;
        cache.get(key).cloned()
    }

    /// Cache selection
    async fn cache_selection(&self, key: &str, proxies: Vec<ProxyNode>) {
        let mut cache = self.selection_cache.write().await;
        cache.insert(key.to_string(), proxies);

        // Schedule cache cleanup
        let cache_clone = Arc::clone(&self.selection_cache);
        let key = key.to_string();
        let duration = self.config.cache_duration;
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let mut cache = cache_clone.write().await;
            cache.remove(&key);
        });
    }

    /// Register proxy node
    pub async fn register_proxy(&self, proxy: ProxyNode) -> AssetResult<()> {
        let mut nodes = self.proxy_nodes.write().await;
        nodes.insert(proxy.node_id.clone(), proxy);
        Ok(())
    }

    /// Update proxy status
    pub async fn update_proxy_status(&self, node_id: &str, status: ProxyNodeStatus) -> AssetResult<()> {
        let mut nodes = self.proxy_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.status = status;
            node.last_health_check = SystemTime::now();
            Ok(())
        } else {
            Err(AssetError::AdapterError {
                message: format!("Proxy node not found: {}", node_id)
            })
        }
    }

    /// Update proxy performance metrics
    pub async fn update_proxy_performance(
        &self,
        node_id: &str,
        performance: ProxyPerformance,
    ) -> AssetResult<()> {
        let mut nodes = self.proxy_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.performance = performance;
            Ok(())
        } else {
            Err(AssetError::AdapterError {
                message: format!("Proxy node not found: {}", node_id)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetType;

    #[test]
    fn test_trust_level_scoring() {
        assert_eq!(TrustLevel::None.to_score(), 0.0);
        assert_eq!(TrustLevel::Basic.to_score(), 0.25);
        assert_eq!(TrustLevel::Medium.to_score(), 0.5);
        assert_eq!(TrustLevel::High.to_score(), 0.75);
        assert_eq!(TrustLevel::Maximum.to_score(), 1.0);
    }

    #[test]
    fn test_trust_level_requirements() {
        assert!(TrustLevel::Maximum.meets_requirement(&TrustLevel::High));
        assert!(TrustLevel::High.meets_requirement(&TrustLevel::Medium));
        assert!(!TrustLevel::Basic.meets_requirement(&TrustLevel::High));
        assert!(!TrustLevel::None.meets_requirement(&TrustLevel::Basic));
    }

    #[tokio::test]
    async fn test_proxy_registration() {
        let trust_chain = Arc::new(TrustChainIntegration::new());
        let validator = Arc::new(CertificateValidator::new().unwrap());
        let selector = ProxySelector::new(trust_chain, validator, ProxySelectorConfig::default());

        let proxy = ProxyNode {
            node_id: "test-node".to_string(),
            address: "[::1]:8080".parse().unwrap(),
            certificate: "test-cert".to_string(),
            trust_level: TrustLevel::Medium,
            performance: ProxyPerformance {
                avg_latency_us: 1000,
                bandwidth_mbps: 500.0,
                success_rate: 0.99,
                current_load: 0.3,
                uptime_percentage: 99.9,
            },
            location: None,
            capabilities: vec![ProxyCapability::MemoryForwarding],
            last_health_check: SystemTime::now(),
            status: ProxyNodeStatus::Healthy,
        };

        selector.register_proxy(proxy.clone()).await.unwrap();

        // Verify proxy is registered
        let nodes = selector.proxy_nodes.read().await;
        assert!(nodes.contains_key("test-node"));
    }
}
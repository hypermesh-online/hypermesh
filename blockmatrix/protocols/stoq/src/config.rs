//! STOQ Configuration - Complete configuration for all STOQ components

use std::path::PathBuf;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::transport::TransportConfig;
use crate::routing::RoutingConfig;
use crate::chunking::ChunkingConfig;
use crate::edge::EdgeConfig;

/// Complete STOQ configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
    /// Routing configuration
    pub routing: RoutingConfig,
    /// Chunking configuration
    pub chunking: ChunkingConfig,
    /// Edge network configuration
    pub edge: EdgeConfig,
    /// Global settings
    pub global: GlobalConfig,
}

impl Default for StoqConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            routing: RoutingConfig::default(),
            chunking: ChunkingConfig::default(),
            edge: EdgeConfig::default(),
            global: GlobalConfig::default(),
        }
    }
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Enable debug logging
    pub debug: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    /// Worker thread count (None = auto)
    pub worker_threads: Option<usize>,
    /// Enable performance profiling
    pub enable_profiling: bool,
    /// Data directory for persistent storage
    pub data_dir: PathBuf,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            debug: false,
            metrics_interval: Duration::from_secs(10),
            max_memory_mb: 4096, // 4GB
            worker_threads: None, // Auto-detect
            enable_profiling: false,
            data_dir: PathBuf::from("/var/lib/stoq"),
        }
    }
}

impl StoqConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let contents = serde_yaml::to_string(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
    
    /// Create a CDN-optimized configuration
    pub fn cdn_optimized() -> Self {
        Self {
            transport: TransportConfig {
                max_connections: None, // Unlimited
                enable_migration: true,
                enable_0rtt: true,
                max_concurrent_streams: 1000,
                ..Default::default()
            },
            routing: RoutingConfig {
                algorithm: crate::routing::RoutingAlgorithm::CDNOptimized,
                matrix_size: 100000,
                enable_ml: true,
                latency_weight: 0.3,
                bandwidth_weight: 0.4,
                load_weight: 0.2,
                geo_weight: 0.1,
                ..Default::default()
            },
            chunking: ChunkingConfig {
                algorithm: crate::chunking::ChunkAlgorithm::ContentDefined,
                enable_dedup: true,
                enable_compression: true,
                avg_size: 256 * 1024, // 256KB chunks for CDN
                ..Default::default()
            },
            edge: EdgeConfig {
                max_nodes: 10000,
                replication_factor: 5,
                distribution_strategy: crate::edge::DistributionStrategy::Hybrid,
                ..Default::default()
            },
            global: GlobalConfig {
                max_memory_mb: 16384, // 16GB for CDN nodes
                ..Default::default()
            },
        }
    }
    
    /// Create a high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            transport: TransportConfig {
                max_connections: Some(100000),
                send_buffer_size: 4 * 1024 * 1024, // 4MB
                receive_buffer_size: 4 * 1024 * 1024,
                ..Default::default()
            },
            routing: RoutingConfig {
                algorithm: crate::routing::RoutingAlgorithm::MLEnhancedDijkstra,
                update_interval: Duration::from_millis(50),
                ..Default::default()
            },
            chunking: ChunkingConfig {
                algorithm: crate::chunking::ChunkAlgorithm::Adaptive,
                compression: crate::chunking::CompressionAlgorithm::Lz4, // Fast compression
                ..Default::default()
            },
            edge: EdgeConfig {
                health_check_interval: Duration::from_secs(10),
                sync_interval: Duration::from_secs(5),
                ..Default::default()
            },
            global: GlobalConfig {
                enable_profiling: true,
                ..Default::default()
            },
        }
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate transport settings
        if self.transport.port == 0 {
            return Err(anyhow::anyhow!("Invalid port number"));
        }
        
        // Validate routing settings
        if self.routing.matrix_size == 0 {
            return Err(anyhow::anyhow!("Invalid matrix size"));
        }
        
        // Validate chunking settings
        if self.chunking.min_size > self.chunking.max_size {
            return Err(anyhow::anyhow!("Min chunk size cannot be greater than max"));
        }
        
        // Validate edge settings
        if self.edge.replication_factor == 0 {
            return Err(anyhow::anyhow!("Replication factor must be at least 1"));
        }
        
        Ok(())
    }
}

/// Configuration builder for fluent API
pub struct StoqConfigBuilder {
    config: StoqConfig,
}

impl StoqConfigBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: StoqConfig::default(),
        }
    }
    
    /// Use CDN-optimized presets
    pub fn cdn_optimized(mut self) -> Self {
        self.config = StoqConfig::cdn_optimized();
        self
    }
    
    /// Use high-performance presets
    pub fn high_performance(mut self) -> Self {
        self.config = StoqConfig::high_performance();
        self
    }
    
    /// Set transport configuration
    pub fn transport(mut self, transport: TransportConfig) -> Self {
        self.config.transport = transport;
        self
    }
    
    /// Set routing configuration
    pub fn routing(mut self, routing: RoutingConfig) -> Self {
        self.config.routing = routing;
        self
    }
    
    /// Set chunking configuration
    pub fn chunking(mut self, chunking: ChunkingConfig) -> Self {
        self.config.chunking = chunking;
        self
    }
    
    /// Set edge configuration
    pub fn edge(mut self, edge: EdgeConfig) -> Self {
        self.config.edge = edge;
        self
    }
    
    /// Set global configuration
    pub fn global(mut self, global: GlobalConfig) -> Self {
        self.config.global = global;
        self
    }
    
    /// Build and validate the configuration
    pub fn build(self) -> Result<StoqConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for StoqConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = StoqConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_cdn_optimized_config() {
        let config = StoqConfig::cdn_optimized();
        assert!(config.validate().is_ok());
        assert_eq!(config.edge.replication_factor, 5);
    }
    
    #[test]
    fn test_high_performance_config() {
        let config = StoqConfig::high_performance();
        assert!(config.validate().is_ok());
        assert!(config.global.enable_profiling);
    }
    
    #[test]
    fn test_config_builder() {
        let config = StoqConfigBuilder::new()
            .cdn_optimized()
            .build();
        assert!(config.is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = StoqConfig::default();
        config.chunking.min_size = 1000;
        config.chunking.max_size = 100;
        assert!(config.validate().is_err());
    }
}
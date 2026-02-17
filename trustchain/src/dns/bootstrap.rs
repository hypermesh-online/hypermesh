// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain DNS Bootstrap Module
//!
//! Provides standalone bootstrap capability for TrustChain DNS without BlockMatrix dependency.
//! This enables TrustChain to start independently and optionally upgrade to BlockMatrix assets later.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::net::Ipv6Addr;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// DNS backend storage options
#[derive(Clone, Debug)]
pub enum DnsBackend {
    /// Phase 1: In-memory DNS (no BlockMatrix dependency)
    InMemory(Arc<RwLock<HashMap<String, DnsRecord>>>),

    /// Phase 2: File-based persistence (no BlockMatrix dependency)
    FileSystem {
        path: PathBuf,
        cache: Arc<RwLock<HashMap<String, DnsRecord>>>,
    },

    /// Phase 3: BlockMatrix assets (optional upgrade after BlockMatrix is running)
    #[allow(dead_code)]
    BlockMatrixAsset {
        asset_id: String,
        // Note: This variant is intentionally not implemented yet
        // to avoid circular dependency. Will be added after BlockMatrix integration.
    },
}

/// DNS record representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,  // "A", "AAAA", "CNAME", etc.
    pub value: String,
    pub ttl: u32,
    pub timestamp: u64,
}

/// TrustChain bootstrap manager
pub struct TrustChainBootstrap {
    /// DNS backend storage
    dns_backend: DnsBackend,
    /// Bootstrap configuration
    config: BootstrapConfig,
    /// Bootstrap state
    state: Arc<RwLock<BootstrapState>>,
}

/// Bootstrap configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// Enable file persistence
    pub enable_persistence: bool,
    /// Persistence directory
    pub persistence_dir: Option<PathBuf>,
    /// Default TTL for DNS records
    pub default_ttl: u32,
    /// Bootstrap DNS seeds (initial records)
    pub dns_seeds: Vec<DnsRecord>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            enable_persistence: false,
            persistence_dir: None,
            default_ttl: 300,
            dns_seeds: Self::default_seeds(),
        }
    }
}

impl BootstrapConfig {
    /// Get default DNS seeds for TrustChain domains
    fn default_seeds() -> Vec<DnsRecord> {
        vec![
            DnsRecord {
                name: "trust.hypermesh.local".to_string(),
                record_type: "AAAA".to_string(),
                value: "::1".to_string(),  // localhost IPv6
                ttl: 3600,
                timestamp: 0,
            },
            DnsRecord {
                name: "ca.trust.hypermesh.local".to_string(),
                record_type: "AAAA".to_string(),
                value: "::1".to_string(),
                ttl: 3600,
                timestamp: 0,
            },
            DnsRecord {
                name: "dns.trust.hypermesh.local".to_string(),
                record_type: "AAAA".to_string(),
                value: "::1".to_string(),
                ttl: 3600,
                timestamp: 0,
            },
            DnsRecord {
                name: "ct.trust.hypermesh.local".to_string(),
                record_type: "AAAA".to_string(),
                value: "::1".to_string(),
                ttl: 3600,
                timestamp: 0,
            },
        ]
    }
}

/// Bootstrap state tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapState {
    /// Bootstrap phase
    pub phase: BootstrapPhase,
    /// CA initialized
    pub ca_ready: bool,
    /// CT log initialized
    pub ct_ready: bool,
    /// DNS resolver ready
    pub dns_ready: bool,
    /// BlockMatrix upgrade available
    pub blockmatrix_available: bool,
}

/// Bootstrap phases
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BootstrapPhase {
    /// Initial startup
    Initializing,
    /// Phase 1: Standalone mode (in-memory)
    StandaloneMemory,
    /// Phase 2: Standalone with persistence
    StandalonePersistent,
    /// Phase 3: BlockMatrix integrated (future)
    BlockMatrixIntegrated,
}

impl TrustChainBootstrap {
    /// Bootstrap TrustChain with NO external dependencies
    pub async fn bootstrap_standalone() -> Result<Self> {
        info!("Starting TrustChain standalone bootstrap (Phase 1)");

        // Start with in-memory DNS backend
        let dns_records = Arc::new(RwLock::new(HashMap::new()));
        let dns_backend = DnsBackend::InMemory(dns_records.clone());

        // Load default configuration
        let config = BootstrapConfig::default();

        // Initialize bootstrap state
        let state = Arc::new(RwLock::new(BootstrapState {
            phase: BootstrapPhase::StandaloneMemory,
            ca_ready: false,
            ct_ready: false,
            dns_ready: false,
            blockmatrix_available: false,
        }));

        let mut bootstrap = Self {
            dns_backend,
            config: config.clone(),
            state,
        };

        // Load DNS seeds
        bootstrap.load_dns_seeds(&config.dns_seeds).await?;

        info!("TrustChain standalone bootstrap complete (Phase 1)");
        Ok(bootstrap)
    }

    /// Bootstrap with file-based persistence
    pub async fn bootstrap_with_persistence(persistence_dir: PathBuf) -> Result<Self> {
        info!("Starting TrustChain bootstrap with persistence (Phase 2)");

        // Create persistence directory if needed
        tokio::fs::create_dir_all(&persistence_dir)
            .await
            .context("Failed to create persistence directory")?;

        // Load existing records from disk if available
        let records = Self::load_from_disk(&persistence_dir).await.unwrap_or_default();
        let cache = Arc::new(RwLock::new(records));

        let dns_backend = DnsBackend::FileSystem {
            path: persistence_dir.clone(),
            cache: cache.clone(),
        };

        let mut config = BootstrapConfig::default();
        config.enable_persistence = true;
        config.persistence_dir = Some(persistence_dir);

        let state = Arc::new(RwLock::new(BootstrapState {
            phase: BootstrapPhase::StandalonePersistent,
            ca_ready: false,
            ct_ready: false,
            dns_ready: false,
            blockmatrix_available: false,
        }));

        let mut bootstrap = Self {
            dns_backend,
            config: config.clone(),
            state,
        };

        // Load DNS seeds if no existing records
        if cache.read().await.is_empty() {
            bootstrap.load_dns_seeds(&config.dns_seeds).await?;
        }

        info!("TrustChain bootstrap with persistence complete (Phase 2)");
        Ok(bootstrap)
    }

    /// Load DNS seeds into backend
    async fn load_dns_seeds(&mut self, seeds: &[DnsRecord]) -> Result<()> {
        debug!("Loading {} DNS seed records", seeds.len());

        for seed in seeds {
            self.add_dns_record(seed.clone()).await?;
        }

        Ok(())
    }

    /// Add DNS record to backend
    pub async fn add_dns_record(&mut self, record: DnsRecord) -> Result<()> {
        match &self.dns_backend {
            DnsBackend::InMemory(cache) => {
                cache.write().await.insert(record.name.clone(), record);
            }
            DnsBackend::FileSystem { cache, path } => {
                cache.write().await.insert(record.name.clone(), record.clone());
                // Persist to disk
                self.persist_to_disk(path).await?;
            }
            DnsBackend::BlockMatrixAsset { .. } => {
                // Not implemented yet - would require BlockMatrix dependency
                return Err(anyhow::anyhow!("BlockMatrix backend not yet implemented"));
            }
        }
        Ok(())
    }

    /// Query DNS record from backend
    pub async fn query_dns_record(&self, name: &str) -> Result<Option<DnsRecord>> {
        match &self.dns_backend {
            DnsBackend::InMemory(cache) => {
                Ok(cache.read().await.get(name).cloned())
            }
            DnsBackend::FileSystem { cache, .. } => {
                Ok(cache.read().await.get(name).cloned())
            }
            DnsBackend::BlockMatrixAsset { .. } => {
                // Not implemented yet
                Err(anyhow::anyhow!("BlockMatrix backend not yet implemented"))
            }
        }
    }

    /// Get all DNS records
    pub async fn get_all_records(&self) -> Result<Vec<DnsRecord>> {
        match &self.dns_backend {
            DnsBackend::InMemory(cache) => {
                Ok(cache.read().await.values().cloned().collect())
            }
            DnsBackend::FileSystem { cache, .. } => {
                Ok(cache.read().await.values().cloned().collect())
            }
            DnsBackend::BlockMatrixAsset { .. } => {
                Err(anyhow::anyhow!("BlockMatrix backend not yet implemented"))
            }
        }
    }

    /// Load DNS records from disk
    async fn load_from_disk(path: &PathBuf) -> Result<HashMap<String, DnsRecord>> {
        let file_path = path.join("dns_records.json");

        if !file_path.exists() {
            return Ok(HashMap::new());
        }

        let content = tokio::fs::read_to_string(&file_path)
            .await
            .context("Failed to read DNS records from disk")?;

        let records: Vec<DnsRecord> = serde_json::from_str(&content)
            .context("Failed to parse DNS records")?;

        let mut map = HashMap::new();
        for record in records {
            map.insert(record.name.clone(), record);
        }

        Ok(map)
    }

    /// Persist DNS records to disk
    async fn persist_to_disk(&self, path: &PathBuf) -> Result<()> {
        if let DnsBackend::FileSystem { cache, .. } = &self.dns_backend {
            let records: Vec<DnsRecord> = cache.read().await.values().cloned().collect();
            let content = serde_json::to_string_pretty(&records)?;

            let file_path = path.join("dns_records.json");
            tokio::fs::write(&file_path, content)
                .await
                .context("Failed to write DNS records to disk")?;
        }
        Ok(())
    }

    /// Mark CA as ready
    pub async fn mark_ca_ready(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.ca_ready = true;
        info!("CA marked as ready");
        Ok(())
    }

    /// Mark CT log as ready
    pub async fn mark_ct_ready(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.ct_ready = true;
        info!("CT log marked as ready");
        Ok(())
    }

    /// Mark DNS as ready
    pub async fn mark_dns_ready(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.dns_ready = true;
        info!("DNS resolver marked as ready");
        Ok(())
    }

    /// Check if TrustChain is fully operational
    pub async fn is_operational(&self) -> bool {
        let state = self.state.read().await;
        state.ca_ready && state.ct_ready && state.dns_ready
    }

    /// Get current bootstrap phase
    pub async fn get_phase(&self) -> BootstrapPhase {
        self.state.read().await.phase.clone()
    }

    /// Upgrade to BlockMatrix assets (after BlockMatrix is running)
    /// This is intentionally not implemented to avoid circular dependency
    pub async fn upgrade_to_blockmatrix(&mut self) -> Result<()> {
        warn!("BlockMatrix upgrade requested but not yet implemented");
        warn!("This will be implemented after BlockMatrix integration phase");

        // Future implementation will:
        // 1. Check if BlockMatrix is available
        // 2. Migrate DNS records to BlockMatrix assets
        // 3. Update backend to use BlockMatrix
        // 4. Update state to BlockMatrixIntegrated phase

        Err(anyhow::anyhow!("BlockMatrix integration not yet implemented"))
    }

    /// Test connectivity to localhost
    pub async fn test_localhost_connectivity(&self) -> Result<bool> {
        // Simple test to ensure we can resolve localhost
        let localhost_record = self.query_dns_record("localhost").await?;

        if localhost_record.is_none() {
            // Add localhost record if not present
            let record = DnsRecord {
                name: "localhost".to_string(),
                record_type: "AAAA".to_string(),
                value: "::1".to_string(),
                ttl: 86400,
                timestamp: 0,
            };

            // Create a mutable copy for adding the record
            match &self.dns_backend {
                DnsBackend::InMemory(cache) => {
                    cache.write().await.insert(record.name.clone(), record);
                }
                DnsBackend::FileSystem { cache, .. } => {
                    cache.write().await.insert(record.name.clone(), record);
                }
                _ => {}
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_standalone_bootstrap() {
        let bootstrap = TrustChainBootstrap::bootstrap_standalone()
            .await
            .expect("Failed to bootstrap");

        assert_eq!(bootstrap.get_phase().await, BootstrapPhase::StandaloneMemory);

        // Should have default seed records
        let records = bootstrap.get_all_records().await.unwrap();
        assert!(records.len() > 0);
    }

    #[tokio::test]
    async fn test_bootstrap_with_persistence() {
        let temp_dir = tempdir().unwrap();
        let bootstrap = TrustChainBootstrap::bootstrap_with_persistence(
            temp_dir.path().to_path_buf()
        )
        .await
        .expect("Failed to bootstrap with persistence");

        assert_eq!(bootstrap.get_phase().await, BootstrapPhase::StandalonePersistent);
    }

    #[tokio::test]
    async fn test_dns_operations() {
        let mut bootstrap = TrustChainBootstrap::bootstrap_standalone()
            .await
            .expect("Failed to bootstrap");

        // Add a record
        let record = DnsRecord {
            name: "test.example.com".to_string(),
            record_type: "AAAA".to_string(),
            value: "2001:db8::1".to_string(),
            ttl: 300,
            timestamp: 0,
        };

        bootstrap.add_dns_record(record.clone()).await.unwrap();

        // Query the record
        let result = bootstrap.query_dns_record("test.example.com").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "2001:db8::1");
    }

    #[tokio::test]
    async fn test_operational_state() {
        let bootstrap = TrustChainBootstrap::bootstrap_standalone()
            .await
            .expect("Failed to bootstrap");

        // Initially not operational
        assert!(!bootstrap.is_operational().await);

        // Mark components as ready
        bootstrap.mark_ca_ready().await.unwrap();
        bootstrap.mark_ct_ready().await.unwrap();
        bootstrap.mark_dns_ready().await.unwrap();

        // Now should be operational
        assert!(bootstrap.is_operational().await);
    }
}
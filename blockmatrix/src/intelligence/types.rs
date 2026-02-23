// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for the Phase 2 Intelligence Layer

use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::assets::multi_node::NetworkId;
use crate::assets::core::PrivacyMode;
use crate::assets::storage::{ContentAddress, DeduplicationResult};
use crate::assets::pipeline::PipelineStats;

/// Asset handle returned after processing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetHandle {
    /// Asset identifier
    pub asset_id: String,

    /// Content address for retrieval
    pub content_address: ContentAddress,

    /// Privacy tier used for processing
    pub privacy_tier: PrivacyMode,

    /// Networks where asset is available
    pub networks: Vec<NetworkId>,

    /// Processing timestamp
    pub processed_at: SystemTime,

    /// Deduplication result
    pub deduplication: DeduplicationResult,

    /// Pipeline statistics
    pub pipeline_stats: PipelineStats,
}

/// Configuration for IntelligenceLayer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntelligenceLayerConfig {
    /// Enable performance monitoring
    pub enable_monitoring: bool,

    /// Maximum concurrent asset processing
    pub max_concurrent_processing: usize,

    /// Asset processing timeout
    pub processing_timeout: Duration,

    /// Retrieval timeout
    pub retrieval_timeout: Duration,

    /// Enable cross-network validation
    pub enable_cross_network_validation: bool,

    /// Default compression level (1-11)
    pub default_compression_level: u32,

    /// Enable quantum-resistant encryption
    pub enable_quantum_encryption: bool,

    /// Reed-Solomon configuration (data shards, parity shards)
    pub sharding_config: (usize, usize),
}

impl Default for IntelligenceLayerConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            max_concurrent_processing: 100,
            processing_timeout: Duration::from_secs(30),
            retrieval_timeout: Duration::from_secs(10),
            enable_cross_network_validation: true,
            default_compression_level: 4,
            enable_quantum_encryption: true,
            sharding_config: (10, 4), // 10 data, 4 parity
        }
    }
}

/// Intelligence layer metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IntelligenceMetrics {
    /// Total assets processed
    pub total_assets_processed: u64,

    /// Total assets retrieved
    pub total_assets_retrieved: u64,

    /// Average processing time (ms)
    pub avg_processing_time_ms: u64,

    /// Average retrieval time (ms)
    pub avg_retrieval_time_ms: u64,

    /// Deduplication rate (0.0 to 1.0)
    pub deduplication_rate: f64,

    /// Cross-network validations
    pub cross_network_validations: u64,

    /// Privacy tier distribution
    pub privacy_tier_distribution: HashMap<String, u64>,

    /// Network participation count
    pub active_networks: usize,

    /// Component health status
    pub component_health: HashMap<String, bool>,
}

/// Inline stub for TrustChainClient (trustchain_stub module removed - was zeroed-key placeholder)
pub(crate) mod inline_trustchain_stub {
    use async_trait::async_trait;
    use crate::assets::multi_node::network_membership::{
        TrustChainClient, NetworkCredentials, NetworkDiscovery,
    };
    use crate::assets::multi_node::NetworkId;
    use crate::assets::core::AssetResult;

    pub struct StubTrustChainClient;

    impl StubTrustChainClient {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl TrustChainClient for StubTrustChainClient {
        async fn request_credentials(&self, _network_id: NetworkId) -> AssetResult<NetworkCredentials> {
            Ok(NetworkCredentials {
                certificate: vec![],
                public_key: vec![],
                private_key_encrypted: vec![],
                session_tokens: vec![],
                expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(86400),
            })
        }

        async fn revoke_credentials(&self, _network_id: NetworkId) -> AssetResult<()> {
            Ok(())
        }

        async fn validate_certificate(&self, _cert: &[u8]) -> AssetResult<bool> {
            Ok(true)
        }

        async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
            Ok(vec![])
        }
    }
}

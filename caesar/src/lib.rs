// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Ephemeral Value Protocol
//!
//! Gold-denominated ephemeral value transfer for HyperMesh. Value exists
//! only in-flight -- born at ingress, dies at egress. Thermodynamic
//! consistency: Input = Output + Transit Fees + Demurrage Decay.
//!
//! **API**: STOQ protocol (HTTP REMOVED)

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use hypermesh_lib::economic::GoldGrams;

// Module declarations -- packet-centric EVP modules
pub mod models;
pub mod storage;
pub mod packet_processor;
pub mod fee_distribution;
pub mod gold_oracle;
pub mod routing;
pub mod holding;

// Legacy banking/cross-chain modules (future sprint update)
pub mod banking_interop_bridge;
pub mod banking_providers;
pub mod crypto_exchange_providers;
pub mod cross_chain_bridge;

// STOQ API layer
pub mod api;

// Core EVP sub-systems
pub mod evp;
pub mod governor;
pub mod upi;
pub mod settlement;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Caesar Protocol Configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaesarConfig {
    /// EVP configuration
    pub evp: evp::EvpConfig,
    /// Storage configuration
    pub storage: storage::StorageConfig,
    /// Gold oracle initial price (USD per troy ounce)
    pub gold_price_usd: Decimal,
    /// Packet processor configuration
    pub processor: packet_processor::ProcessorConfig,
}

impl Default for CaesarConfig {
    fn default() -> Self {
        Self {
            evp: evp::EvpConfig::default(),
            storage: storage::StorageConfig {
                path: "caesar_data".to_string(),
            },
            gold_price_usd: dec!(2350),
            processor: packet_processor::ProcessorConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// CaesarProtocol
// ---------------------------------------------------------------------------

/// Caesar Ephemeral Value Protocol -- the main system entry point.
///
/// Coordinates packet lifecycle, fee distribution, settlement, and
/// the Governor PID controller. No wallets, no token supply, no minting.
pub struct CaesarProtocol {
    #[allow(dead_code)]
    config: Arc<CaesarConfig>,
    storage: Arc<storage::CaesarStorage>,
    #[allow(dead_code)]
    processor: packet_processor::PacketProcessor,
    #[allow(dead_code)]
    fee_distributor: fee_distribution::FeeDistributor,
    oracle: gold_oracle::GoldOracle,
    #[allow(dead_code)]
    router: routing::PacketRouter,
    governor: governor::GovernorPid,
}

impl CaesarProtocol {
    /// Create a new Caesar protocol instance.
    pub async fn new(config: CaesarConfig) -> Result<Self> {
        info!("Initializing Caesar Ephemeral Value Protocol");

        let config = Arc::new(config);
        let storage = Arc::new(
            storage::CaesarStorage::new(config.storage.clone()).await?,
        );
        let processor = packet_processor::PacketProcessor::new(
            config.processor.clone(),
        );
        let fee_distributor = fee_distribution::FeeDistributor::default();
        let oracle = gold_oracle::GoldOracle::new(config.gold_price_usd);
        let router = routing::PacketRouter::default();
        let governor = governor::GovernorPid::new();

        Ok(Self {
            config,
            storage,
            processor,
            fee_distributor,
            oracle,
            router,
            governor,
        })
    }

    /// Get the Governor PID controller (mutable for recalculation).
    pub fn governor_mut(&mut self) -> &mut governor::GovernorPid {
        &mut self.governor
    }

    /// Get the Governor PID controller (read-only).
    pub fn governor(&self) -> &governor::GovernorPid {
        &self.governor
    }

    /// Get the gold oracle for price queries.
    pub fn oracle(&self) -> &gold_oracle::GoldOracle {
        &self.oracle
    }

    /// Get the storage layer.
    pub fn storage(&self) -> &Arc<storage::CaesarStorage> {
        &self.storage
    }

    /// Get active packet count.
    pub async fn active_packet_count(&self) -> Result<usize> {
        self.storage.get_active_packet_count().await
    }

    /// Get total value currently in transit.
    pub async fn in_transit_value(&self) -> Result<GoldGrams> {
        self.storage.get_total_in_transit_value().await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_caesar_protocol_initialization() {
        let config = CaesarConfig::default();
        let protocol = CaesarProtocol::new(config).await;
        assert!(
            protocol.is_ok(),
            "CaesarProtocol should initialize: {:?}",
            protocol.err()
        );
    }

    #[test]
    fn test_default_config() {
        let config = CaesarConfig::default();
        assert_eq!(config.gold_price_usd, dec!(2350));
        assert_eq!(config.processor.default_hop_limit, 32);
    }
}

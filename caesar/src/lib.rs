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

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

// Conservation law (whitepaper section 3.1)
pub mod conservation;

// STOQ API layer
pub mod api;

// CLI module (library, no framework deps)
pub mod cli;

// Core EVP sub-systems
pub mod evp;
pub mod governor;
pub mod upi;
pub mod settlement;

// CaesarProtocol impl and orchestration methods
mod protocol;
mod protocol_settlement;

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
    config: Arc<CaesarConfig>,
    storage: Arc<storage::CaesarStorage>,
    processor: packet_processor::PacketProcessor,
    fee_distributor: fee_distribution::FeeDistributor,
    oracle: gold_oracle::GoldOracle,
    router: routing::PacketRouter,
    governor: governor::GovernorPid,
    holding_buffer: holding::HoldingBuffer,
    conservation: conservation::ConservationLaw,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CaesarConfig::default();
        assert_eq!(config.gold_price_usd, dec!(2350));
        assert_eq!(config.processor.default_hop_limit, 32);
    }
}

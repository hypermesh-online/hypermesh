// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Banking Interoperability Bridge
//!
//! Integration layer for external payment rails and velocity economics.
//!
//! STATUS: PLANNED — velocity zone economics and scoring are functional,
//! but external provider integrations (OpenBanking, Stripe, Plaid, Square)
//! await external prerequisites:
//! - API keys and sandbox credentials for each provider
//! - OAuth2 client implementation (via Gateway HTTP outbound proxy)
//! - PSD2/PCI-DSS compliance review for fiat rail operations
//!
//! WORKING: Velocity zone management, economic adjustment calculation,
//! velocity scoring, fee adjustment, economic health scoring.
//!
//! INTEGRATION DEPENDENCIES:
//! - Gateway: Outbound HTTP proxy for external API calls
//! - TrustChain: FALCON-1024 signed settlement attestations
//! - BlockMatrix: Provider instances registered as assets with Proof of State
//! - Caesar UPI: Each provider implements IngressAdapter + EgressAdapter traits

pub mod operations;
pub mod types;
pub mod validation;

// Re-export all public types for backward compatibility
pub use types::*;

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Banking Interoperability Bridge
pub struct BankingInteropBridge {
    pub(crate) banking_providers: HashMap<BankingProvider, Arc<dyn BankingApiProvider>>,
    pub(crate) crypto_providers: HashMap<CryptoExchange, Arc<dyn CryptoExchangeProvider>>,
    pub(crate) velocity_zones: Arc<RwLock<HashMap<String, VelocityZone>>>,
    pub(crate) active_transactions: Arc<RwLock<HashMap<String, InteropTransaction>>>,
    pub(crate) exchange_rates: Arc<RwLock<HashMap<String, HashMap<String, Decimal>>>>,
}

impl Default for BankingInteropBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl BankingInteropBridge {
    pub fn new() -> Self {
        Self {
            banking_providers: HashMap::new(),
            crypto_providers: HashMap::new(),
            velocity_zones: Arc::new(RwLock::new(Self::default_velocity_zones())),
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            exchange_rates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register banking provider
    pub fn register_banking_provider(
        &mut self,
        provider_type: BankingProvider,
        provider: Arc<dyn BankingApiProvider>,
    ) {
        self.banking_providers.insert(provider_type, provider);
    }

    /// Register crypto exchange provider
    pub fn register_crypto_provider(
        &mut self,
        exchange_type: CryptoExchange,
        provider: Arc<dyn CryptoExchangeProvider>,
    ) {
        self.crypto_providers.insert(exchange_type, provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_market_stabilization_adjustment() {
        let bridge = BankingInteropBridge::new();

        // global_primary: stabilization=0.04, volatility=0, liquidity=-0.5 => -0.46
        // Liquidity depth (2M) dominates, producing a net discount despite slight positive stabilization
        let adjustment = bridge
            .calculate_velocity_adjustment(Some("global_primary"), dec!(1000))
            .await
            .expect("test: global_primary velocity adjustment");
        assert!(
            adjustment < dec!(0),
            "Deep-liquidity primary market gets net discount from liquidity surplus"
        );

        // global_secondary: negative deviation (-0.12), throttle_factor 0.95 < 1.0 => negative stabilization
        let adjustment = bridge
            .calculate_velocity_adjustment(Some("global_secondary"), dec!(1000))
            .await
            .expect("test: global_secondary velocity adjustment");
        assert!(
            adjustment < dec!(0),
            "Below-gold markets should get incentive discounts"
        );

        // global_volatile: deviation 0.19 > 0.16 (80% of 0.20) => positive throttle + high volatility surcharge
        let adjustment = bridge
            .calculate_velocity_adjustment(Some("global_volatile"), dec!(1000))
            .await
            .expect("test: global_volatile velocity adjustment");
        assert!(
            adjustment > dec!(0),
            "Volatile markets near bounds should get throttling fees"
        );

        // global_stable: deviation 0.02 < min_bound 0.05 => small discount (-1.0) + liquidity discount (-0.5)
        let adjustment = bridge
            .calculate_velocity_adjustment(Some("global_stable"), dec!(1000))
            .await
            .expect("test: global_stable velocity adjustment");
        assert!(
            adjustment < dec!(0),
            "Too-stable markets should get activity encouragement"
        );

        // emergency_throttle: deviation -0.22, |0.22| > 0.16, negative => large negative stabilization + liquidity stress
        let adjustment = bridge
            .calculate_velocity_adjustment(Some("emergency_throttle"), dec!(1000))
            .await
            .expect("test: emergency_throttle velocity adjustment");
        assert!(
            adjustment < dec!(-10),
            "Emergency markets should get maximum intervention"
        );
    }

    #[tokio::test]
    async fn test_velocity_score_calculation() {
        let bridge = BankingInteropBridge::new();

        // global_primary: base=40, econ=~227.8, activity=20, decay=7 => ~294.8 => A+
        let score = bridge
            .calculate_velocity_score("global_primary")
            .await
            .expect("test: global_primary velocity score");
        assert_eq!(score.grade, "A+");
        assert!(score.total_score >= dec!(85));
        assert!(score.recommended_fee_adjustment < dec!(0));

        // global_volatile: base=72, econ=~155.5, activity=20, decay=0 => ~247.5 => A+
        // All zones score very high due to the 30x economic multiplier
        let score = bridge
            .calculate_velocity_score("global_volatile")
            .await
            .expect("test: global_volatile velocity score");
        assert_eq!(score.grade, "A+");
        assert!(score.total_score >= dec!(85));
        assert!(score.recommended_fee_adjustment < dec!(0));

        // Verify zone with worst economic health (emergency_throttle) still scores high
        // but lower than primary: base=12, econ=~142.4, activity=20, decay=0 => ~174.4
        let emergency = bridge
            .calculate_velocity_score("emergency_throttle")
            .await
            .expect("test: emergency_throttle velocity score");
        assert!(
            emergency.total_score < score.total_score,
            "Emergency zone should score lower than volatile zone"
        );

        // Nonexistent zone returns error
        assert!(
            bridge
                .calculate_velocity_score("nonexistent_zone")
                .await
                .is_err(),
            "Nonexistent zone should return error"
        );
    }

    #[tokio::test]
    async fn test_gold_price_adjustment_calculation() {
        let bridge = BankingInteropBridge::new();

        let above_gold_indicators = EconomicIndicators {
            current_gold_price_usd: dec!(3000),
            target_gold_price_usd: dec!(2600),
            market_volatility: dec!(0.2),
            transaction_volume: dec!(500000),
            liquidity_depth: dec!(1500000),
        };
        let adjustment = bridge.calculate_economic_adjustment(&above_gold_indicators, dec!(1000));
        assert!(
            adjustment > dec!(0),
            "Above-gold prices should get throttling fees"
        );

        let below_gold_indicators = EconomicIndicators {
            current_gold_price_usd: dec!(2200),
            target_gold_price_usd: dec!(2600),
            market_volatility: dec!(0.2),
            transaction_volume: dec!(300000),
            liquidity_depth: dec!(800000),
        };
        let adjustment = bridge.calculate_economic_adjustment(&below_gold_indicators, dec!(1000));
        assert!(
            adjustment < dec!(0),
            "Below-gold prices should get incentive discounts"
        );

        let emergency_indicators = EconomicIndicators {
            current_gold_price_usd: dec!(3100),
            target_gold_price_usd: dec!(2600),
            market_volatility: dec!(0.4),
            transaction_volume: dec!(2000000),
            liquidity_depth: dec!(200000),
        };
        let adjustment = bridge.calculate_economic_adjustment(&emergency_indicators, dec!(1000));
        assert!(
            adjustment > dec!(15),
            "Emergency deviation should trigger heavy throttling"
        );
    }

    #[test]
    fn test_global_market_stabilization_zones() {
        let zones = BankingInteropBridge::default_velocity_zones();

        assert!(zones.contains_key("global_primary"));
        assert!(zones.contains_key("global_secondary"));
        assert!(zones.contains_key("global_volatile"));
        assert!(zones.contains_key("global_stable"));
        assert!(zones.contains_key("emergency_throttle"));

        let primary = &zones["global_primary"];
        let secondary = &zones["global_secondary"];
        let volatile = &zones["global_volatile"];
        let stable = &zones["global_stable"];
        let emergency = &zones["emergency_throttle"];

        assert!(
            primary.stability_deviation.abs() <= dec!(0.20),
            "Primary market within bounds"
        );
        assert!(
            secondary.stability_deviation.abs() <= dec!(0.20),
            "Secondary market within bounds"
        );
        assert!(
            volatile.stability_deviation.abs() >= dec!(0.15),
            "Volatile market near upper bound"
        );
        assert!(
            stable.stability_deviation.abs() <= dec!(0.05),
            "Stable market very low deviation"
        );
        assert!(
            emergency.stability_deviation.abs() > dec!(0.20),
            "Emergency market beyond bounds"
        );

        assert!(
            volatile.throttle_factor > dec!(1.0),
            "Volatile market has throttling"
        );
        assert!(
            secondary.throttle_factor < dec!(1.0),
            "Secondary market has incentives"
        );
        assert!(
            emergency.throttle_factor < dec!(0.6),
            "Emergency market has maximum intervention"
        );
    }

    #[test]
    fn test_score_to_grade_conversion() {
        let bridge = BankingInteropBridge::new();

        assert_eq!(bridge.score_to_grade(dec!(90)), "A+");
        assert_eq!(bridge.score_to_grade(dec!(82)), "A");
        assert_eq!(bridge.score_to_grade(dec!(77)), "A-");
        assert_eq!(bridge.score_to_grade(dec!(67)), "B");
        assert_eq!(bridge.score_to_grade(dec!(52)), "C");
        assert_eq!(bridge.score_to_grade(dec!(42)), "D");
        assert_eq!(bridge.score_to_grade(dec!(30)), "F");
    }

    #[test]
    fn test_fee_adjustment_ranges() {
        let bridge = BankingInteropBridge::new();

        let adjustment = bridge.score_to_fee_adjustment(dec!(90));
        assert_eq!(adjustment, dec!(-0.008));

        let adjustment = bridge.score_to_fee_adjustment(dec!(30));
        assert_eq!(adjustment, dec!(0.005));

        let adjustment = bridge.score_to_fee_adjustment(dec!(50));
        assert_eq!(adjustment, dec!(0));
    }

    #[tokio::test]
    async fn test_bridge_transaction_creation() {
        let bridge = BankingInteropBridge::new();

        let transactions = bridge.list_active_transactions().await.unwrap();
        assert_eq!(transactions.len(), 0);
    }

    #[test]
    fn test_economic_health_score() {
        let bridge = BankingInteropBridge::new();

        // Perfect: gold_dev=0 => gold_score=10, vol_score=9.5, volume=1.0(capped), liq=10(capped)
        // health = 10*0.4 + 9.5*0.3 + 1.0*0.2 + 10*0.1 = 4+2.85+0.2+1.0 = 8.05
        let perfect_indicators = EconomicIndicators {
            current_gold_price_usd: dec!(2000),
            target_gold_price_usd: dec!(2000),
            market_volatility: dec!(0.05),
            transaction_volume: dec!(1000000),
            liquidity_depth: dec!(5000000),
        };
        let score = bridge.calculate_economic_health_score(&perfect_indicators);
        assert!(score > dec!(8), "Perfect indicators should score highly");

        // Truly poor: gold_dev=0.8 => gold_score=2.0, vol=0.9 => vol_score=1.0,
        // volume=1000 => 0.001, liq=10000 => 0.1
        // health = 2.0*0.4 + 1.0*0.3 + 0.001*0.2 + 0.1*0.1 = 0.8+0.3+0.0002+0.01 = 1.1102
        let poor_indicators = EconomicIndicators {
            current_gold_price_usd: dec!(4500),
            target_gold_price_usd: dec!(2500),
            market_volatility: dec!(0.9),
            transaction_volume: dec!(1000),
            liquidity_depth: dec!(10000),
        };
        let score = bridge.calculate_economic_health_score(&poor_indicators);
        assert!(score < dec!(2), "Poor indicators should score low");
        assert!(score > dec!(0), "Score should remain positive");
    }
}

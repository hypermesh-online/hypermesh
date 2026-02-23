// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Banking interoperability bridge validation - velocity economics, scoring, and rate calculations

use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use super::types::*;
use super::BankingInteropBridge;

impl BankingInteropBridge {
    /// Calculate market stabilization adjustment to maintain 5-20% gold deviation
    pub(crate) async fn calculate_velocity_adjustment(
        &self,
        velocity_zone: Option<&str>,
        amount: Decimal
    ) -> Result<Decimal> {
        let zones = self.velocity_zones.read().await;

        let adjustment = if let Some(zone_id) = velocity_zone {
            if let Some(zone) = zones.get(zone_id) {
                let current_deviation = zone.stability_deviation;
                let throttle_factor = zone.throttle_factor;
                let (min_bound, max_bound) = zone.target_stability_range;

                let stabilization_adjustment = if current_deviation.abs() > max_bound * dec!(0.8) {
                    let severity = (current_deviation.abs() - max_bound * dec!(0.8)) / (max_bound * dec!(0.2));
                    let throttle_rate = severity * dec!(0.015);

                    if current_deviation > dec!(0) {
                        amount * throttle_rate
                    } else {
                        amount * throttle_rate * dec!(-1)
                    }
                } else if current_deviation.abs() < min_bound {
                    amount * dec!(-0.001)
                } else {
                    amount * (throttle_factor - dec!(1)) * dec!(0.002)
                };

                let volatility_adjustment = self.calculate_volatility_adjustment(&zone.location_data.economic_indicators, amount);
                let liquidity_adjustment = self.calculate_liquidity_adjustment(&zone.location_data.economic_indicators, amount);

                stabilization_adjustment + volatility_adjustment + liquidity_adjustment
            } else {
                dec!(0)
            }
        } else {
            self.calculate_global_stabilization_adjustment(amount).await
        };

        Ok(adjustment.clamp(amount * dec!(-0.02), amount * dec!(0.02)))
    }

    /// Calculate volatility-based adjustment for market stability
    fn calculate_volatility_adjustment(&self, indicators: &EconomicIndicators, amount: Decimal) -> Decimal {
        let volatility = indicators.market_volatility;

        if volatility > dec!(0.3) {
            amount * volatility * dec!(0.005)
        } else if volatility < dec!(0.1) {
            amount * dec!(-0.001)
        } else {
            dec!(0)
        }
    }

    /// Calculate liquidity-based adjustment
    fn calculate_liquidity_adjustment(&self, indicators: &EconomicIndicators, amount: Decimal) -> Decimal {
        let liquidity_depth = indicators.liquidity_depth;

        if liquidity_depth < dec!(100000) {
            let liquidity_stress = (dec!(100000) - liquidity_depth) / dec!(100000);
            amount * liquidity_stress * dec!(0.01)
        } else if liquidity_depth > dec!(1000000) {
            amount * dec!(-0.0005)
        } else {
            dec!(0)
        }
    }

    /// Calculate global market stabilization when no zone specified
    async fn calculate_global_stabilization_adjustment(&self, amount: Decimal) -> Decimal {
        let current_gold_price = dec!(85.2);
        let target_gold_price = dec!(84.0);
        let global_deviation = (current_gold_price - target_gold_price) / target_gold_price;

        if global_deviation.abs() > dec!(0.15) {
            let throttle_rate = (global_deviation.abs() - dec!(0.05)) * dec!(0.02);

            if global_deviation > dec!(0) {
                amount * throttle_rate
            } else {
                amount * throttle_rate * dec!(-1)
            }
        } else {
            amount * global_deviation * dec!(0.001)
        }
    }

    /// Calculate gold price deviation-based adjustment for market stability
    #[cfg(test)]
    pub(crate) fn calculate_economic_adjustment(&self, indicators: &EconomicIndicators, amount: Decimal) -> Decimal {
        let current_gold = indicators.current_gold_price_usd;
        let target_gold = indicators.target_gold_price_usd;
        let price_deviation = (current_gold - target_gold) / target_gold;

        let volume_adjustment = if indicators.transaction_volume > dec!(1000000) {
            let volume_factor = (indicators.transaction_volume - dec!(500000)) / dec!(1000000);
            amount * volume_factor * dec!(0.003)
        } else if indicators.transaction_volume < dec!(100000) {
            amount * dec!(-0.001)
        } else {
            dec!(0)
        };

        let deviation_adjustment = if price_deviation.abs() > dec!(0.18) {
            let emergency_factor = (price_deviation.abs() - dec!(0.18)) / dec!(0.02);
            let throttle_rate = emergency_factor * dec!(0.02);

            if price_deviation > dec!(0) {
                amount * throttle_rate
            } else {
                amount * throttle_rate * dec!(-1)
            }
        } else if price_deviation.abs() > dec!(0.1) {
            let moderate_factor = (price_deviation.abs() - dec!(0.05)) / dec!(0.05);
            let throttle_rate = moderate_factor * dec!(0.005);

            if price_deviation > dec!(0) {
                amount * throttle_rate
            } else {
                amount * throttle_rate * dec!(-1)
            }
        } else if price_deviation.abs() < dec!(0.03) {
            amount * dec!(-0.0005)
        } else {
            amount * price_deviation * dec!(0.001)
        };

        volume_adjustment + deviation_adjustment
    }

    /// Calculate comprehensive velocity economics score for a zone
    pub async fn calculate_velocity_score(&self, zone_id: &str) -> Result<VelocityScore> {
        let zones = self.velocity_zones.read().await;

        let zone = zones.get(zone_id)
            .ok_or_else(|| anyhow!("Velocity zone not found: {}", zone_id))?;

        let base_score = zone.market_velocity * dec!(40);
        let economic_score = self.calculate_economic_health_score(&zone.location_data.economic_indicators) * dec!(30);

        let activity_score = if zone.location_data.economic_indicators.transaction_volume > dec!(500) {
            dec!(20)
        } else {
            (zone.location_data.economic_indicators.transaction_volume / dec!(500)) * dec!(20)
        };

        let decay_score = (dec!(0.15) - zone.stability_deviation.abs()).max(dec!(0)) * dec!(100);
        let total_score = base_score + economic_score + activity_score + decay_score;

        Ok(VelocityScore {
            zone_id: zone_id.to_string(),
            total_score,
            base_velocity_component: base_score,
            economic_component: economic_score,
            activity_component: activity_score,
            decay_component: decay_score,
            grade: self.score_to_grade(total_score),
            recommended_fee_adjustment: self.score_to_fee_adjustment(total_score),
        })
    }

    /// Calculate economic health score from indicators
    pub(crate) fn calculate_economic_health_score(&self, indicators: &EconomicIndicators) -> Decimal {
        let gold_deviation = ((indicators.current_gold_price_usd - indicators.target_gold_price_usd) / indicators.target_gold_price_usd).abs();
        let gold_score = (dec!(1) - gold_deviation).max(dec!(0)) * dec!(10);
        let volatility_score = (dec!(1) - indicators.market_volatility).max(dec!(0)) * dec!(10);
        let volume_score = (indicators.transaction_volume / dec!(1000000)).min(dec!(10));
        let liquidity_score = (indicators.liquidity_depth / dec!(100000)).min(dec!(10));
        let _col_score = dec!(10);

        gold_score * dec!(0.4) + volatility_score * dec!(0.3) + volume_score * dec!(0.2) + liquidity_score * dec!(0.1)
    }

    /// Convert velocity score to letter grade
    pub(crate) fn score_to_grade(&self, score: Decimal) -> String {
        if score >= dec!(85) { "A+".to_string() }
        else if score >= dec!(80) { "A".to_string() }
        else if score >= dec!(75) { "A-".to_string() }
        else if score >= dec!(70) { "B+".to_string() }
        else if score >= dec!(65) { "B".to_string() }
        else if score >= dec!(60) { "B-".to_string() }
        else if score >= dec!(55) { "C+".to_string() }
        else if score >= dec!(50) { "C".to_string() }
        else if score >= dec!(45) { "C-".to_string() }
        else if score >= dec!(40) { "D".to_string() }
        else { "F".to_string() }
    }

    /// Convert velocity score to recommended fee adjustment
    pub(crate) fn score_to_fee_adjustment(&self, score: Decimal) -> Decimal {
        if score >= dec!(85) { dec!(-0.008) }
        else if score >= dec!(75) { dec!(-0.006) }
        else if score >= dec!(65) { dec!(-0.004) }
        else if score >= dec!(55) { dec!(-0.002) }
        else if score >= dec!(50) { dec!(0) }
        else if score >= dec!(40) { dec!(0.002) }
        else { dec!(0.005) }
    }

    /// Get dynamic exchange rate from oracles
    pub(crate) async fn get_crypto_exchange_rate(&self, from: &str, to: &str) -> Result<Decimal> {
        {
            let rates = self.exchange_rates.read().await;
            if let Some(from_rates) = rates.get(from) {
                if let Some(&_rate) = from_rates.get(to) {
                    // In production, check if cached rate is still fresh
                }
            }
        }

        let rate = match (from, to) {
            ("USD", "CSR") => {
                let _gold_price_per_gram = self.fetch_gold_price_oracle().await?;
                let caesar_market_price = self.fetch_caesar_market_price().await?;
                dec!(1) / caesar_market_price
            },
            ("CSR", "USD") => {
                self.fetch_caesar_market_price().await?
            },
            ("CSR", "GOLD_GRAM") => {
                let caesar_usd = self.fetch_caesar_market_price().await?;
                let gold_usd = self.fetch_gold_price_oracle().await?;
                caesar_usd / gold_usd
            },
            ("GOLD_GRAM", "CSR") => {
                let caesar_usd = self.fetch_caesar_market_price().await?;
                let gold_usd = self.fetch_gold_price_oracle().await?;
                gold_usd / caesar_usd
            },
            ("USD", "ETH") | ("ETH", "USD") => {
                return Err(anyhow!("ETH oracle not implemented - requires Chainlink integration"));
            },
            _ => return Err(anyhow!("Unsupported currency pair: {} -> {}", from, to)),
        };

        {
            let mut rates = self.exchange_rates.write().await;
            rates.entry(from.to_string())
                .or_insert_with(HashMap::new)
                .insert(to.to_string(), rate);
        }

        Ok(rate)
    }

    /// Fetch current gold price per gram from oracle
    async fn fetch_gold_price_oracle(&self) -> Result<Decimal> {
        Err(anyhow!("Gold price oracle not implemented - requires precious metals data feed"))
    }

    /// Fetch current Caesar market price from DEX/exchanges
    async fn fetch_caesar_market_price(&self) -> Result<Decimal> {
        Err(anyhow!("Caesar market price not implemented - requires DEX/CEX integration"))
    }

    /// Default market stabilization zones for global economy
    pub(crate) fn default_velocity_zones() -> HashMap<String, VelocityZone> {
        let mut zones = HashMap::new();

        zones.insert("global_primary".to_string(), VelocityZone {
            zone_id: "global_primary".to_string(),
            name: "Global Primary Market".to_string(),
            market_velocity: dec!(1.0),
            stability_deviation: dec!(0.08),
            throttle_factor: dec!(1.02),
            target_stability_range: (dec!(0.05), dec!(0.20)),
            location_data: LocationData {
                country: "GLOBAL".to_string(),
                region: "PRIMARY".to_string(),
                city: None,
                economic_indicators: EconomicIndicators {
                    current_gold_price_usd: dec!(85.2),
                    target_gold_price_usd: dec!(84.0),
                    market_volatility: dec!(0.15),
                    transaction_volume: dec!(500000),
                    liquidity_depth: dec!(2000000),
                },
            },
        });

        zones.insert("global_secondary".to_string(), VelocityZone {
            zone_id: "global_secondary".to_string(),
            name: "Global Secondary Market".to_string(),
            market_velocity: dec!(0.85),
            stability_deviation: dec!(-0.12),
            throttle_factor: dec!(0.95),
            target_stability_range: (dec!(0.05), dec!(0.20)),
            location_data: LocationData {
                country: "GLOBAL".to_string(),
                region: "SECONDARY".to_string(),
                city: None,
                economic_indicators: EconomicIndicators {
                    current_gold_price_usd: dec!(74.0),
                    target_gold_price_usd: dec!(84.0),
                    market_volatility: dec!(0.25),
                    transaction_volume: dec!(200000),
                    liquidity_depth: dec!(800000),
                },
            },
        });

        zones.insert("global_volatile".to_string(), VelocityZone {
            zone_id: "global_volatile".to_string(),
            name: "Global Volatile Market".to_string(),
            market_velocity: dec!(1.8),
            stability_deviation: dec!(0.19),
            throttle_factor: dec!(1.15),
            target_stability_range: (dec!(0.05), dec!(0.20)),
            location_data: LocationData {
                country: "GLOBAL".to_string(),
                region: "VOLATILE".to_string(),
                city: None,
                economic_indicators: EconomicIndicators {
                    current_gold_price_usd: dec!(100.0),
                    target_gold_price_usd: dec!(84.0),
                    market_volatility: dec!(0.45),
                    transaction_volume: dec!(2000000),
                    liquidity_depth: dec!(5000000),
                },
            },
        });

        zones.insert("global_stable".to_string(), VelocityZone {
            zone_id: "global_stable".to_string(),
            name: "Global Stable Market".to_string(),
            market_velocity: dec!(1.05),
            stability_deviation: dec!(0.02),
            throttle_factor: dec!(0.98),
            target_stability_range: (dec!(0.05), dec!(0.20)),
            location_data: LocationData {
                country: "GLOBAL".to_string(),
                region: "STABLE".to_string(),
                city: None,
                economic_indicators: EconomicIndicators {
                    current_gold_price_usd: dec!(85.7),
                    target_gold_price_usd: dec!(84.0),
                    market_volatility: dec!(0.08),
                    transaction_volume: dec!(450000),
                    liquidity_depth: dec!(1500000),
                },
            },
        });

        zones.insert("emergency_throttle".to_string(), VelocityZone {
            zone_id: "emergency_throttle".to_string(),
            name: "Emergency Market Intervention".to_string(),
            market_velocity: dec!(0.3),
            stability_deviation: dec!(-0.22),
            throttle_factor: dec!(0.5),
            target_stability_range: (dec!(0.05), dec!(0.20)),
            location_data: LocationData {
                country: "GLOBAL".to_string(),
                region: "EMERGENCY".to_string(),
                city: None,
                economic_indicators: EconomicIndicators {
                    current_gold_price_usd: dec!(64.0),
                    target_gold_price_usd: dec!(84.0),
                    market_volatility: dec!(0.8),
                    transaction_volume: dec!(5000000),
                    liquidity_depth: dec!(100000),
                },
            },
        });

        zones
    }
}

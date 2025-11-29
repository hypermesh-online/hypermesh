//! Pricing configuration and models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::assets::core::AssetType;

/// Pricing configuration for resource sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfiguration {
    pub base_prices: HashMap<AssetType, PriceModel>,
    pub dynamic_pricing: DynamicPricingConfig,
    pub payment_preferences: PaymentPreferences,
    pub discount_settings: DiscountSettings,
}

/// Price model for resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceModel {
    pub base_price: f64,
    pub currency: Currency,
    pub minimum_price: f64,
    pub peak_multiplier: f64,
    pub volume_tiers: Vec<VolumeTier>,
}

/// Supported currencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    CaesarTokens,
    HyperMeshCredits,
    Custom(String),
}

/// Volume discount tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeTier {
    pub min_hours: u64,
    pub discount_percentage: f64,
}

/// Dynamic pricing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPricingConfig {
    pub enabled: bool,
    pub demand_multiplier: f64,
    pub supply_multiplier: f64,
    pub reputation_bonus: f64,
    pub performance_bonus: f64,
    pub update_frequency: Duration,
}

/// Payment preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPreferences {
    pub preferred_currency: Currency,
    pub payment_frequency: PaymentFrequency,
    pub minimum_payout: f64,
    pub auto_reinvest_percentage: f64,
    pub tax_reporting: TaxReporting,
}

/// Payment frequencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Immediate,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
}

/// Tax reporting settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxReporting {
    pub enabled: bool,
    pub tax_jurisdiction: String,
    pub tax_rate: f64,
    pub reporting_format: String,
}

/// Discount settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountSettings {
    pub loyalty_discounts: Vec<LoyaltyDiscount>,
    pub referral_bonuses: ReferralBonus,
    pub seasonal_promotions: Vec<SeasonalPromotion>,
}

/// Loyalty discount tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyDiscount {
    pub min_reputation: f64,
    pub discount_percentage: f64,
    pub additional_benefits: Vec<String>,
}

/// Referral bonus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralBonus {
    pub enabled: bool,
    pub bonus_amount: f64,
    pub bonus_currency: Currency,
    pub max_referrals: Option<u32>,
}

/// Seasonal promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPromotion {
    pub name: String,
    pub start_date: std::time::SystemTime,
    pub end_date: std::time::SystemTime,
    pub discount_percentage: f64,
    pub applicable_resources: Vec<AssetType>,
}

impl Default for PricingConfiguration {
    fn default() -> Self {
        let mut base_prices = HashMap::new();
        base_prices.insert(AssetType::Cpu, PriceModel {
            base_price: 0.10,
            currency: Currency::CaesarTokens,
            minimum_price: 0.01,
            peak_multiplier: 1.5,
            volume_tiers: vec![
                VolumeTier { min_hours: 10, discount_percentage: 5.0 },
                VolumeTier { min_hours: 100, discount_percentage: 10.0 },
                VolumeTier { min_hours: 1000, discount_percentage: 15.0 },
            ],
        });

        Self {
            base_prices,
            dynamic_pricing: DynamicPricingConfig {
                enabled: true,
                demand_multiplier: 1.2,
                supply_multiplier: 0.8,
                reputation_bonus: 1.1,
                performance_bonus: 1.05,
                update_frequency: Duration::from_secs(300),
            },
            payment_preferences: PaymentPreferences {
                preferred_currency: Currency::CaesarTokens,
                payment_frequency: PaymentFrequency::Daily,
                minimum_payout: 1.0,
                auto_reinvest_percentage: 0.0,
                tax_reporting: TaxReporting {
                    enabled: false,
                    tax_jurisdiction: "".to_string(),
                    tax_rate: 0.0,
                    reporting_format: "".to_string(),
                },
            },
            discount_settings: DiscountSettings {
                loyalty_discounts: vec![],
                referral_bonuses: ReferralBonus {
                    enabled: true,
                    bonus_amount: 10.0,
                    bonus_currency: Currency::CaesarTokens,
                    max_referrals: Some(10),
                },
                seasonal_promotions: vec![],
            },
        }
    }
}

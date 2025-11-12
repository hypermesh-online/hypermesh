//! Cross-Node Resource Sharing and Trading
//!
//! Implements resource sharing protocols, pricing models, and secure
//! resource trading between HyperMesh nodes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::assets::core::{AssetType, AssetResult, AssetError, PrivacyLevel};
use super::{NodeId, ResourceAmount, ServiceLevelAgreement};

/// Resource sharing manager
pub struct ResourceSharing {
    /// Active sharing agreements
    agreements: Arc<RwLock<HashMap<String, SharingAgreement>>>,
    /// Resource offers
    offers: Arc<RwLock<Vec<ResourceOffer>>>,
    /// Resource requests
    requests: Arc<RwLock<Vec<ResourceRequest>>>,
    /// Sharing protocol
    protocol: SharingProtocol,
    /// Pricing model
    pricing_model: PricingModel,
    /// Configuration
    config: SharingConfig,
}

/// Sharing configuration
#[derive(Clone, Debug)]
pub struct SharingConfig {
    /// Enable automatic matching
    pub auto_matching: bool,
    /// Matching interval
    pub matching_interval: Duration,
    /// Minimum agreement duration
    pub min_duration: Duration,
    /// Maximum agreement duration
    pub max_duration: Duration,
    /// Enable pricing
    pub pricing_enabled: bool,
    /// Commission rate
    pub commission_rate: f64,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            auto_matching: true,
            matching_interval: Duration::from_secs(30),
            min_duration: Duration::from_secs(60),
            max_duration: Duration::from_secs(86400), // 24 hours
            pricing_enabled: false,
            commission_rate: 0.05, // 5%
        }
    }
}

/// Resource sharing protocol
#[derive(Clone, Debug)]
pub enum SharingProtocol {
    /// Direct peer-to-peer sharing
    P2P,
    /// Market-based trading
    Market,
    /// Auction-based allocation
    Auction,
    /// Cooperative sharing
    Cooperative,
}

/// Resource pricing model
#[derive(Clone, Debug)]
pub enum PricingModel {
    /// Fixed pricing
    Fixed,
    /// Dynamic market-based pricing
    Dynamic,
    /// Auction-based pricing
    Auction,
    /// Usage-based pricing
    UsageBased,
    /// Free tier with limits
    Freemium,
}

/// Sharing agreement between nodes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingAgreement {
    /// Agreement ID
    pub agreement_id: String,
    /// Provider node
    pub provider: NodeId,
    /// Consumer node
    pub consumer: NodeId,
    /// Resource type
    pub resource_type: AssetType,
    /// Resource amount
    pub amount: ResourceAmount,
    /// Price per unit per hour
    pub price_per_hour: f64,
    /// Service level agreement
    pub sla: ServiceLevelAgreement,
    /// Agreement start time
    pub start_time: SystemTime,
    /// Agreement duration
    pub duration: Duration,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Agreement status
    pub status: AgreementStatus,
}

/// Agreement status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AgreementStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Disputed,
}

/// Resource offer from provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceOffer {
    /// Offer ID
    pub offer_id: String,
    /// Provider node
    pub provider: NodeId,
    /// Resource type
    pub resource_type: AssetType,
    /// Available amount
    pub available_amount: ResourceAmount,
    /// Price per unit per hour
    pub price_per_hour: f64,
    /// Minimum commitment
    pub min_commitment: Duration,
    /// Maximum commitment
    pub max_commitment: Duration,
    /// Service level
    pub sla: ServiceLevelAgreement,
    /// Offer expiry
    pub expires_at: SystemTime,
    /// Privacy requirements
    pub privacy_requirements: PrivacyLevel,
}

/// Resource request from consumer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceRequest {
    /// Request ID
    pub request_id: String,
    /// Consumer node
    pub consumer: NodeId,
    /// Resource type
    pub resource_type: AssetType,
    /// Requested amount
    pub requested_amount: ResourceAmount,
    /// Maximum price willing to pay
    pub max_price_per_hour: f64,
    /// Required duration
    pub duration: Duration,
    /// Required service level
    pub required_sla: ServiceLevelAgreement,
    /// Request expiry
    pub expires_at: SystemTime,
    /// Privacy requirements
    pub privacy_requirements: PrivacyLevel,
}

/// Resource usage tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsageRecord {
    /// Agreement ID
    pub agreement_id: String,
    /// Usage period start
    pub period_start: SystemTime,
    /// Usage period end
    pub period_end: SystemTime,
    /// Amount used
    pub amount_used: ResourceAmount,
    /// Cost incurred
    pub cost: f64,
    /// Performance metrics
    pub performance: PerformanceMetrics,
}

/// Performance metrics for resource usage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Uptime percentage
    pub uptime_percentage: f32,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Throughput (ops/sec)
    pub throughput_ops: f64,
    /// SLA violations
    pub sla_violations: u32,
}

impl ResourceSharing {
    /// Create new resource sharing manager
    pub fn new(protocol: SharingProtocol, pricing_model: PricingModel, config: SharingConfig) -> Self {
        Self {
            agreements: Arc::new(RwLock::new(HashMap::new())),
            offers: Arc::new(RwLock::new(Vec::new())),
            requests: Arc::new(RwLock::new(Vec::new())),
            protocol,
            pricing_model,
            config,
        }
    }

    /// Submit resource offer
    pub async fn submit_offer(&self, offer: ResourceOffer) -> AssetResult<()> {
        self.offers.write().await.push(offer);

        if self.config.auto_matching {
            self.match_offers_and_requests().await?;
        }

        Ok(())
    }

    /// Submit resource request
    pub async fn submit_request(&self, request: ResourceRequest) -> AssetResult<()> {
        self.requests.write().await.push(request);

        if self.config.auto_matching {
            self.match_offers_and_requests().await?;
        }

        Ok(())
    }

    /// Match offers with requests
    async fn match_offers_and_requests(&self) -> AssetResult<()> {
        let offers = self.offers.read().await;
        let mut requests = self.requests.write().await;

        for offer in offers.iter() {
            // Find matching request
            let matching_request = requests.iter()
                .position(|req| {
                    req.resource_type == offer.resource_type &&
                    req.max_price_per_hour >= offer.price_per_hour &&
                    req.expires_at > SystemTime::now() &&
                    offer.expires_at > SystemTime::now()
                });

            if let Some(idx) = matching_request {
                let request = requests.remove(idx);

                // Create agreement
                let agreement = SharingAgreement {
                    agreement_id: format!("agr_{}", uuid::Uuid::new_v4()),
                    provider: offer.provider.clone(),
                    consumer: request.consumer.clone(),
                    resource_type: offer.resource_type,
                    amount: request.requested_amount.clone(),
                    price_per_hour: offer.price_per_hour,
                    sla: offer.sla.clone(),
                    start_time: SystemTime::now(),
                    duration: request.duration,
                    privacy_level: request.privacy_requirements,
                    status: AgreementStatus::Active,
                };

                self.agreements.write().await.insert(
                    agreement.agreement_id.clone(),
                    agreement,
                );
            }
        }

        Ok(())
    }

    /// Calculate price for resource
    pub async fn calculate_price(
        &self,
        resource_type: AssetType,
        amount: &ResourceAmount,
        duration: Duration,
    ) -> f64 {
        let base_price = match resource_type {
            AssetType::Cpu => 0.10,
            AssetType::Memory => 0.05,
            AssetType::Gpu => 0.50,
            AssetType::Storage => 0.02,
            _ => 0.01,
        };

        let amount_multiplier = match amount {
            ResourceAmount::CpuCores(cores) => *cores as f64,
            ResourceAmount::MemoryBytes(bytes) => (*bytes as f64) / (1024.0 * 1024.0 * 1024.0), // GB
            ResourceAmount::GpuUnits(units) => *units as f64,
            ResourceAmount::StorageBytes(bytes) => (*bytes as f64) / (1024.0 * 1024.0 * 1024.0), // GB
            ResourceAmount::BandwidthMbps(mbps) => (*mbps as f64) / 1000.0, // Gbps
        };

        let hours = duration.as_secs_f64() / 3600.0;

        match self.pricing_model {
            PricingModel::Fixed => base_price * amount_multiplier * hours,
            PricingModel::Dynamic => {
                // Dynamic pricing based on demand
                let demand_factor = 1.5; // Would be calculated from market conditions
                base_price * amount_multiplier * hours * demand_factor
            }
            PricingModel::UsageBased => {
                // Pay only for what you use
                base_price * amount_multiplier * hours * 0.8
            }
            _ => base_price * amount_multiplier * hours,
        }
    }

    /// Get active agreements
    pub async fn get_active_agreements(&self) -> Vec<SharingAgreement> {
        self.agreements.read().await
            .values()
            .filter(|a| a.status == AgreementStatus::Active)
            .cloned()
            .collect()
    }

    /// Cancel agreement
    pub async fn cancel_agreement(&self, agreement_id: &str) -> AssetResult<()> {
        let mut agreements = self.agreements.write().await;

        let agreement = agreements.get_mut(agreement_id)
            .ok_or_else(|| AssetError::NotFound {
                resource: agreement_id.to_string(),
            })?;

        agreement.status = AgreementStatus::Cancelled;

        Ok(())
    }

    /// Record resource usage
    pub async fn record_usage(&self, usage: ResourceUsageRecord) -> AssetResult<()> {
        // In production, this would update billing and metrics
        tracing::info!(
            "Recorded usage for agreement {}: {:?} units, cost: ${:.2}",
            usage.agreement_id,
            usage.amount_used,
            usage.cost
        );

        Ok(())
    }
}
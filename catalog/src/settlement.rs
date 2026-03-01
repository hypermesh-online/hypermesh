// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar settlement integration for type definition contribution rewards.
//!
//! Type definitions registered in the catalog are blockchain assets. Publishers
//! who contribute quality typedefs earn CAES rewards through Caesar's EVP
//! (Economic Value Protocol) settlement pipeline.
//!
//! This module provides:
//! - `CatalogRewardAdapter`: EgressAdapter for distributing CAES rewards
//! - `ContributionTracker`: Tracks typedef contributions per publisher
//! - `RewardService`: Calculates and distributes contribution-based rewards

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use caesar::upi::{EgressAdapter, SettlementFinality, SettlementReceipt, UpiError};
use hypermesh_lib::{GoldGrams, NodeId};

// ---------------------------------------------------------------------------
// Contribution tracking
// ---------------------------------------------------------------------------

/// Contribution metrics for a typedef publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionMetrics {
    /// Publisher node ID
    pub publisher_id: String,
    /// Number of type definitions published
    pub typedefs_published: u64,
    /// Number of times their typedefs have been referenced/installed
    pub typedef_references: u64,
    /// Number of successful validations against their schemas
    pub successful_validations: u64,
    /// Number of failed validations (quality penalty)
    pub failed_validations: u64,
    /// Schema maintenance score (updates, fixes, deprecation handling)
    pub maintenance_score: f64,
    /// Last contribution timestamp
    pub last_contribution: chrono::DateTime<Utc>,
}

impl ContributionMetrics {
    /// Create new metrics for a publisher.
    fn new(publisher_id: String) -> Self {
        Self {
            publisher_id,
            typedefs_published: 0,
            typedef_references: 0,
            successful_validations: 0,
            failed_validations: 0,
            maintenance_score: 0.5, // Neutral starting score
            last_contribution: Utc::now(),
        }
    }

    /// Calculate contribution score (0.0 - 1.0).
    ///
    /// Weighted formula:
    /// - 30% published count (log-scaled)
    /// - 30% reference/adoption rate
    /// - 25% validation success rate
    /// - 15% maintenance score
    pub fn contribution_score(&self) -> f64 {
        let pub_score = (1.0 + self.typedefs_published as f64).ln() / 5.0;
        let ref_score = (self.typedef_references as f64).min(1000.0) / 1000.0;
        let total_validations = self.successful_validations + self.failed_validations;
        let validation_score = if total_validations > 0 {
            self.successful_validations as f64 / total_validations as f64
        } else {
            0.5 // Neutral if no validations yet
        };

        let score = pub_score * 0.30
            + ref_score * 0.30
            + validation_score * 0.25
            + self.maintenance_score * 0.15;

        score.clamp(0.0, 1.0)
    }
}

/// Tracks typedef contributions per publisher.
pub struct ContributionTracker {
    metrics: Arc<RwLock<HashMap<String, ContributionMetrics>>>,
}

impl Default for ContributionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ContributionTracker {
    /// Create new contribution tracker.
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a typedef publication.
    pub async fn record_publication(&self, publisher_id: &str) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics
            .entry(publisher_id.to_string())
            .or_insert_with(|| ContributionMetrics::new(publisher_id.to_string()));
        entry.typedefs_published += 1;
        entry.last_contribution = Utc::now();
    }

    /// Record a typedef reference (someone installed/used the typedef).
    pub async fn record_reference(&self, publisher_id: &str) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics
            .entry(publisher_id.to_string())
            .or_insert_with(|| ContributionMetrics::new(publisher_id.to_string()));
        entry.typedef_references += 1;
    }

    /// Record a validation result against a publisher's schema.
    pub async fn record_validation(&self, publisher_id: &str, success: bool) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics
            .entry(publisher_id.to_string())
            .or_insert_with(|| ContributionMetrics::new(publisher_id.to_string()));
        if success {
            entry.successful_validations += 1;
        } else {
            entry.failed_validations += 1;
        }
    }

    /// Record schema maintenance activity (update, fix, deprecation).
    pub async fn record_maintenance(&self, publisher_id: &str, quality_delta: f64) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics
            .entry(publisher_id.to_string())
            .or_insert_with(|| ContributionMetrics::new(publisher_id.to_string()));
        entry.maintenance_score = (entry.maintenance_score + quality_delta).clamp(0.0, 1.0);
        entry.last_contribution = Utc::now();
    }

    /// Get metrics for a publisher.
    pub async fn get_metrics(&self, publisher_id: &str) -> Option<ContributionMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(publisher_id).cloned()
    }

    /// Get all publishers sorted by contribution score (descending).
    pub async fn top_contributors(&self, limit: usize) -> Vec<ContributionMetrics> {
        let metrics = self.metrics.read().await;
        let mut contributors: Vec<_> = metrics.values().cloned().collect();
        contributors.sort_by(|a, b| {
            b.contribution_score()
                .partial_cmp(&a.contribution_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        contributors.into_iter().take(limit).collect()
    }

    /// Get total number of tracked publishers.
    pub async fn publisher_count(&self) -> usize {
        self.metrics.read().await.len()
    }
}

// ---------------------------------------------------------------------------
// Reward adapter
// ---------------------------------------------------------------------------

/// Catalog-specific egress adapter for distributing CAES contribution rewards.
///
/// Implements Caesar's `EgressAdapter` to receive CAES reward settlements
/// for typedef publishers based on their contribution metrics.
pub struct CatalogRewardAdapter {
    /// Accumulated rewards per publisher (NodeId string -> GoldGrams)
    rewards: Arc<RwLock<HashMap<String, GoldGrams>>>,
    /// Maximum reward capacity per settlement period
    max_capacity: GoldGrams,
    /// Local node ID
    node_id: NodeId,
}

impl CatalogRewardAdapter {
    /// Create a new catalog reward adapter.
    pub fn new(node_id: NodeId, max_capacity: GoldGrams) -> Self {
        Self {
            rewards: Arc::new(RwLock::new(HashMap::new())),
            max_capacity,
            node_id,
        }
    }

    /// Get accumulated rewards for a publisher.
    pub async fn publisher_rewards(&self, publisher_id: &str) -> GoldGrams {
        let rewards = self.rewards.read().await;
        rewards
            .get(publisher_id)
            .copied()
            .unwrap_or_else(GoldGrams::zero)
    }

    /// Get all publisher rewards.
    pub async fn all_rewards(&self) -> HashMap<String, GoldGrams> {
        self.rewards.read().await.clone()
    }
}

#[async_trait]
impl EgressAdapter for CatalogRewardAdapter {
    fn adapter_id(&self) -> &str {
        "catalog_contribution_rewards"
    }

    fn supported_denominations(&self) -> Vec<String> {
        vec!["CAES".to_string()]
    }

    async fn available_capacity(&self) -> Result<GoldGrams, UpiError> {
        Ok(self.max_capacity)
    }

    async fn settle(
        &self,
        value: GoldGrams,
        destination: &str,
        denomination: &str,
        gold_price_usd: Decimal,
    ) -> Result<SettlementReceipt, UpiError> {
        if denomination != "CAES" {
            return Err(UpiError::UnsupportedDenomination {
                denomination: denomination.to_string(),
            });
        }

        if value > self.max_capacity {
            return Err(UpiError::InsufficientLiquidity {
                needed: value,
                available: self.max_capacity,
            });
        }

        // Credit publisher reward account
        let mut rewards = self.rewards.write().await;
        let balance = rewards
            .entry(destination.to_string())
            .or_insert_with(GoldGrams::zero);
        *balance = *balance + value;

        let settlement_id = format!("cat-reward-{}", uuid::Uuid::new_v4());

        info!(
            "Contribution reward: {} CAES -> publisher {} (total: {})",
            value.0, destination, balance.0
        );

        Ok(SettlementReceipt {
            settlement_id,
            adapter_id: self.adapter_id().to_string(),
            value,
            destination_denomination: denomination.to_string(),
            destination_amount: value.0, // 1:1 for CAES
            gold_price_at_settlement: gold_price_usd,
            settling_node: self.node_id.clone(),
            settled_at: Utc::now(),
            external_reference: format!("catalog-reward-{destination}"),
            finality: SettlementFinality::Trustless,
        })
    }

    async fn capacity_ratio(&self) -> Result<Decimal, UpiError> {
        if self.max_capacity.is_zero() {
            return Ok(Decimal::ZERO);
        }
        Ok(Decimal::ONE)
    }
}

// ---------------------------------------------------------------------------
// Reward service
// ---------------------------------------------------------------------------

/// Reward distribution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    /// Distribution ID
    pub distribution_id: String,
    /// Publisher receiving the reward
    pub publisher_id: String,
    /// Reward amount in CAES (gold grams)
    pub amount: GoldGrams,
    /// Contribution score at time of distribution
    pub contribution_score: f64,
    /// Settlement receipt
    pub receipt: Option<SettlementReceipt>,
    /// Distribution timestamp
    pub distributed_at: chrono::DateTime<Utc>,
}

/// Service for calculating and distributing contribution-based CAES rewards.
pub struct RewardService {
    /// Contribution tracker
    tracker: Arc<ContributionTracker>,
    /// Reward adapter
    adapter: Arc<CatalogRewardAdapter>,
    /// Distribution history
    distributions: Arc<RwLock<Vec<RewardDistribution>>>,
    /// Base reward pool per distribution cycle (gold grams)
    reward_pool: GoldGrams,
    /// Default gold price
    default_gold_price: Decimal,
}

impl RewardService {
    /// Create new reward service.
    pub fn new(node_id: NodeId, reward_pool: GoldGrams) -> Self {
        let max_capacity = reward_pool;
        Self {
            tracker: Arc::new(ContributionTracker::new()),
            adapter: Arc::new(CatalogRewardAdapter::new(node_id, max_capacity)),
            distributions: Arc::new(RwLock::new(Vec::new())),
            reward_pool,
            default_gold_price: Decimal::new(2350, 0),
        }
    }

    /// Get the contribution tracker.
    pub fn tracker(&self) -> &Arc<ContributionTracker> {
        &self.tracker
    }

    /// Get the reward adapter (for Caesar integration).
    pub fn reward_adapter(&self) -> Arc<CatalogRewardAdapter> {
        Arc::clone(&self.adapter)
    }

    /// Distribute rewards to all contributors proportional to their scores.
    ///
    /// Each publisher receives: `reward_pool * (their_score / total_scores)`.
    pub async fn distribute_rewards(&self) -> Result<Vec<RewardDistribution>> {
        let contributors = self.tracker.top_contributors(1000).await;
        if contributors.is_empty() {
            debug!("No contributors to reward");
            return Ok(Vec::new());
        }

        // Calculate total contribution score
        let total_score: f64 = contributors.iter().map(|c| c.contribution_score()).sum();
        if total_score <= 0.0 {
            debug!("Total contribution score is zero, skipping distribution");
            return Ok(Vec::new());
        }

        let mut distributions = Vec::new();

        for contributor in &contributors {
            let score = contributor.contribution_score();
            let share = score / total_score;
            let share_dec = Decimal::from_f64(share).unwrap_or(Decimal::ZERO);
            let reward_amount = GoldGrams::from_decimal(self.reward_pool.0 * share_dec);

            if reward_amount.is_zero() {
                continue;
            }

            let distribution_id = format!("dist-{}", uuid::Uuid::new_v4());

            let receipt = match self
                .adapter
                .settle(
                    reward_amount,
                    &contributor.publisher_id,
                    "CAES",
                    self.default_gold_price,
                )
                .await
            {
                Ok(r) => Some(r),
                Err(e) => {
                    info!(
                        "Reward settlement skipped for {}: {}",
                        contributor.publisher_id, e
                    );
                    None
                }
            };

            let distribution = RewardDistribution {
                distribution_id,
                publisher_id: contributor.publisher_id.clone(),
                amount: reward_amount,
                contribution_score: score,
                receipt,
                distributed_at: Utc::now(),
            };

            distributions.push(distribution);
        }

        // Record distributions
        let mut history = self.distributions.write().await;
        history.extend(distributions.clone());

        info!(
            "Distributed rewards to {} contributors from pool of {} CAES",
            distributions.len(),
            self.reward_pool.0
        );

        Ok(distributions)
    }

    /// Get distribution history for a publisher.
    pub async fn publisher_distributions(&self, publisher_id: &str) -> Vec<RewardDistribution> {
        let history = self.distributions.read().await;
        history
            .iter()
            .filter(|d| d.publisher_id == publisher_id)
            .cloned()
            .collect()
    }

    /// Get total rewards distributed.
    pub async fn total_distributed(&self) -> GoldGrams {
        let history = self.distributions.read().await;
        history
            .iter()
            .fold(GoldGrams::zero(), |acc, d| acc + d.amount)
    }

    /// Get publisher reward balance.
    pub async fn publisher_rewards(&self, publisher_id: &str) -> GoldGrams {
        self.adapter.publisher_rewards(publisher_id).await
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> NodeId {
        NodeId::from_public_key(b"test-catalog-node")
    }

    fn test_pool() -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(1000, 0)) // 1000 gram reward pool
    }

    #[test]
    fn test_contribution_score_calculation() {
        let mut metrics = ContributionMetrics::new("pub-1".to_string());
        metrics.typedefs_published = 5;
        metrics.typedef_references = 200;
        metrics.successful_validations = 90;
        metrics.failed_validations = 10;
        metrics.maintenance_score = 0.8;

        let score = metrics.contribution_score();
        assert!(score > 0.0 && score <= 1.0, "score {score} out of range");
        // 5 published = ln(6)/5 ~ 0.358 * 0.30 = 0.107
        // 200 refs = 200/1000 = 0.2 * 0.30 = 0.06
        // 90% validation = 0.9 * 0.25 = 0.225
        // 0.8 maintenance * 0.15 = 0.12
        // Total ~ 0.512
        assert!(score > 0.4, "score {score} should be > 0.4");
    }

    #[tokio::test]
    async fn test_contribution_tracker() {
        let tracker = ContributionTracker::new();

        tracker.record_publication("pub-1").await;
        tracker.record_publication("pub-1").await;
        tracker.record_reference("pub-1").await;
        tracker.record_validation("pub-1", true).await;
        tracker.record_validation("pub-1", true).await;
        tracker.record_validation("pub-1", false).await;

        let metrics = tracker
            .get_metrics("pub-1")
            .await
            .expect("test: metrics should exist");
        assert_eq!(metrics.typedefs_published, 2);
        assert_eq!(metrics.typedef_references, 1);
        assert_eq!(metrics.successful_validations, 2);
        assert_eq!(metrics.failed_validations, 1);
    }

    #[tokio::test]
    async fn test_top_contributors() {
        let tracker = ContributionTracker::new();

        // Publisher A: high contributor
        for _ in 0..10 {
            tracker.record_publication("pub-a").await;
            tracker.record_reference("pub-a").await;
        }

        // Publisher B: low contributor
        tracker.record_publication("pub-b").await;

        let top = tracker.top_contributors(10).await;
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].publisher_id, "pub-a"); // A should be first
    }

    #[tokio::test]
    async fn test_catalog_reward_adapter() {
        let adapter = CatalogRewardAdapter::new(test_node(), test_pool());
        assert_eq!(adapter.adapter_id(), "catalog_contribution_rewards");

        let value = GoldGrams::from_decimal(Decimal::new(50, 0));
        let receipt = adapter
            .settle(value, "pub-1", "CAES", Decimal::new(75, 0))
            .await
            .expect("test: settle should succeed");

        assert_eq!(receipt.finality, SettlementFinality::Trustless);

        let rewards = adapter.publisher_rewards("pub-1").await;
        assert_eq!(rewards, value);
    }

    #[tokio::test]
    async fn test_reward_service_distribution() {
        let service = RewardService::new(test_node(), test_pool());

        // Record contributions
        service.tracker().record_publication("pub-a").await;
        service.tracker().record_publication("pub-a").await;
        service.tracker().record_reference("pub-a").await;
        service.tracker().record_publication("pub-b").await;

        // Distribute rewards
        let distributions = service
            .distribute_rewards()
            .await
            .expect("test: distribution should succeed");

        assert_eq!(distributions.len(), 2);

        // pub-a should get more than pub-b (higher contribution)
        let a_dist = distributions
            .iter()
            .find(|d| d.publisher_id == "pub-a")
            .expect("test: pub-a should have distribution");
        let b_dist = distributions
            .iter()
            .find(|d| d.publisher_id == "pub-b")
            .expect("test: pub-b should have distribution");
        assert!(
            a_dist.amount > b_dist.amount,
            "pub-a should get more rewards"
        );

        // Verify total doesn't exceed pool
        let total = service.total_distributed().await;
        assert!(
            total.0 <= test_pool().0,
            "total {} exceeds pool {}",
            total.0,
            test_pool().0
        );
    }

    #[tokio::test]
    async fn test_empty_distribution() {
        let service = RewardService::new(test_node(), test_pool());

        // No contributions recorded
        let distributions = service
            .distribute_rewards()
            .await
            .expect("test: distribution should succeed");
        assert!(distributions.is_empty());
    }

    #[tokio::test]
    async fn test_unsupported_denomination() {
        let adapter = CatalogRewardAdapter::new(test_node(), test_pool());
        let value = GoldGrams::from_decimal(Decimal::new(10, 0));

        let result = adapter
            .settle(value, "pub-1", "BTC", Decimal::new(75, 0))
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("test: should be UnsupportedDenomination"),
            UpiError::UnsupportedDenomination { .. }
        ));
    }
}

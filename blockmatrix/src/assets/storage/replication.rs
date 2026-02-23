// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Replication Strategy
//!
//! Geospatial-aware replication based on content popularity and access patterns.

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::collections::HashMap;

use crate::integration::phase1_foundation::MatrixFoundation;
use crate::matrix::MatrixCoordinate;

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Base replication factor
    pub base_factor: usize,

    /// Maximum replication factor
    pub max_factor: usize,

    /// Popularity threshold for replication
    pub popularity_threshold: f64,

    /// Access count threshold
    pub access_threshold: usize,

    /// Time window for popularity calculation (seconds)
    pub time_window: u64,

    /// Geographic diversity requirement
    pub geo_diversity: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            base_factor: 3,
            max_factor: 20,
            popularity_threshold: 10.0,
            access_threshold: 100,
            time_window: 3600, // 1 hour
            geo_diversity: 3,
        }
    }
}

/// Popularity metrics for content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopularityMetrics {
    /// Total access count
    pub access_count: usize,

    /// Unique accessor count
    pub unique_accessors: usize,

    /// Access frequency (accesses per hour)
    pub access_frequency: f64,

    /// Popularity score (0-100)
    pub popularity_score: f64,

    /// Last access timestamp
    pub last_access: i64,

    /// First access timestamp
    pub first_access: i64,

    /// Geographic spread (number of regions)
    pub geo_spread: usize,

    /// Viral coefficient (rate of growth)
    pub viral_coefficient: f64,
}

impl PopularityMetrics {
    /// Calculate popularity score
    pub fn calculate_score(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let age = (now - self.first_access) as f64;

        if age > 0.0 {
            // Base score from access frequency
            let frequency_score = (self.access_frequency * 10.0).min(30.0);

            // Unique accessor bonus
            let unique_score = (self.unique_accessors as f64).sqrt() * 2.0;

            // Recency bonus (decay over time)
            let recency = (now - self.last_access) as f64;
            let recency_score = (20.0 * (-recency / 3600.0).exp()).max(0.0);

            // Geographic spread bonus
            let geo_score = (self.geo_spread as f64 * 5.0).min(15.0);

            // Viral growth bonus
            let viral_score = (self.viral_coefficient * 10.0).min(20.0);

            // Combined score (0-100)
            self.popularity_score = (frequency_score + unique_score + recency_score + geo_score + viral_score)
                .min(100.0)
                .max(0.0);
        }
    }

    /// Update metrics with new access
    pub fn record_access(&mut self, _accessor: &MatrixCoordinate) {
        let now = chrono::Utc::now().timestamp();

        if self.first_access == 0 {
            self.first_access = now;
        }

        self.access_count += 1;
        self.last_access = now;

        // Update frequency
        let age_hours = ((now - self.first_access) as f64 / 3600.0).max(1.0);
        self.access_frequency = self.access_count as f64 / age_hours;

        // Update viral coefficient (simplified)
        if self.access_count > 10 {
            let growth_rate = self.access_frequency / age_hours.sqrt();
            self.viral_coefficient = growth_rate;
        }

        self.calculate_score();
    }

    /// Check if content is viral
    pub fn is_viral(&self) -> bool {
        self.viral_coefficient > 1.0 && self.popularity_score > 50.0
    }

    /// Check if content needs replication
    pub fn needs_replication(&self, config: &ReplicationConfig) -> bool {
        self.popularity_score > config.popularity_threshold ||
        self.access_count > config.access_threshold ||
        self.is_viral()
    }
}

/// Replication strategy manager
pub struct ReplicationStrategy {
    /// Matrix foundation for geospatial operations
    _foundation: Arc<MatrixFoundation>,

    /// Replication configuration
    config: ReplicationConfig,

    /// Content popularity tracking
    popularity: Arc<RwLock<HashMap<String, PopularityMetrics>>>,

    /// Replication decisions cache
    decisions: Arc<RwLock<HashMap<String, ReplicationDecision>>>,

    /// Global statistics
    stats: Arc<RwLock<ReplicationStats>>,
}

/// Replication decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationDecision {
    /// Content identifier
    pub content_id: String,

    /// Calculated replication factor
    pub replication_factor: usize,

    /// Target matrix positions
    pub target_positions: Vec<MatrixCoordinate>,

    /// Decision timestamp
    pub timestamp: i64,

    /// Reason for replication
    pub reason: ReplicationReason,
}

/// Reason for replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationReason {
    /// High popularity score
    Popular,
    /// Viral growth detected
    Viral,
    /// Geographic distribution needed
    Geographic,
    /// Manual override
    Manual,
    /// Preventive caching
    Predictive,
}

/// Replication statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationStats {
    /// Total replications performed
    pub total_replications: usize,

    /// Viral content detected
    pub viral_detections: usize,

    /// Average replication factor
    pub avg_replication_factor: f64,

    /// Storage overhead from replication
    pub storage_overhead: usize,

    /// Cache hit rate improvement
    pub cache_hit_improvement: f64,
}

impl ReplicationStrategy {
    /// Create new replication strategy
    pub fn new(foundation: Arc<MatrixFoundation>) -> Self {
        Self {
            _foundation: foundation,
            config: ReplicationConfig::default(),
            popularity: Arc::new(RwLock::new(HashMap::new())),
            decisions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
        }
    }

    /// Create with custom configuration
    pub fn with_config(foundation: Arc<MatrixFoundation>, config: ReplicationConfig) -> Self {
        Self {
            _foundation: foundation,
            config,
            popularity: Arc::new(RwLock::new(HashMap::new())),
            decisions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
        }
    }

    /// Calculate replication factor based on popularity
    pub async fn calculate_replication_factor(&self, popularity: f64) -> usize {
        // Base factor
        let mut factor = self.config.base_factor;

        // Increase based on popularity (logarithmic growth)
        if popularity > self.config.popularity_threshold {
            let extra = ((popularity / 10.0).ln() * 2.0) as usize;
            factor += extra;
        }

        // Cap at maximum
        factor.min(self.config.max_factor)
    }

    /// Record content access and update metrics
    pub async fn record_access(&self, content_id: String, accessor: MatrixCoordinate) -> Result<()> {
        let mut popularity = self.popularity.write().await;
        let metrics = popularity.entry(content_id.clone()).or_default();

        metrics.record_access(&accessor);

        // Check if replication needed
        if metrics.needs_replication(&self.config) {
            let factor = self.calculate_replication_factor(metrics.popularity_score).await;

            // Make replication decision
            let decision = self.make_decision(content_id.clone(), factor, metrics.clone()).await?;

            let mut decisions = self.decisions.write().await;
            decisions.insert(content_id.clone(), decision);

            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_replications += 1;
            if metrics.is_viral() {
                stats.viral_detections += 1;
            }
        }

        Ok(())
    }

    /// Make replication decision
    async fn make_decision(
        &self,
        content_id: String,
        factor: usize,
        metrics: PopularityMetrics,
    ) -> Result<ReplicationDecision> {
        let reason = if metrics.is_viral() {
            ReplicationReason::Viral
        } else if metrics.popularity_score > self.config.popularity_threshold {
            ReplicationReason::Popular
        } else {
            ReplicationReason::Geographic
        };

        // Select target positions (placeholder - would use foundation in production)
        let target_positions = self.select_target_positions(factor).await?;

        Ok(ReplicationDecision {
            content_id,
            replication_factor: factor,
            target_positions,
            timestamp: chrono::Utc::now().timestamp(),
            reason,
        })
    }

    /// Select target positions for replication
    async fn select_target_positions(&self, count: usize) -> Result<Vec<MatrixCoordinate>> {
        // In production, this would use the foundation's geospatial algorithms
        // For now, generate diverse positions
        let mut positions = Vec::new();

        for i in 0..count {
            let x = (i % 10) as i64 * 10;
            let y = (i / 10) as i64 * 10;
            if let Ok(coord) = MatrixCoordinate::new(x, y, 0) {
                positions.push(coord);
            }
        }

        Ok(positions)
    }

    /// Get replication decision for content
    pub async fn get_decision(&self, content_id: &str) -> Option<ReplicationDecision> {
        self.decisions.read().await.get(content_id).cloned()
    }

    /// Get popularity metrics for content
    pub async fn get_metrics(&self, content_id: &str) -> Option<PopularityMetrics> {
        self.popularity.read().await.get(content_id).cloned()
    }

    /// Predict future popular content
    pub async fn predict_popular(&self) -> Vec<String> {
        let popularity = self.popularity.read().await;

        let mut predictions: Vec<_> = popularity
            .iter()
            .filter(|(_, metrics)| {
                // Rising content with high viral coefficient
                metrics.viral_coefficient > 0.5 &&
                metrics.popularity_score > 20.0 &&
                metrics.access_count < self.config.access_threshold
            })
            .map(|(id, metrics)| (id.clone(), metrics.viral_coefficient))
            .collect();

        // Sort by viral coefficient
        predictions.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        predictions.into_iter()
            .take(10)
            .map(|(id, _)| id)
            .collect()
    }

    /// Clean up old metrics
    pub async fn cleanup_old_metrics(&self, max_age_seconds: u64) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut popularity = self.popularity.write().await;

        let before = popularity.len();
        popularity.retain(|_, metrics| {
            ((now - metrics.last_access) as u64) < max_age_seconds
        });

        before - popularity.len()
    }

    /// Get replication statistics
    pub async fn get_stats(&self) -> ReplicationStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::phase1_foundation::MatrixFoundationConfig;

    #[test]
    fn test_popularity_metrics() {
        let mut metrics = PopularityMetrics::default();

        // Record multiple accesses
        for i in 0..100 {
            metrics.record_access(&MatrixCoordinate::new(i % 10, i / 10, 0).unwrap());
        }

        assert_eq!(metrics.access_count, 100);
        assert!(metrics.popularity_score > 0.0);
    }

    #[test]
    fn test_viral_detection() {
        let mut metrics = PopularityMetrics::default();
        metrics.viral_coefficient = 2.0;
        metrics.popularity_score = 60.0;

        assert!(metrics.is_viral());
    }

    #[tokio::test]
    async fn test_replication_factor_calculation() {
        let foundation = Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.unwrap());
        let strategy = ReplicationStrategy::new(foundation);

        // Low popularity
        let factor = strategy.calculate_replication_factor(5.0).await;
        assert_eq!(factor, strategy.config.base_factor);

        // High popularity
        let factor = strategy.calculate_replication_factor(50.0).await;
        assert!(factor > strategy.config.base_factor);
        assert!(factor <= strategy.config.max_factor);
    }

    #[tokio::test]
    async fn test_access_recording() {
        let foundation = Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.unwrap());
        let strategy = ReplicationStrategy::new(foundation);

        let content_id = "test_content".to_string();
        let accessor = MatrixCoordinate::new(0, 0, 0).unwrap();

        strategy.record_access(content_id.clone(), accessor).await.unwrap();

        let metrics = strategy.get_metrics(&content_id).await.unwrap();
        assert_eq!(metrics.access_count, 1);
    }

    #[tokio::test]
    async fn test_cleanup_old_metrics() {
        let foundation = Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.unwrap());
        let strategy = ReplicationStrategy::new(foundation);

        // Add some metrics
        let mut popularity = strategy.popularity.write().await;
        let mut old_metric = PopularityMetrics::default();
        old_metric.last_access = chrono::Utc::now().timestamp() - 7200; // 2 hours old
        popularity.insert("old".to_string(), old_metric);

        let mut new_metric = PopularityMetrics::default();
        new_metric.last_access = chrono::Utc::now().timestamp();
        popularity.insert("new".to_string(), new_metric);
        drop(popularity);

        // Clean up metrics older than 1 hour
        let cleaned = strategy.cleanup_old_metrics(3600).await;
        assert_eq!(cleaned, 1);

        assert!(strategy.get_metrics("old").await.is_none());
        assert!(strategy.get_metrics("new").await.is_some());
    }
}
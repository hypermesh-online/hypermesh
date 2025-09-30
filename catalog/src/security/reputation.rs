//! Publisher Reputation System
//!
//! Tracks and manages publisher reputation based on package quality and community feedback

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, debug, warn};

/// Publisher reputation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherReputation {
    /// Publisher TrustChain ID
    pub publisher_id: String,
    /// Current reputation score (0.0 - 1.0)
    pub score: f64,
    /// Total packages published
    pub total_packages: u64,
    /// Successful installations
    pub successful_installs: u64,
    /// Failed installations
    pub failed_installs: u64,
    /// Average user rating (1-5)
    pub average_rating: Option<f64>,
    /// Total ratings received
    pub total_ratings: u64,
    /// Number of verified packages
    pub verified_packages: u64,
    /// Number of packages with vulnerabilities
    pub vulnerable_packages: u64,
    /// Publisher tier
    pub tier: PublisherTier,
    /// Reputation history
    pub history: Vec<ReputationEvent>,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Publisher tier based on reputation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PublisherTier {
    /// New publisher with no history
    Unverified,
    /// Basic verification completed
    Bronze,
    /// Good track record
    Silver,
    /// Excellent track record
    Gold,
    /// Trusted partner/official
    Platinum,
}

/// Reputation event in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    /// Event type
    pub event_type: ReputationEventType,
    /// Score change
    pub score_change: f64,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event description
    pub description: String,
}

/// Types of reputation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEventType {
    /// Package published
    PackagePublished,
    /// Package installed successfully
    SuccessfulInstall,
    /// Package installation failed
    FailedInstall,
    /// User rating received
    UserRating(u8),
    /// Vulnerability reported
    VulnerabilityReported,
    /// Vulnerability fixed
    VulnerabilityFixed,
    /// Certificate verified
    CertificateVerified,
    /// Policy violation
    PolicyViolation,
    /// Manual adjustment
    ManualAdjustment,
}

/// Reputation system manager
pub struct ReputationSystem {
    /// Reputation database
    reputations: Arc<RwLock<HashMap<String, PublisherReputation>>>,
    /// Configuration
    config: ReputationConfig,
    /// Reputation calculator
    calculator: ReputationCalculator,
}

/// Reputation system configuration
#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Initial score for new publishers
    pub initial_score: f64,
    /// Score decay rate per day
    pub decay_rate: f64,
    /// Minimum score threshold
    pub min_score: f64,
    /// Maximum score threshold
    pub max_score: f64,
    /// Weight for successful installs
    pub success_weight: f64,
    /// Weight for failed installs
    pub failure_weight: f64,
    /// Weight for user ratings
    pub rating_weight: f64,
    /// Weight for vulnerabilities
    pub vulnerability_weight: f64,
    /// History retention days
    pub history_retention_days: u32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_score: 0.5,
            decay_rate: 0.001,
            min_score: 0.0,
            max_score: 1.0,
            success_weight: 0.01,
            failure_weight: -0.05,
            rating_weight: 0.02,
            vulnerability_weight: -0.1,
            history_retention_days: 365,
        }
    }
}

/// Reputation score calculator
struct ReputationCalculator {
    config: ReputationConfig,
}

impl ReputationCalculator {
    fn new(config: ReputationConfig) -> Self {
        Self { config }
    }

    /// Calculate reputation score
    fn calculate_score(&self, reputation: &PublisherReputation) -> f64 {
        let mut score = reputation.score;

        // Success rate factor
        if reputation.total_packages > 0 {
            let success_rate = reputation.successful_installs as f64 /
                              (reputation.successful_installs + reputation.failed_installs).max(1) as f64;
            score = score * 0.7 + success_rate * 0.3;
        }

        // Rating factor
        if let Some(avg_rating) = reputation.average_rating {
            let rating_factor = (avg_rating - 1.0) / 4.0; // Normalize 1-5 to 0-1
            score = score * 0.8 + rating_factor * 0.2;
        }

        // Vulnerability penalty
        if reputation.total_packages > 0 {
            let vuln_rate = reputation.vulnerable_packages as f64 / reputation.total_packages as f64;
            score *= 1.0 - (vuln_rate * 0.5);
        }

        // Apply time decay
        let days_since_update = (chrono::Utc::now() - reputation.last_updated).num_days() as f64;
        score *= 1.0 - (days_since_update * self.config.decay_rate);

        // Clamp to valid range
        score.max(self.config.min_score).min(self.config.max_score)
    }

    /// Determine publisher tier
    fn calculate_tier(&self, score: f64, total_packages: u64) -> PublisherTier {
        if total_packages == 0 {
            PublisherTier::Unverified
        } else if score >= 0.9 && total_packages >= 20 {
            PublisherTier::Platinum
        } else if score >= 0.75 && total_packages >= 10 {
            PublisherTier::Gold
        } else if score >= 0.5 && total_packages >= 5 {
            PublisherTier::Silver
        } else if total_packages >= 1 {
            PublisherTier::Bronze
        } else {
            PublisherTier::Unverified
        }
    }
}

impl ReputationSystem {
    /// Create new reputation system
    pub async fn new() -> Result<Self> {
        let config = ReputationConfig::default();
        let calculator = ReputationCalculator::new(config.clone());

        Ok(Self {
            reputations: Arc::new(RwLock::new(HashMap::new())),
            config,
            calculator,
        })
    }

    /// Get publisher reputation score
    pub async fn get_publisher_score(&self, publisher_id: &str) -> Result<f64> {
        let reputations = self.reputations.read().await;

        if let Some(reputation) = reputations.get(publisher_id) {
            Ok(self.calculator.calculate_score(reputation))
        } else {
            Ok(self.config.initial_score)
        }
    }

    /// Get full publisher reputation
    pub async fn get_publisher_reputation(&self, publisher_id: &str) -> Option<PublisherReputation> {
        let reputations = self.reputations.read().await;
        reputations.get(publisher_id).cloned()
    }

    /// Update reputation based on event
    pub async fn update_reputation(
        &self,
        publisher_id: &str,
        success: bool,
        user_rating: Option<u8>,
    ) -> Result<()> {
        let mut reputations = self.reputations.write().await;

        let reputation = reputations.entry(publisher_id.to_string())
            .or_insert_with(|| PublisherReputation {
                publisher_id: publisher_id.to_string(),
                score: self.config.initial_score,
                total_packages: 0,
                successful_installs: 0,
                failed_installs: 0,
                average_rating: None,
                total_ratings: 0,
                verified_packages: 0,
                vulnerable_packages: 0,
                tier: PublisherTier::Unverified,
                history: vec![],
                last_updated: chrono::Utc::now(),
            });

        // Update install counters
        if success {
            reputation.successful_installs += 1;
            self.add_event(reputation, ReputationEventType::SuccessfulInstall,
                          self.config.success_weight);
        } else {
            reputation.failed_installs += 1;
            self.add_event(reputation, ReputationEventType::FailedInstall,
                          self.config.failure_weight);
        }

        // Update user rating
        if let Some(rating) = user_rating {
            if rating >= 1 && rating <= 5 {
                // Update average rating
                let new_total = reputation.total_ratings + 1;
                let current_sum = reputation.average_rating.unwrap_or(0.0) * reputation.total_ratings as f64;
                reputation.average_rating = Some((current_sum + rating as f64) / new_total as f64);
                reputation.total_ratings = new_total;

                // Add rating event
                let rating_change = ((rating as f64 - 3.0) / 2.0) * self.config.rating_weight;
                self.add_event(reputation, ReputationEventType::UserRating(rating), rating_change);
            }
        }

        // Recalculate score and tier
        reputation.score = self.calculator.calculate_score(reputation);
        reputation.tier = self.calculator.calculate_tier(reputation.score, reputation.total_packages);
        reputation.last_updated = chrono::Utc::now();

        // Clean up old history
        self.cleanup_history(reputation);

        info!("Updated reputation for publisher {}: score={:.3}, tier={:?}",
              publisher_id, reputation.score, reputation.tier);

        Ok(())
    }

    /// Record package publication
    pub async fn record_package_published(&self, publisher_id: &str, is_verified: bool) -> Result<()> {
        let mut reputations = self.reputations.write().await;

        let reputation = reputations.entry(publisher_id.to_string())
            .or_insert_with(|| PublisherReputation {
                publisher_id: publisher_id.to_string(),
                score: self.config.initial_score,
                total_packages: 0,
                successful_installs: 0,
                failed_installs: 0,
                average_rating: None,
                total_ratings: 0,
                verified_packages: 0,
                vulnerable_packages: 0,
                tier: PublisherTier::Unverified,
                history: vec![],
                last_updated: chrono::Utc::now(),
            });

        reputation.total_packages += 1;
        if is_verified {
            reputation.verified_packages += 1;
        }

        self.add_event(reputation, ReputationEventType::PackagePublished, 0.01);

        reputation.score = self.calculator.calculate_score(reputation);
        reputation.tier = self.calculator.calculate_tier(reputation.score, reputation.total_packages);
        reputation.last_updated = chrono::Utc::now();

        Ok(())
    }

    /// Record vulnerability
    pub async fn record_vulnerability(&self, publisher_id: &str, fixed: bool) -> Result<()> {
        let mut reputations = self.reputations.write().await;

        if let Some(reputation) = reputations.get_mut(publisher_id) {
            if fixed {
                self.add_event(reputation, ReputationEventType::VulnerabilityFixed,
                              self.config.vulnerability_weight * 0.5);
                if reputation.vulnerable_packages > 0 {
                    reputation.vulnerable_packages -= 1;
                }
            } else {
                reputation.vulnerable_packages += 1;
                self.add_event(reputation, ReputationEventType::VulnerabilityReported,
                              self.config.vulnerability_weight);
            }

            reputation.score = self.calculator.calculate_score(reputation);
            reputation.tier = self.calculator.calculate_tier(reputation.score, reputation.total_packages);
            reputation.last_updated = chrono::Utc::now();
        }

        Ok(())
    }

    /// Get top publishers
    pub async fn get_top_publishers(&self, limit: usize) -> Vec<PublisherReputation> {
        let reputations = self.reputations.read().await;

        let mut publishers: Vec<_> = reputations.values()
            .cloned()
            .collect();

        publishers.sort_by(|a, b| {
            self.calculator.calculate_score(b)
                .partial_cmp(&self.calculator.calculate_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        publishers.into_iter().take(limit).collect()
    }

    /// Get publishers by tier
    pub async fn get_publishers_by_tier(&self, tier: PublisherTier) -> Vec<PublisherReputation> {
        let reputations = self.reputations.read().await;

        reputations.values()
            .filter(|r| r.tier == tier)
            .cloned()
            .collect()
    }

    /// Add reputation event
    fn add_event(&self, reputation: &mut PublisherReputation,
                event_type: ReputationEventType, score_change: f64) {
        reputation.history.push(ReputationEvent {
            event_type,
            score_change,
            timestamp: chrono::Utc::now(),
            description: format!("Score change: {:+.3}", score_change),
        });

        reputation.score = (reputation.score + score_change)
            .max(self.config.min_score)
            .min(self.config.max_score);
    }

    /// Clean up old history entries
    fn cleanup_history(&self, reputation: &mut PublisherReputation) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.config.history_retention_days as i64);

        reputation.history.retain(|event| event.timestamp > cutoff);
    }

    /// Export reputation data
    pub async fn export_reputations(&self) -> Result<Vec<PublisherReputation>> {
        let reputations = self.reputations.read().await;
        Ok(reputations.values().cloned().collect())
    }

    /// Import reputation data
    pub async fn import_reputations(&self, data: Vec<PublisherReputation>) -> Result<()> {
        let mut reputations = self.reputations.write().await;

        for reputation in data {
            reputations.insert(reputation.publisher_id.clone(), reputation);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reputation_system_creation() {
        let system = ReputationSystem::new().await;
        assert!(system.is_ok());
    }

    #[tokio::test]
    async fn test_initial_reputation() {
        let system = ReputationSystem::new().await.unwrap();
        let score = system.get_publisher_score("new-publisher").await.unwrap();
        assert_eq!(score, 0.5); // Default initial score
    }

    #[tokio::test]
    async fn test_reputation_update() {
        let system = ReputationSystem::new().await.unwrap();

        // Record successful install
        system.update_reputation("test-publisher", true, Some(5)).await.unwrap();

        let score = system.get_publisher_score("test-publisher").await.unwrap();
        assert!(score > 0.5); // Should increase from initial

        // Record failed install
        system.update_reputation("test-publisher", false, Some(2)).await.unwrap();

        let new_score = system.get_publisher_score("test-publisher").await.unwrap();
        assert!(new_score < score); // Should decrease
    }

    #[tokio::test]
    async fn test_tier_calculation() {
        let config = ReputationConfig::default();
        let calculator = ReputationCalculator::new(config);

        assert_eq!(calculator.calculate_tier(0.95, 25), PublisherTier::Platinum);
        assert_eq!(calculator.calculate_tier(0.8, 15), PublisherTier::Gold);
        assert_eq!(calculator.calculate_tier(0.6, 7), PublisherTier::Silver);
        assert_eq!(calculator.calculate_tier(0.4, 3), PublisherTier::Bronze);
        assert_eq!(calculator.calculate_tier(0.5, 0), PublisherTier::Unverified);
    }
}
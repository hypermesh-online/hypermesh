// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Organic vs speculative traffic detection (whitepaper section 16.5).
//!
//! Classification is **pattern-based and aggregate** -- it examines traffic
//! flow characteristics over a window, NOT individual transactions.
//!
//! Key heuristics:
//! - High diversity + sustained velocity = Organic
//! - Low diversity + high velocity = Speculative (wash-trading pattern)
//! - Low diversity + low velocity = Normal/Organic (small user)
//! - High velocity + concentrated counterparties = Speculative

use hypermesh_lib::GoldGrams;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TrafficPattern
// ---------------------------------------------------------------------------

/// Aggregate flow characteristics over a measurement window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficPattern {
    /// How many different resource types are being accessed (0.0..1.0).
    pub resource_diversity: f64,
    /// How many distinct counterparties are involved (0.0..1.0).
    pub counterparty_diversity: f64,
    /// Geographic spread of activity (0.0..1.0).
    pub geographic_spread: f64,
    /// Transactions per second over the window.
    pub velocity: f64,
    /// Average transaction value.
    pub avg_value: GoldGrams,
    /// Duration of the measurement window in seconds.
    pub duration_secs: u64,
}

// ---------------------------------------------------------------------------
// TrafficClassification
// ---------------------------------------------------------------------------

/// The result of classifying a [`TrafficPattern`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrafficClassification {
    /// Genuine resource usage from real users.
    Organic { confidence: f64 },
    /// Artificial / wash-trading patterns.
    Speculative { confidence: f64 },
    /// Ambiguous mixture.
    Mixed { confidence: f64, organic_ratio: f64 },
}

impl TrafficClassification {
    /// Confidence in the classification (0.0..1.0).
    pub fn confidence(&self) -> f64 {
        match self {
            Self::Organic { confidence } => *confidence,
            Self::Speculative { confidence } => *confidence,
            Self::Mixed { confidence, .. } => *confidence,
        }
    }

    /// Estimated fraction of organic traffic (1.0 for Organic, 0.0 for Speculative).
    pub fn organic_ratio(&self) -> f64 {
        match self {
            Self::Organic { .. } => 1.0,
            Self::Speculative { .. } => 0.0,
            Self::Mixed { organic_ratio, .. } => *organic_ratio,
        }
    }
}

// ---------------------------------------------------------------------------
// ClassifierConfig
// ---------------------------------------------------------------------------

/// Tunable thresholds for the traffic classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierConfig {
    /// Minimum diversity score to favor an Organic classification.
    pub min_organic_diversity: f64,
    /// Maximum counterparty concentration before Speculative flag.
    pub max_speculative_concentration: f64,
    /// Velocity above which high-frequency patterns are flagged (tps).
    pub velocity_threshold: f64,
    /// Minimum window duration for a meaningful classification (seconds).
    pub min_pattern_duration_secs: u64,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            min_organic_diversity: 0.4,
            max_speculative_concentration: 0.8,
            velocity_threshold: 10.0,
            min_pattern_duration_secs: 3600,
        }
    }
}

// ---------------------------------------------------------------------------
// TrafficClassifier
// ---------------------------------------------------------------------------

/// Classifies [`TrafficPattern`]s as Organic, Speculative, or Mixed.
#[derive(Debug, Clone)]
pub struct TrafficClassifier {
    config: ClassifierConfig,
}

impl TrafficClassifier {
    /// Create a classifier with the given configuration.
    pub fn new(config: ClassifierConfig) -> Self {
        Self { config }
    }

    /// Create a classifier with default thresholds.
    pub fn with_defaults() -> Self {
        Self::new(ClassifierConfig::default())
    }

    /// Classify a traffic pattern.
    pub fn classify(&self, pattern: &TrafficPattern) -> TrafficClassification {
        // Insufficient data -> low-confidence Mixed
        if pattern.duration_secs < self.config.min_pattern_duration_secs {
            return TrafficClassification::Mixed {
                confidence: 0.3,
                organic_ratio: 0.5,
            };
        }

        let diversity = combined_diversity(pattern);
        let is_high_velocity = pattern.velocity > self.config.velocity_threshold;
        let is_concentrated =
            pattern.counterparty_diversity < (1.0 - self.config.max_speculative_concentration);

        // High diversity + sustained velocity = Organic
        if diversity >= self.config.min_organic_diversity && !is_concentrated {
            let confidence = organic_confidence(pattern, &self.config);
            return TrafficClassification::Organic { confidence };
        }

        // Low diversity + high velocity + concentrated = Speculative (wash trading)
        if is_high_velocity && is_concentrated {
            let confidence = speculative_confidence(pattern, &self.config);
            return TrafficClassification::Speculative { confidence };
        }

        // Low diversity + low velocity = small user, treat as organic
        if !is_high_velocity && !is_concentrated {
            let confidence = organic_confidence(pattern, &self.config) * 0.8;
            return TrafficClassification::Organic { confidence };
        }

        // Everything else: Mixed
        let organic_ratio = diversity.clamp(0.0, 1.0);
        let confidence = 0.4 + (diversity * 0.3); // partial confidence
        TrafficClassification::Mixed {
            confidence: confidence.clamp(0.0, 1.0),
            organic_ratio,
        }
    }

    /// Read-only access to configuration.
    pub fn config(&self) -> &ClassifierConfig {
        &self.config
    }
}

impl Default for TrafficClassifier {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ---------------------------------------------------------------------------
// Internal scoring helpers
// ---------------------------------------------------------------------------

/// Combined diversity metric (mean of resource, counterparty, geographic).
fn combined_diversity(pattern: &TrafficPattern) -> f64 {
    (pattern.resource_diversity + pattern.counterparty_diversity + pattern.geographic_spread) / 3.0
}

/// Organic confidence based on diversity and duration.
fn organic_confidence(pattern: &TrafficPattern, config: &ClassifierConfig) -> f64 {
    let diversity = combined_diversity(pattern);
    let duration_factor = if config.min_pattern_duration_secs > 0 {
        (pattern.duration_secs as f64 / config.min_pattern_duration_secs as f64).clamp(0.0, 1.5)
    } else {
        1.0
    };
    let base = diversity * 0.6 + duration_factor * 0.2 + pattern.geographic_spread * 0.2;
    base.clamp(0.0, 1.0)
}

/// Speculative confidence based on velocity concentration and low diversity.
fn speculative_confidence(pattern: &TrafficPattern, config: &ClassifierConfig) -> f64 {
    let velocity_factor = if config.velocity_threshold > 0.0 {
        (pattern.velocity / config.velocity_threshold).clamp(0.0, 2.0) / 2.0
    } else {
        0.5
    };
    let concentration = 1.0 - pattern.counterparty_diversity;
    let low_diversity = 1.0 - combined_diversity(pattern);
    let base = velocity_factor * 0.4 + concentration * 0.35 + low_diversity * 0.25;
    base.clamp(0.0, 1.0)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn gold(n: i64) -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(n, 0))
    }

    fn organic_pattern() -> TrafficPattern {
        TrafficPattern {
            resource_diversity: 0.8,
            counterparty_diversity: 0.7,
            geographic_spread: 0.6,
            velocity: 5.0,
            avg_value: gold(10),
            duration_secs: 7200,
        }
    }

    fn speculative_pattern() -> TrafficPattern {
        TrafficPattern {
            resource_diversity: 0.1,
            counterparty_diversity: 0.05,
            geographic_spread: 0.1,
            velocity: 50.0,
            avg_value: gold(1000),
            duration_secs: 7200,
        }
    }

    #[test]
    fn classify_organic_pattern() {
        let classifier = TrafficClassifier::with_defaults();
        let result = classifier.classify(&organic_pattern());
        match result {
            TrafficClassification::Organic { confidence } => {
                assert!(confidence > 0.5, "organic confidence: {confidence}");
            }
            other => panic!("expected Organic, got: {other:?}"),
        }
    }

    #[test]
    fn classify_speculative_pattern() {
        let classifier = TrafficClassifier::with_defaults();
        let result = classifier.classify(&speculative_pattern());
        match result {
            TrafficClassification::Speculative { confidence } => {
                assert!(confidence > 0.3, "speculative confidence: {confidence}");
            }
            other => panic!("expected Speculative, got: {other:?}"),
        }
    }

    #[test]
    fn classify_insufficient_duration() {
        let classifier = TrafficClassifier::with_defaults();
        let mut pattern = organic_pattern();
        pattern.duration_secs = 60; // too short
        let result = classifier.classify(&pattern);
        match result {
            TrafficClassification::Mixed {
                confidence,
                organic_ratio,
            } => {
                assert!((confidence - 0.3).abs() < 1e-9);
                assert!((organic_ratio - 0.5).abs() < 1e-9);
            }
            other => panic!("expected Mixed for short duration, got: {other:?}"),
        }
    }

    #[test]
    fn classify_small_user_as_organic() {
        let classifier = TrafficClassifier::with_defaults();
        // Low diversity but low velocity and not concentrated -> small user -> organic
        let pattern = TrafficPattern {
            resource_diversity: 0.2,
            counterparty_diversity: 0.3,
            geographic_spread: 0.1,
            velocity: 1.0,
            avg_value: gold(5),
            duration_secs: 7200,
        };
        let result = classifier.classify(&pattern);
        match result {
            TrafficClassification::Organic { .. } => {} // expected
            other => panic!("expected Organic for small user, got: {other:?}"),
        }
    }

    #[test]
    fn classify_mixed_pattern() {
        let classifier = TrafficClassifier::with_defaults();
        // High velocity but NOT concentrated -> falls into Mixed
        let pattern = TrafficPattern {
            resource_diversity: 0.2,
            counterparty_diversity: 0.5,
            geographic_spread: 0.2,
            velocity: 20.0,
            avg_value: gold(50),
            duration_secs: 7200,
        };
        let result = classifier.classify(&pattern);
        match result {
            TrafficClassification::Mixed {
                confidence,
                organic_ratio,
            } => {
                assert!(confidence > 0.0);
                assert!((0.0..=1.0).contains(&organic_ratio));
            }
            other => panic!("expected Mixed, got: {other:?}"),
        }
    }

    #[test]
    fn organic_ratio_values() {
        assert!(
            (TrafficClassification::Organic { confidence: 0.9 }.organic_ratio() - 1.0).abs() < 1e-9
        );
        assert!(
            (TrafficClassification::Speculative { confidence: 0.9 }.organic_ratio()).abs() < 1e-9
        );
        let mixed = TrafficClassification::Mixed {
            confidence: 0.5,
            organic_ratio: 0.6,
        };
        assert!((mixed.organic_ratio() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn confidence_accessor() {
        assert!(
            (TrafficClassification::Organic { confidence: 0.8 }.confidence() - 0.8).abs() < 1e-9
        );
        assert!(
            (TrafficClassification::Speculative { confidence: 0.7 }.confidence() - 0.7).abs()
                < 1e-9
        );
        let mixed = TrafficClassification::Mixed {
            confidence: 0.6,
            organic_ratio: 0.5,
        };
        assert!((mixed.confidence() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn custom_config() {
        let config = ClassifierConfig {
            min_organic_diversity: 0.2,
            max_speculative_concentration: 0.9,
            velocity_threshold: 5.0,
            min_pattern_duration_secs: 1800,
        };
        let classifier = TrafficClassifier::new(config.clone());
        assert!((classifier.config().min_organic_diversity - 0.2).abs() < 1e-9);
        assert_eq!(classifier.config().min_pattern_duration_secs, 1800);
    }

    #[test]
    fn classification_serde_roundtrip() {
        let cases: Vec<TrafficClassification> = vec![
            TrafficClassification::Organic { confidence: 0.85 },
            TrafficClassification::Speculative { confidence: 0.72 },
            TrafficClassification::Mixed {
                confidence: 0.55,
                organic_ratio: 0.4,
            },
        ];
        for tc in &cases {
            let json = serde_json::to_string(tc).expect("test: serialize classification");
            let back: TrafficClassification =
                serde_json::from_str(&json).expect("test: deserialize classification");
            assert!(
                (tc.confidence() - back.confidence()).abs() < 1e-9,
                "confidence mismatch for {tc:?}"
            );
        }
    }
}

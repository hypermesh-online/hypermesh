// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Metrics ingestion pipeline for processing incoming MetricsFrame payloads.
//!
//! Routes frames by payload type to appropriate handlers, applies differential
//! privacy filtering, and stores results in time-series format for trending.

use std::collections::VecDeque;

use hypermesh_lib::NodeId;

use crate::streaming::privacy_filter::DifferentialPrivacyFilter;
use crate::streaming::protocol::{
    CapacitySnapshot, CongestionSnapshot, EconomicSnapshot, MetricsFrame, MetricsPayload,
    RoutingSnapshot, VerificationSnapshot,
};
use crate::trending::EpochTracker;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the ingestion pipeline.
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Maximum entries to retain per payload type per source.
    pub max_entries_per_source: usize,
    /// Differential privacy epsilon (higher = less noise).
    pub privacy_epsilon: f64,
    /// Whether to apply differential privacy filtering.
    pub enable_privacy_filter: bool,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_entries_per_source: 100,
            privacy_epsilon: 1.0,
            enable_privacy_filter: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Time-series storage
// ---------------------------------------------------------------------------

/// A time-stamped metrics entry for trending.
#[derive(Debug, Clone)]
pub struct TimestampedEntry<T> {
    /// Unix microsecond timestamp.
    pub timestamp_us: u64,
    /// Source node.
    pub source: NodeId,
    /// The payload data.
    pub data: T,
}

/// Per-type time-series storage.
#[derive(Debug, Default)]
struct TypedTimeSeries {
    capacity: VecDeque<TimestampedEntry<CapacitySnapshot>>,
    congestion: VecDeque<TimestampedEntry<CongestionSnapshot>>,
    routing: VecDeque<TimestampedEntry<RoutingSnapshot>>,
    economic: VecDeque<TimestampedEntry<EconomicSnapshot>>,
    verification: VecDeque<TimestampedEntry<VerificationSnapshot>>,
}

// ---------------------------------------------------------------------------
// IngestionStats
// ---------------------------------------------------------------------------

/// Statistics about ingestion pipeline throughput.
#[derive(Debug, Clone, Default)]
pub struct IngestionStats {
    /// Total frames ingested.
    pub frames_ingested: u64,
    /// Frames filtered out by privacy filter.
    pub frames_filtered: u64,
    /// Capacity payloads stored.
    pub capacity_count: u64,
    /// Congestion payloads stored.
    pub congestion_count: u64,
    /// Routing payloads stored.
    pub routing_count: u64,
    /// Economic payloads stored.
    pub economic_count: u64,
    /// Verification payloads stored.
    pub verification_count: u64,
}

// ---------------------------------------------------------------------------
// MetricsIngestionPipeline
// ---------------------------------------------------------------------------

/// Ingests MetricsFrame payloads, applies privacy filters, and stores
/// results in time-series format for trending analysis.
pub struct MetricsIngestionPipeline {
    config: IngestionConfig,
    privacy_filter: DifferentialPrivacyFilter,
    series: TypedTimeSeries,
    trend_tracker: EpochTracker,
    stats: IngestionStats,
}

impl MetricsIngestionPipeline {
    /// Create a new ingestion pipeline with the given configuration.
    pub fn new(config: IngestionConfig) -> Self {
        let epsilon = config.privacy_epsilon;
        Self {
            config,
            privacy_filter: DifferentialPrivacyFilter::new(epsilon),
            series: TypedTimeSeries::default(),
            trend_tracker: EpochTracker::new(100),
            stats: IngestionStats::default(),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(IngestionConfig::default())
    }

    /// Ingest a single MetricsFrame.
    ///
    /// Applies the differential privacy filter, then routes to the
    /// appropriate time-series store by payload type.
    pub fn ingest(&mut self, frame: MetricsFrame) {
        self.stats.frames_ingested += 1;

        // Apply privacy filter if enabled.
        let filtered = if self.config.enable_privacy_filter {
            match self.privacy_filter.filter_frame(frame) {
                Some(f) => f,
                None => {
                    self.stats.frames_filtered += 1;
                    return;
                }
            }
        } else {
            frame
        };

        let ts = filtered.timestamp_us;
        let source = filtered.source_node;
        let max = self.config.max_entries_per_source;

        match filtered.payload {
            MetricsPayload::Capacity(data) => {
                self.series.capacity.push_back(TimestampedEntry {
                    timestamp_us: ts,
                    source,
                    data,
                });
                Self::trim_deque(&mut self.series.capacity, max);
                self.stats.capacity_count += 1;
            }
            MetricsPayload::Congestion(data) => {
                self.series.congestion.push_back(TimestampedEntry {
                    timestamp_us: ts,
                    source,
                    data,
                });
                Self::trim_deque(&mut self.series.congestion, max);
                self.stats.congestion_count += 1;
            }
            MetricsPayload::Routing(data) => {
                self.series.routing.push_back(TimestampedEntry {
                    timestamp_us: ts,
                    source,
                    data,
                });
                Self::trim_deque(&mut self.series.routing, max);
                self.stats.routing_count += 1;
            }
            MetricsPayload::Economic(data) => {
                self.series.economic.push_back(TimestampedEntry {
                    timestamp_us: ts,
                    source,
                    data,
                });
                Self::trim_deque(&mut self.series.economic, max);
                self.stats.economic_count += 1;
            }
            MetricsPayload::Verification(data) => {
                self.series.verification.push_back(TimestampedEntry {
                    timestamp_us: ts,
                    source,
                    data,
                });
                Self::trim_deque(&mut self.series.verification, max);
                self.stats.verification_count += 1;
            }
        }
    }

    /// Get ingestion statistics.
    pub fn stats(&self) -> &IngestionStats {
        &self.stats
    }

    /// Access the trend tracker for epoch analysis.
    pub fn trend_tracker(&self) -> &EpochTracker {
        &self.trend_tracker
    }

    /// Mutable access to the trend tracker.
    pub fn trend_tracker_mut(&mut self) -> &mut EpochTracker {
        &mut self.trend_tracker
    }

    /// Number of stored capacity entries.
    pub fn capacity_entry_count(&self) -> usize {
        self.series.capacity.len()
    }

    /// Number of stored congestion entries.
    pub fn congestion_entry_count(&self) -> usize {
        self.series.congestion.len()
    }

    /// Number of stored routing entries.
    pub fn routing_entry_count(&self) -> usize {
        self.series.routing.len()
    }

    /// Number of stored economic entries.
    pub fn economic_entry_count(&self) -> usize {
        self.series.economic.len()
    }

    /// Number of stored verification entries.
    pub fn verification_entry_count(&self) -> usize {
        self.series.verification.len()
    }

    /// Trim a deque to the maximum allowed size.
    fn trim_deque<T>(deque: &mut VecDeque<T>, max: usize) {
        while deque.len() > max {
            deque.pop_front();
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::PrivacyMode;

    fn test_node() -> NodeId {
        NodeId::from_public_key(b"ingestion-test-node")
    }

    fn capacity_frame(seq: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_000_000 + seq,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024 * seq,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: seq,
        }
    }

    fn congestion_frame(fullness: f64) -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 2_000_000,
            privacy_mode: PrivacyMode::PRIVATE,
            payload: MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: fullness,
                queue_depth: 10,
                dropped_packets_epoch: 0,
                avg_queue_wait_us: 50,
            }),
            sequence: 1,
        }
    }

    fn economic_frame() -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(),
            timestamp_us: 3_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Economic(EconomicSnapshot {
                in_flight_float_grams: 42.5,
                settlement_rate_per_epoch: 10.0,
                active_packets: 7,
                ..Default::default()
            }),
            sequence: 2,
        }
    }

    #[test]
    fn ingest_routes_capacity_frames() {
        let mut pipeline = MetricsIngestionPipeline::new(IngestionConfig {
            enable_privacy_filter: false,
            ..Default::default()
        });

        pipeline.ingest(capacity_frame(1));
        pipeline.ingest(capacity_frame(2));

        assert_eq!(pipeline.capacity_entry_count(), 2);
        assert_eq!(pipeline.stats().frames_ingested, 2);
        assert_eq!(pipeline.stats().capacity_count, 2);
        assert_eq!(pipeline.stats().frames_filtered, 0);
    }

    #[test]
    fn ingest_routes_by_payload_type() {
        let mut pipeline = MetricsIngestionPipeline::new(IngestionConfig {
            enable_privacy_filter: false,
            ..Default::default()
        });

        pipeline.ingest(capacity_frame(1));
        pipeline.ingest(congestion_frame(0.5));
        pipeline.ingest(economic_frame());

        assert_eq!(pipeline.capacity_entry_count(), 1);
        assert_eq!(pipeline.congestion_entry_count(), 1);
        assert_eq!(pipeline.economic_entry_count(), 1);
        assert_eq!(pipeline.stats().frames_ingested, 3);
    }

    #[test]
    fn privacy_filter_suppresses_anonymous_frames() {
        let mut pipeline = MetricsIngestionPipeline::with_defaults();

        let anon_frame = MetricsFrame {
            source_node: test_node(),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::ANONYMOUS,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: 0,
        };

        pipeline.ingest(anon_frame);

        assert_eq!(pipeline.capacity_entry_count(), 0);
        assert_eq!(pipeline.stats().frames_filtered, 1);
    }

    #[test]
    fn time_series_trimming() {
        let mut pipeline = MetricsIngestionPipeline::new(IngestionConfig {
            max_entries_per_source: 3,
            enable_privacy_filter: false,
            ..Default::default()
        });

        for i in 0..10 {
            pipeline.ingest(capacity_frame(i));
        }

        assert_eq!(
            pipeline.capacity_entry_count(),
            3,
            "should trim to max_entries_per_source"
        );
        assert_eq!(pipeline.stats().capacity_count, 10);
    }
}

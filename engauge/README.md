# engauge - Network Analytics and Engagement Metrics

**Status: Alpha | 21 files | ~5.7K lines | 135 tests | 100% feature-complete**

engauge provides work tracking, content receipts, capacity metrics, and network analytics for the HyperMesh ecosystem. It measures what nodes actually contribute (bytes served, compute delivered, bandwidth, uptime) without trust scoring or reputation systems. Metrics are streamed with differential privacy filtering and used for routing intelligence and resource marketplace pricing.

## Architecture

```
Node Activity
  |-> Content receipts (BLAKE3 hash + timestamp + signature)
  |-> Capacity metrics (bytes, compute, storage, bandwidth, uptime)
  |-> Organic detection (aggregate flow analysis, no per-user tracking)
  |
  +-> Streaming (MetricsFrame protocol, privacy-filtered)
  |     |-> Regional aggregation (multi-node rollup)
  |     +-> Routing intelligence (tensor weight + path recommendations)
  |
  +-> Governor throttle signal (activity score, band/demurrage modifiers)
  +-> Marketplace (resource pools, lease contracts, pricing engine)
```

**Privacy model**: Anonymous = nothing shared. Private = capacity + congestion within federation. Public = all 4 payload types mesh-wide. Differential privacy (Laplace noise, epsilon-calibrated) applied to all streamed metrics.

## Key Features

- **Content receipts** -- BLAKE3 hash + timestamp + node signature, receipt bundles, verification
- **Metrics collector** -- compute cycles, bandwidth, latency, receipt density, activity scoring
- **Organic detection** -- pattern-based aggregate flow analysis (whitepaper S16.5), no per-user tracking
- **Governor throttle signal** -- activity score, band modifier, demurrage modifier, organic ratio feedback to Caesar
- **Capacity metrics** -- bytes served, compute delivered, storage, bandwidth, uptime; no trust/reputation
- **Multi-epoch trending** -- EpochTracker, CapacityTrend, AggregatedCapacity over time windows
- **Streaming protocol** -- MetricsFrame with 4 payload types: Capacity, Congestion, Routing, Economic
- **Differential privacy** -- Laplace noise injection calibrated by epsilon per privacy mode
- **Regional aggregation** -- multi-node metrics rollup for routing decisions
- **Routing intelligence** -- TensorWeightModifier (BlockMatrix integration), PathPolicyRecommendation (STOQ integration)
- **Resource marketplace** -- ResourcePool with sovereign allocation %, LeaseContract lifecycle (Proposed -> Active -> Completed)
- **Pricing engine** -- Governor-adjusted, tier multipliers (L0=1.0, L1=0.8, L2=0.5, L3=0.2), supply-demand scoring
- **Content push** -- opt-in/out recipients, publisher registration, fee-based anti-spam
- **STOQ integration** -- custom METRICS frame type (0xfe000007), feature-gated

## Modules

| Module | Purpose |
|--------|---------|
| `receipt` | Content receipts (BLAKE3 hash, signature, bundles) |
| `metrics` | Capacity collector, activity scoring |
| `compliance` | KYC compliance checker (self-sovereign attestation) |
| `organic_detection` | Speculative vs organic traffic classification |
| `throttle` | Governor throttle signal generation |
| `capacity` | Bytes/compute/storage/bandwidth/uptime tracking |
| `trending` | Multi-epoch aggregation, capacity trends |
| `streaming` | MetricsFrame protocol, publisher/subscriber, privacy filter |
| `routing_intel` | TensorWeightModifier, PathAdvisor traits |
| `marketplace` | Resource pools, lease contracts, pricing engine, content push |

## Quick Start

```bash
# Build
cargo build -p engauge

# Run tests
cargo test -p engauge
```

## Usage

```rust
use engauge::receipt::ContentReceipt;
use engauge::metrics::MetricsCollector;
use engauge::streaming::MetricsPublisher;

// Create a content receipt
let receipt = ContentReceipt::new(content_hash, node_id, timestamp);

// Collect capacity metrics
let collector = MetricsCollector::new(node_id);
collector.record_bytes_served(1024);

// Stream metrics with privacy filtering
let publisher = MetricsPublisher::new(privacy_mode);
publisher.publish(collector.snapshot()).await?;
```

## Dependencies

- `hypermesh-lib` -- canonical types (NodeId, PrivacyMode, MarketTier)
- `blake3` -- content receipt hashing
- `rust_decimal` -- precise pricing calculations
- `tokio` -- async runtime

## License

MIT OR Apache-2.0

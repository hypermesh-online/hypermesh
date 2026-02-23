// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

mod bandwidth;
mod mtu;
mod loss;
mod tier;
mod connection;
mod manager;

pub use bandwidth::{BandwidthSample, EwmaBandwidthEstimator};
pub use mtu::{MtuProbeState, MtuDiscovery};
pub use loss::LossBasedAdjuster;
pub use tier::congestion_control_for_tier;
pub use connection::{
    NetworkConditions, ConnectionParameters, AdaptiveConnection, AdaptationStats,
};
pub use manager::AdaptationManager;

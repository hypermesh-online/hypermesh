// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Governor Module -- PID Controller for Caesar EVP
//!
//! Adjusts fees, demurrage overrides, and routing incentives based on
//! real-time network metrics. Per-tier control loops for L0-L3 markets.

pub mod params;
pub mod pid;

pub use params::{GovernanceParams, PressureQuadrant};
pub use pid::GovernorPid;

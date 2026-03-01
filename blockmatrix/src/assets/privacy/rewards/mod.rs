// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CAESAR Reward Calculation System
//!
//! Calculates and manages CAESAR token rewards for hosting paid content via NGauge.
//!
//! ## CRITICAL: When CAESAR Rewards Apply
//!
//! CAESAR earnings ONLY occur when hosting paid content through NGauge:
//! - Advertisements
//! - KYCML-related content
//! - Paid hosting services (AWS-meets-torrent model)
//!
//! You do NOT earn CAESAR for:
//! - General P2P network participation
//! - Using work computer on private/federated network
//! - Buying products/services (e.g., buying a car)
//!
//! Earnings are specific to Asset type and paid content hosting, not network participation.
//!
//! ## Reward Calculation
//!
//! For eligible paid content hosting, rewards are calculated based on:
//! - Privacy levels (higher public access = higher rewards)
//! - Resource allocation and utilization
//! - Consensus proof validation
//! - Performance metrics and tier bonuses

mod calculation;
mod config;
mod defaults;
mod types;

pub use config::*;
pub use types::*;
// defaults module only provides Default impls, no additional exports needed

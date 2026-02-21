// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Ephemeral Value Protocol (EVP) -- Caesar's core packet-based value transfer engine.
//!
//! Value exists only in-flight: born at ingress, dies at egress.
//! Thermodynamic consistency: Input = Output + Friction.

pub mod demurrage;
pub mod packet;
pub mod types;

pub use demurrage::DemurrageEngine;
pub use packet::CaesPacket;
pub use types::{EvpConfig, TierClassifier};

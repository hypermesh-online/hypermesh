// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar SDK — Universal Payment Interface
//!
//! Adapter traits for integrating external payment rails with the Caesar
//! Ephemeral Value Protocol. Third-party developers implement
//! [`IngressAdapter`] and [`EgressAdapter`] to bridge value between external
//! systems (fiat, crypto, internal ledgers) and the Caesar network.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use caesar_sdk::{IngressAdapter, EgressAdapter, MeshCreditAdapter};
//! use caesar_sdk::types::{UpiError, IngressLockProof, SettlementReceipt};
//!
//! // Use the built-in MeshCreditAdapter for internal transfers
//! let adapter = MeshCreditAdapter::new(node_id);
//!
//! // Or implement your own adapter
//! struct MyPaymentRail;
//! impl IngressAdapter for MyPaymentRail { /* ... */ }
//! impl EgressAdapter for MyPaymentRail { /* ... */ }
//! ```

pub mod egress;
pub mod ingress;
pub mod mesh_credit;
pub mod types;

pub use egress::EgressAdapter;
pub use ingress::IngressAdapter;
pub use mesh_credit::MeshCreditAdapter;
pub use types::*;

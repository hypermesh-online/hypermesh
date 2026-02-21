// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Universal Payment Interface (UPI)
//!
//! Stateless adapter traits for value ingress (external -> CAES) and
//! egress (CAES -> external). Every node with an adapter becomes a
//! sovereign settlement point.

pub mod egress;
pub mod ingress;
pub mod types;

pub use egress::EgressAdapter;
pub use ingress::IngressAdapter;
pub use types::*;

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Universal Payment Interface (UPI)
//!
//! Re-exports from the standalone `caesar-sdk` crate. Stateless adapter
//! traits for value ingress (external -> CAES) and egress (CAES -> external).
//! Every node with an adapter becomes a sovereign settlement point.

pub mod egress {
    //! Egress adapter trait and test mocks.
    pub use caesar_sdk::egress::*;
}

pub mod ingress {
    //! Ingress adapter trait and test mocks.
    pub use caesar_sdk::ingress::*;
}

pub mod mesh_credit {
    //! MeshCredit reference adapter.
    pub use caesar_sdk::mesh_credit::*;
}

pub mod types {
    //! UPI shared types.
    pub use caesar_sdk::types::*;
}

pub use caesar_sdk::types::*;
pub use caesar_sdk::EgressAdapter;
pub use caesar_sdk::IngressAdapter;

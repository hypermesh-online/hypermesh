// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! FALCON-1024 Node Identity — re-exported from TrustChain
//!
//! The canonical implementation lives in `trustchain::identity`.
//! BlockMatrix re-exports it for backward compatibility.

pub use trustchain::identity::FalconIdentity;

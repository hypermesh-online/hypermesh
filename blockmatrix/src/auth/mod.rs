// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Authentication and authorization primitives (Phase K.1).
//!
//! Currently houses [`capability_token::CapabilityToken`] — FALCON-1024
//! signed scope-bounded session tokens issued by the daemon and bound to
//! a device pubkey. Tokens flow through `auth.create_session` IPC.

#![deny(unsafe_code)]

pub mod capability_token;

pub use capability_token::{
    Capability, CapabilityToken, CapabilityTokenError, CapabilityTokenIssuer, RevocationRegistry,
    SessionAction,
};

// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Direct messaging between HyperMesh nodes.
//!
//! Post-quantum encrypted (Kyber-1024 KEM + AES-256-GCM) and signed
//! (FALCON-1024) peer-to-peer messages, stored in a blockchain-backed
//! [`MessageStore`].

pub mod message;
pub mod store;

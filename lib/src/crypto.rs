// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shared cryptographic type definitions

use serde::{Serialize, Deserialize};

/// Cryptographic algorithm selection
/// FALCON-1024 for STOQ protocol signing
/// Kyber-1024 for asset encryption (NOT wrapping AES)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoAlgorithm {
    /// FALCON-1024 post-quantum signature scheme (STOQ protocol)
    Falcon1024,
    /// Kyber-1024 post-quantum encryption (asset encryption)
    Kyber1024,
    /// AES-256-GCM symmetric encryption (legacy, being replaced by Kyber)
    Aes256Gcm,
}

/// A cryptographic key pair identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPairId {
    pub algorithm: CryptoAlgorithm,
    pub fingerprint: String,
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Common types for HyperMesh transport layer

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::Ipv6Addr;

/// BlockMatrix's transport-layer peer identity with network addressing.
/// Unlike hypermesh_lib::NodeId (simple String wrapper), this carries the full
/// transport context: human-readable name, 32-byte cryptographic ID, IPv6 address,
/// and public key for peer verification during STOQ connections.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PeerIdentity {
    /// Human-readable node name
    pub name: String,
    /// Cryptographic node identifier (32-byte from certificate)
    pub id: [u8; 32],
    /// Node's IPv6 address
    pub address: Ipv6Addr,
    /// Public key for verification
    pub pub_key: Vec<u8>,
}

impl PeerIdentity {
    /// Create from name (for testing)
    pub fn from_name(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut id = [0u8; 32];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(32);
        id[..len].copy_from_slice(&name_bytes[..len]);

        Self {
            name,
            id,
            address: Ipv6Addr::LOCALHOST,
            pub_key: Vec::new(),
        }
    }

    /// Create with full details
    pub fn new(name: String, id: [u8; 32], address: Ipv6Addr, pub_key: Vec<u8>) -> Self {
        Self {
            name,
            id,
            address,
            pub_key,
        }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for PeerIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<String> for PeerIdentity {
    fn from(name: String) -> Self {
        Self::from_name(name)
    }
}

impl From<&str> for PeerIdentity {
    fn from(name: &str) -> Self {
        Self::from_name(name)
    }
}

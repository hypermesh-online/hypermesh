// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Common types for HyperMesh transport layer

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::Ipv6Addr;

/// BlockMatrix's transport-layer peer identity with network addressing.
/// Unlike hypermesh_lib::NodeId (a bare 32-byte BLAKE3 hash), this carries the full
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

    /// Extract the canonical `NodeId` from this peer's 32-byte cryptographic ID.
    pub fn to_node_id(&self) -> hypermesh_lib::NodeId {
        hypermesh_lib::NodeId::from_bytes(self.id)
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

impl From<&hypermesh_lib::ScopedIdentity> for PeerIdentity {
    fn from(identity: &hypermesh_lib::ScopedIdentity) -> Self {
        let node_id = identity.node_id;
        let label = identity
            .label
            .clone()
            .unwrap_or_else(|| node_id.to_string());
        Self {
            name: label,
            id: *node_id.as_bytes(),
            address: Ipv6Addr::LOCALHOST,
            pub_key: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::{IdentityScope, NodeId, ScopedIdentity};

    #[test]
    fn peer_identity_to_node_id_roundtrip() {
        let bytes = [0xAB; 32];
        let peer = PeerIdentity::new(
            "test-peer".into(),
            bytes,
            Ipv6Addr::LOCALHOST,
            Vec::new(),
        );
        let node_id = peer.to_node_id();
        assert_eq!(node_id.as_bytes(), &bytes);
    }

    #[test]
    fn peer_identity_from_scoped_identity() {
        let node_id = NodeId::from_public_key(b"falcon-key-data");
        let scope = IdentityScope::private_network();
        let identity = ScopedIdentity::new_node_with_label(node_id, scope, "my-node");

        let peer = PeerIdentity::from(&identity);
        assert_eq!(peer.name, "my-node");
        assert_eq!(&peer.id, node_id.as_bytes());
        assert_eq!(peer.to_node_id(), node_id);
    }

    #[test]
    fn peer_identity_from_scoped_identity_no_label() {
        let node_id = NodeId::from_bytes([0xCD; 32]);
        let scope = IdentityScope::anonymous_device();
        let identity = ScopedIdentity::new_node(node_id, scope);

        let peer = PeerIdentity::from(&identity);
        // Without a label, name falls back to NodeId display
        assert_eq!(&peer.id, node_id.as_bytes());
        assert_eq!(peer.to_node_id(), node_id);
    }
}

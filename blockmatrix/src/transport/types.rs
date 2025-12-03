//! Common types for HyperMesh transport layer

use serde::{Serialize, Deserialize};
use std::fmt;
use std::net::Ipv6Addr;

/// Canonical node identifier for HyperMesh network
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId {
    /// Human-readable node name
    pub name: String,
    /// Cryptographic node identifier (32-byte from certificate)
    pub id: [u8; 32],
    /// Node's IPv6 address
    pub address: Ipv6Addr,
    /// Public key for verification
    pub pub_key: Vec<u8>,
}

impl NodeId {
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
        Self { name, id, address, pub_key }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<String> for NodeId {
    fn from(name: String) -> Self {
        Self::from_name(name)
    }
}

impl From<&str> for NodeId {
    fn from(name: &str) -> Self {
        Self::from_name(name)
    }
}
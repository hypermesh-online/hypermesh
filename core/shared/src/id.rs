//! Identifier types for Nexus components

use serde::{Deserialize, Serialize};
use std::fmt;

/// Node identifier using 256-bit hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Create a new NodeId from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generate a new random NodeId
    pub fn random() -> Self {
        use ring::rand::{SecureRandom, SystemRandom};
        let rng = SystemRandom::new();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes).expect("RNG failure");
        Self(bytes)
    }

    /// Get the underlying bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create NodeId from hex string
    pub fn from_hex(hex: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(hex, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_hex()[..8])
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        // For tests, create a deterministic NodeId from string
        use ring::digest::{digest, SHA256};
        let digest_result = digest(&SHA256, s.as_bytes());
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(digest_result.as_ref());
        Self(bytes)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Resource identifier for containers, services, etc.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceId {
    namespace: String,
    name: String,
    kind: String,
}

impl ResourceId {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            kind: kind.into(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn random() -> Self {
        use ring::rand::{SecureRandom, SystemRandom};
        let rng = SystemRandom::new();
        let mut bytes = [0u8; 8];
        rng.fill(&mut bytes).expect("RNG failure");
        let random_suffix = hex::encode(bytes);
        
        Self {
            namespace: "default".to_string(),
            name: format!("resource-{}", random_suffix),
            kind: "resource".to_string(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        format!("{}/{}/{}", self.kind, self.namespace, self.name).into_bytes()
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.kind, self.namespace, self.name)
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            name: "unnamed".to_string(),
            kind: "resource".to_string(),
        }
    }
}

impl From<String> for ResourceId {
    fn from(s: String) -> Self {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 3 {
            Self {
                kind: parts[0].to_string(),
                namespace: parts[1].to_string(),
                name: parts[2].to_string(),
            }
        } else {
            Self {
                namespace: "default".to_string(),
                name: s,
                kind: "resource".to_string(),
            }
        }
    }
}

/// Service identifier for network services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId {
    name: String,
    namespace: String,
}

impl ServiceId {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.name, self.namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_creation() {
        let id1 = NodeId::random();
        let id2 = NodeId::random();
        assert_ne!(id1, id2);
    }

    #[test] 
    fn test_node_id_hex() {
        let id = NodeId::new([1; 32]);
        let hex = id.to_hex();
        let parsed = NodeId::from_hex(&hex).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_resource_id() {
        let rid = ResourceId::new("default", "web-server", "deployment");
        assert_eq!(rid.to_string(), "deployment/default/web-server");
    }
}
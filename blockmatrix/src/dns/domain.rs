// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Domain Asset Type & Registration
//!
//! Domains are first-class blockchain assets. Registering a domain derives
//! a Network-scope chain ID (BLAKE3 hash) and a corresponding DNS pool.

use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::PrivacyMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::info;

/// A domain registered as a blockchain asset that creates a Network-scope chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainRegistration {
    /// The full domain name (e.g. "home.persist.hypermesh")
    pub domain_name: String,
    /// BLAKE3 hash of the domain name — deterministic chain identifier
    pub chain_id: [u8; 32],
    /// Hex-encoded first 16 bytes of chain_id — human-readable network ID
    pub network_id: String,
    /// Network ID of the parent domain, if this domain has components
    pub parent_network_id: Option<String>,
    /// Privacy mode for this domain's network scope
    pub privacy_mode: PrivacyMode,
    /// Node that owns/registered this domain
    pub owner_node_id: String,
    /// When the domain was registered
    pub created_at: SystemTime,
    /// Serialized state proof bytes used at registration time
    pub state_proof_bytes: Option<Vec<u8>>,
}

impl DomainRegistration {
    /// Create a new domain registration with derived chain_id, network_id,
    /// and parent_network_id computed from domain components.
    pub fn new(
        domain_name: &str,
        privacy_mode: PrivacyMode,
        owner_node_id: String,
    ) -> Self {
        let chain_id = derive_chain_id(domain_name);
        let network_id = derive_network_id(domain_name);

        // Parent: split on '.', if >1 component, parent is everything after first '.'
        let parent_network_id = extract_parent_domain(domain_name)
            .map(|parent| derive_network_id(&parent));

        Self {
            domain_name: domain_name.to_string(),
            chain_id,
            network_id,
            parent_network_id,
            privacy_mode,
            owner_node_id,
            created_at: SystemTime::now(),
            state_proof_bytes: None,
        }
    }
}

/// Derive a deterministic 32-byte chain ID from a domain name using BLAKE3.
pub fn derive_chain_id(domain_name: &str) -> [u8; 32] {
    *blake3::hash(domain_name.as_bytes()).as_bytes()
}

/// Derive a human-readable network ID: hex of the first 16 bytes of the chain ID.
/// Returns a 32-character hex string.
pub fn derive_network_id(domain_name: &str) -> String {
    let chain_id = derive_chain_id(domain_name);
    hex::encode(&chain_id[..16])
}

/// Extract the parent domain from a dotted domain name.
/// "home.persist.hypermesh" -> Some("persist.hypermesh")
/// "hypermesh" -> None
fn extract_parent_domain(domain_name: &str) -> Option<String> {
    let first_dot = domain_name.find('.')?;
    let parent = &domain_name[first_dot + 1..];
    if parent.is_empty() {
        None
    } else {
        Some(parent.to_string())
    }
}

/// Persist domain registrations to a JSON file.
pub fn save_domains(domains: &[DomainRegistration], path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(domains)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load domain registrations from a JSON file.
pub fn load_domains(path: &Path) -> anyhow::Result<Vec<DomainRegistration>> {
    let data = std::fs::read_to_string(path)?;
    let domains: Vec<DomainRegistration> = serde_json::from_str(&data)?;
    Ok(domains)
}

/// Manages the relationship between domain registrations and network membership.
///
/// When a node joins a domain, it also joins the corresponding Network-scope chain
/// via the `SyncManager`. Leaving a domain removes the network membership.
pub struct DomainNetworkManager {
    sync_manager: Arc<tokio::sync::Mutex<SyncManager>>,
    /// Retained for future domain-aware operations (e.g. listing registered domains).
    #[allow(dead_code)]
    domain_registry: Arc<RwLock<HashMap<String, DomainRegistration>>>,
}

impl DomainNetworkManager {
    /// Create a new domain-network manager.
    pub fn new(
        sync_manager: Arc<tokio::sync::Mutex<SyncManager>>,
        domain_registry: Arc<RwLock<HashMap<String, DomainRegistration>>>,
    ) -> Self {
        Self {
            sync_manager,
            domain_registry,
        }
    }

    /// Join a domain's Network-scope chain via the SyncManager.
    ///
    /// Derives a deterministic network ID from the domain name and registers
    /// the node as a member of that network.
    pub async fn join_domain(
        &self,
        domain_name: &str,
        privacy_mode: PrivacyMode,
    ) -> Result<(), String> {
        let network_id = derive_network_id(domain_name);
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        info!(
            domain = %domain_name,
            network_id = %network_id,
            "Joining domain network"
        );

        self.sync_manager
            .lock()
            .await
            .join_network(network_id, privacy_mode, now_secs)
    }

    /// Leave a domain's Network-scope chain.
    pub async fn leave_domain(&self, domain_name: &str) -> Result<(), String> {
        let network_id = derive_network_id(domain_name);

        info!(
            domain = %domain_name,
            network_id = %network_id,
            "Leaving domain network"
        );

        self.sync_manager.lock().await.leave_network(&network_id)
    }

    /// Check whether this node is a member of the given domain's network.
    pub async fn is_domain_member(&self, domain_name: &str) -> bool {
        let network_id = derive_network_id(domain_name);
        self.sync_manager.lock().await.is_member(&network_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_chain_id_deterministic() {
        let id1 = derive_chain_id("hypermesh");
        let id2 = derive_chain_id("hypermesh");
        assert_eq!(id1, id2, "same input must produce same output");

        let id3 = derive_chain_id("otherdomain");
        assert_ne!(id1, id3, "different input must produce different output");
    }

    #[test]
    fn test_derive_network_id_format() {
        let nid = derive_network_id("hypermesh");
        assert_eq!(nid.len(), 32, "network_id must be 32 hex chars");
        assert!(
            nid.chars().all(|c| c.is_ascii_hexdigit()),
            "network_id must be all hex chars"
        );
    }

    #[test]
    fn test_parent_chain_derivation() {
        // Multi-component: parent is everything after first dot
        let parent = extract_parent_domain("home.persist.hypermesh");
        assert_eq!(parent.as_deref(), Some("persist.hypermesh"));

        // Single component: no parent
        let parent = extract_parent_domain("hypermesh");
        assert!(parent.is_none());
    }

    #[test]
    fn test_domain_registration_serde_roundtrip() {
        let reg = DomainRegistration::new(
            "home.persist.hypermesh",
            PrivacyMode::PRIVATE,
            "node-42".to_string(),
        );
        let json = serde_json::to_string(&reg).expect("test: serialize");
        let back: DomainRegistration =
            serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(back.domain_name, reg.domain_name);
        assert_eq!(back.chain_id, reg.chain_id);
        assert_eq!(back.network_id, reg.network_id);
        assert_eq!(back.parent_network_id, reg.parent_network_id);
        assert_eq!(back.owner_node_id, reg.owner_node_id);
    }

    #[test]
    fn test_domain_persistence_roundtrip() {
        let regs = vec![
            DomainRegistration::new("alpha", PrivacyMode::PUBLIC, "n1".to_string()),
            DomainRegistration::new("beta.alpha", PrivacyMode::PRIVATE, "n2".to_string()),
        ];

        let dir = tempfile::tempdir().expect("test: tempdir");
        let path = dir.path().join("domains.json");

        save_domains(&regs, &path).expect("test: save");
        let loaded = load_domains(&path).expect("test: load");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].domain_name, "alpha");
        assert_eq!(loaded[1].domain_name, "beta.alpha");
        assert_eq!(loaded[0].chain_id, regs[0].chain_id);
        assert_eq!(loaded[1].chain_id, regs[1].chain_id);
    }

    #[test]
    fn test_domain_registration_constructor() {
        let reg = DomainRegistration::new(
            "test.hypermesh",
            PrivacyMode::ANONYMOUS,
            "owner-1".to_string(),
        );
        assert_eq!(reg.domain_name, "test.hypermesh");
        assert_eq!(reg.owner_node_id, "owner-1");
        assert_eq!(reg.network_id.len(), 32);
        assert!(reg.state_proof_bytes.is_none());
        assert!(reg.parent_network_id.is_some());
    }

    #[test]
    fn test_different_domains_different_ids() {
        let nid_foo = derive_network_id("foo");
        let nid_bar = derive_network_id("bar");
        assert_ne!(nid_foo, nid_bar);
    }

    #[test]
    fn test_parent_network_id_derivation() {
        let reg = DomainRegistration::new(
            "home.persist.hypermesh",
            PrivacyMode::PRIVATE,
            "n1".to_string(),
        );
        let expected_parent_nid = derive_network_id("persist.hypermesh");
        assert_eq!(
            reg.parent_network_id.as_deref(),
            Some(expected_parent_nid.as_str())
        );
    }

    use crate::blockchain::sync_manager::{SyncConfig, SyncManager};

    fn test_sync_manager() -> SyncManager {
        SyncManager::new(
            "test-device-chain".to_string(),
            SyncConfig {
                max_networks: 8,
                ..SyncConfig::default()
            },
        )
    }

    #[tokio::test]
    async fn test_join_domain_calls_sync_manager() {
        let sm = Arc::new(tokio::sync::Mutex::new(test_sync_manager()));
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let mgr = DomainNetworkManager::new(sm.clone(), registry);

        mgr.join_domain("example.hypermesh", PrivacyMode::PRIVATE)
            .await
            .expect("test: join domain");

        let network_id = derive_network_id("example.hypermesh");
        assert!(sm.lock().await.is_member(&network_id));
    }

    #[tokio::test]
    async fn test_leave_domain_removes_membership() {
        let sm = Arc::new(tokio::sync::Mutex::new(test_sync_manager()));
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let mgr = DomainNetworkManager::new(sm.clone(), registry);

        mgr.join_domain("test.domain", PrivacyMode::PUBLIC)
            .await
            .expect("test: join");

        let network_id = derive_network_id("test.domain");
        assert!(sm.lock().await.is_member(&network_id));

        mgr.leave_domain("test.domain")
            .await
            .expect("test: leave");
        assert!(!sm.lock().await.is_member(&network_id));
    }

    #[tokio::test]
    async fn test_is_domain_member() {
        let sm = Arc::new(tokio::sync::Mutex::new(test_sync_manager()));
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let mgr = DomainNetworkManager::new(sm, registry);

        assert!(!mgr.is_domain_member("absent.domain").await);

        mgr.join_domain("absent.domain", PrivacyMode::ANONYMOUS)
            .await
            .expect("test: join");

        assert!(mgr.is_domain_member("absent.domain").await);
    }
}

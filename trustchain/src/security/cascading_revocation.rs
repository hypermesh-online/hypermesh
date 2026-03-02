// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cascading Certificate Revocation
//!
//! When a parent certificate is revoked, all child certificates in the
//! hierarchy must also be revoked. `CascadingRevocation` tracks parent-child
//! relationships and propagates revocations downward through the tree.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::errors::{Result as TrustChainResult, TrustChainError};

/// A revocation event produced by the cascade.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CascadeRevocation {
    /// Serial number of the certificate that was revoked.
    pub serial_number: String,
    /// Serial number of the parent whose revocation triggered this one.
    pub triggered_by: String,
    /// Depth in the cascade (0 = directly triggered by the root revocation).
    pub depth: u32,
    /// Timestamp when the cascade revocation was applied.
    pub revoked_at: SystemTime,
}

/// Result of a cascade operation.
#[derive(Clone, Debug)]
pub struct CascadeResult {
    /// The root certificate that was revoked.
    pub root_serial: String,
    /// All certificates revoked as a result (including the root).
    pub affected: Vec<CascadeRevocation>,
}

/// Tracks parent-child certificate relationships and propagates revocations.
pub struct CascadingRevocation {
    /// Maps parent serial -> set of child serials.
    children: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Maps child serial -> parent serial (for reverse lookups).
    parents: Arc<RwLock<HashMap<String, String>>>,
    /// Set of serials that have been revoked.
    revoked: Arc<RwLock<HashSet<String>>>,
}

impl CascadingRevocation {
    /// Create a new empty cascading revocation tracker.
    pub fn new() -> Self {
        Self {
            children: Arc::new(RwLock::new(HashMap::new())),
            parents: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Register a parent-child relationship between two certificates.
    pub async fn register_relationship(
        &self,
        parent_serial: &str,
        child_serial: &str,
    ) -> TrustChainResult<()> {
        // Prevent self-referencing
        if parent_serial == child_serial {
            return Err(TrustChainError::InvalidRequest {
                reason: "Certificate cannot be its own parent".to_string(),
            });
        }

        let mut children = self.children.write().await;
        children
            .entry(parent_serial.to_string())
            .or_default()
            .insert(child_serial.to_string());

        let mut parents = self.parents.write().await;
        parents.insert(child_serial.to_string(), parent_serial.to_string());

        debug!(
            "Registered cert relationship: {} -> {}",
            parent_serial, child_serial
        );
        Ok(())
    }

    /// Revoke a certificate and cascade the revocation to all descendants.
    ///
    /// Returns the list of all affected certificates (including the root).
    pub async fn revoke_with_cascade(
        &self,
        serial_number: &str,
        reason: &str,
    ) -> TrustChainResult<CascadeResult> {
        let now = SystemTime::now();
        let mut affected = Vec::new();

        // Check if already revoked
        {
            let revoked = self.revoked.read().await;
            if revoked.contains(serial_number) {
                return Err(TrustChainError::InvalidRequest {
                    reason: format!(
                        "Certificate '{}' is already revoked",
                        serial_number
                    ),
                });
            }
        }

        // BFS traversal to find all descendants
        let mut queue: Vec<(String, String, u32)> =
            vec![(serial_number.to_string(), serial_number.to_string(), 0)];

        while let Some((current, triggered_by, depth)) = queue.pop() {
            // Mark as revoked
            {
                let mut revoked = self.revoked.write().await;
                if !revoked.insert(current.clone()) {
                    // Already revoked (cycle protection)
                    continue;
                }
            }

            affected.push(CascadeRevocation {
                serial_number: current.clone(),
                triggered_by: triggered_by.clone(),
                depth,
                revoked_at: now,
            });

            // Find children and enqueue them
            let children = self.children.read().await;
            if let Some(child_set) = children.get(&current) {
                for child in child_set {
                    queue.push((
                        child.clone(),
                        current.clone(),
                        depth + 1,
                    ));
                }
            }
        }

        info!(
            "Cascading revocation from '{}' (reason: {}): {} certificates affected",
            serial_number,
            reason,
            affected.len()
        );

        Ok(CascadeResult {
            root_serial: serial_number.to_string(),
            affected,
        })
    }

    /// Check whether a certificate has been revoked (directly or via cascade).
    pub async fn is_revoked(&self, serial_number: &str) -> bool {
        self.revoked.read().await.contains(serial_number)
    }

    /// Get the parent of a certificate, if registered.
    pub async fn get_parent(&self, serial_number: &str) -> Option<String> {
        self.parents.read().await.get(serial_number).cloned()
    }

    /// Get all direct children of a certificate.
    pub async fn get_children(&self, serial_number: &str) -> Vec<String> {
        self.children
            .read()
            .await
            .get(serial_number)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for CascadingRevocation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_level_cascade() {
        let cr = CascadingRevocation::new();

        // Root -> Child1, Child2
        cr.register_relationship("root", "child-1")
            .await
            .expect("test: register");
        cr.register_relationship("root", "child-2")
            .await
            .expect("test: register");

        let result = cr
            .revoke_with_cascade("root", "key compromise")
            .await
            .expect("test: cascade");

        assert_eq!(result.root_serial, "root");
        assert_eq!(result.affected.len(), 3); // root + 2 children

        assert!(cr.is_revoked("root").await);
        assert!(cr.is_revoked("child-1").await);
        assert!(cr.is_revoked("child-2").await);

        // Verify depths
        let root_entry = result
            .affected
            .iter()
            .find(|e| e.serial_number == "root")
            .expect("test: root in affected");
        assert_eq!(root_entry.depth, 0);

        let child_entries: Vec<_> = result
            .affected
            .iter()
            .filter(|e| e.depth == 1)
            .collect();
        assert_eq!(child_entries.len(), 2);
    }

    #[tokio::test]
    async fn test_multi_level_cascade() {
        let cr = CascadingRevocation::new();

        // Root -> Intermediate -> Leaf1, Leaf2
        cr.register_relationship("root-ca", "intermediate-ca")
            .await
            .expect("test: register");
        cr.register_relationship("intermediate-ca", "leaf-1")
            .await
            .expect("test: register");
        cr.register_relationship("intermediate-ca", "leaf-2")
            .await
            .expect("test: register");

        let result = cr
            .revoke_with_cascade("root-ca", "ca compromise")
            .await
            .expect("test: cascade");

        assert_eq!(result.affected.len(), 4); // root + intermediate + 2 leaves

        assert!(cr.is_revoked("root-ca").await);
        assert!(cr.is_revoked("intermediate-ca").await);
        assert!(cr.is_revoked("leaf-1").await);
        assert!(cr.is_revoked("leaf-2").await);

        // Verify cascade chain
        let intermediate = result
            .affected
            .iter()
            .find(|e| e.serial_number == "intermediate-ca")
            .expect("test: intermediate in affected");
        assert_eq!(intermediate.depth, 1);
        assert_eq!(intermediate.triggered_by, "root-ca");

        let leaf = result
            .affected
            .iter()
            .find(|e| e.serial_number == "leaf-1")
            .expect("test: leaf in affected");
        assert_eq!(leaf.depth, 2);
        assert_eq!(leaf.triggered_by, "intermediate-ca");
    }

    #[tokio::test]
    async fn test_double_revocation_rejected() {
        let cr = CascadingRevocation::new();

        cr.revoke_with_cascade("cert-1", "test")
            .await
            .expect("test: first revoke");

        let err = cr.revoke_with_cascade("cert-1", "test again").await;
        assert!(err.is_err(), "Double revocation should be rejected");
    }

    #[tokio::test]
    async fn test_self_reference_rejected() {
        let cr = CascadingRevocation::new();

        let err = cr.register_relationship("cert-x", "cert-x").await;
        assert!(err.is_err(), "Self-reference should be rejected");
    }

    #[tokio::test]
    async fn test_parent_child_lookups() {
        let cr = CascadingRevocation::new();

        cr.register_relationship("parent", "child-a")
            .await
            .expect("test: register");
        cr.register_relationship("parent", "child-b")
            .await
            .expect("test: register");

        assert_eq!(
            cr.get_parent("child-a").await,
            Some("parent".to_string())
        );

        let children = cr.get_children("parent").await;
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"child-a".to_string()));
        assert!(children.contains(&"child-b".to_string()));
    }
}

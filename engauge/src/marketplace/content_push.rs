// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Content push manager for paid content distribution.
//!
//! Publishers can push content to opted-in recipients in exchange for
//! Caesar EVP fees. Recipients must explicitly opt in before receiving
//! any pushed content, preserving node sovereignty.

use std::collections::{HashMap, HashSet};

use hypermesh_lib::economic::{GoldGrams, PacketId};
use hypermesh_lib::{AssetId, NodeId};
use serde::{Deserialize, Serialize};

/// A content push request from a publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPushRequest {
    pub content_id: AssetId,
    pub publisher: NodeId,
    pub fee_budget: GoldGrams,
    pub target_recipients: Vec<NodeId>,
    pub settlement_evp: Option<PacketId>,
}

/// Status of a content push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PushStatus {
    Pending,
    Delivering,
    Delivered,
    Failed,
}

/// Manages content push operations.
pub struct ContentPushManager {
    /// Active push requests by content_id string.
    pushes: HashMap<String, ContentPushRequest>,
    /// Push statuses by content_id string.
    statuses: HashMap<String, PushStatus>,
    /// Opt-in recipients (node IDs that accept content push).
    opted_in: HashSet<String>,
}

impl ContentPushManager {
    pub fn new() -> Self {
        Self {
            pushes: HashMap::new(),
            statuses: HashMap::new(),
            opted_in: HashSet::new(),
        }
    }

    /// Register a node as opted-in for content push.
    pub fn opt_in(&mut self, node_id: &NodeId) {
        self.opted_in.insert(node_id.0.clone());
    }

    /// Remove a node's content push opt-in.
    pub fn opt_out(&mut self, node_id: &NodeId) {
        self.opted_in.remove(&node_id.0);
    }

    /// Check if a node has opted in.
    pub fn is_opted_in(&self, node_id: &NodeId) -> bool {
        self.opted_in.contains(&node_id.0)
    }

    /// Submit a content push request.
    ///
    /// Filters `target_recipients` to only those nodes that have opted in.
    /// Returns the filtered list of actual recipients.
    pub fn submit_push(&mut self, request: ContentPushRequest) -> Vec<NodeId> {
        let content_key = request.content_id.0.clone();

        let actual_recipients: Vec<NodeId> = request
            .target_recipients
            .iter()
            .filter(|n| self.opted_in.contains(&n.0))
            .cloned()
            .collect();

        let status = if actual_recipients.is_empty() {
            PushStatus::Failed
        } else {
            PushStatus::Pending
        };

        self.statuses.insert(content_key.clone(), status);
        self.pushes.insert(content_key, request);

        actual_recipients
    }

    /// Get push status.
    pub fn get_status(&self, content_id: &str) -> Option<PushStatus> {
        self.statuses.get(content_id).copied()
    }

    /// Mark push as delivered.
    pub fn mark_delivered(&mut self, content_id: &str) -> Result<(), PushError> {
        if !self.statuses.contains_key(content_id) {
            return Err(PushError::NotFound(content_id.to_string()));
        }
        self.statuses
            .insert(content_id.to_string(), PushStatus::Delivered);
        Ok(())
    }

    /// Mark push as failed.
    pub fn mark_failed(&mut self, content_id: &str) -> Result<(), PushError> {
        if !self.statuses.contains_key(content_id) {
            return Err(PushError::NotFound(content_id.to_string()));
        }
        self.statuses
            .insert(content_id.to_string(), PushStatus::Failed);
        Ok(())
    }

    /// Count of active pushes (Pending or Delivering).
    pub fn active_push_count(&self) -> usize {
        self.statuses
            .values()
            .filter(|s| matches!(s, PushStatus::Pending | PushStatus::Delivering))
            .count()
    }
}

impl Default for ContentPushManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("content push not found: {0}")]
    NotFound(String),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn test_request(targets: Vec<NodeId>) -> ContentPushRequest {
        ContentPushRequest {
            content_id: AssetId::from("content-001"),
            publisher: NodeId::from("publisher-node"),
            fee_budget: GoldGrams::from_decimal(Decimal::new(1, 2)), // 0.01g
            target_recipients: targets,
            settlement_evp: None,
        }
    }

    #[test]
    fn opt_in_and_verify() {
        let mut mgr = ContentPushManager::new();
        let node = NodeId::from("node-a");

        assert!(!mgr.is_opted_in(&node));
        mgr.opt_in(&node);
        assert!(mgr.is_opted_in(&node));
        mgr.opt_out(&node);
        assert!(!mgr.is_opted_in(&node));
    }

    #[test]
    fn submit_push_filters_to_opted_in_only() {
        let mut mgr = ContentPushManager::new();

        let opted = NodeId::from("opted-node");
        let not_opted = NodeId::from("not-opted-node");
        mgr.opt_in(&opted);

        let request = test_request(vec![opted.clone(), not_opted]);
        let recipients = mgr.submit_push(request);

        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0], opted);
        assert_eq!(
            mgr.get_status("content-001"),
            Some(PushStatus::Pending)
        );
    }

    #[test]
    fn mark_delivered_and_failed() {
        let mut mgr = ContentPushManager::new();
        let node = NodeId::from("recipient");
        mgr.opt_in(&node);

        let request = test_request(vec![node]);
        mgr.submit_push(request);

        mgr.mark_delivered("content-001")
            .expect("test: mark delivered");
        assert_eq!(
            mgr.get_status("content-001"),
            Some(PushStatus::Delivered)
        );

        // Mark as failed after delivered (allowed -- status overwrite).
        mgr.mark_failed("content-001")
            .expect("test: mark failed");
        assert_eq!(
            mgr.get_status("content-001"),
            Some(PushStatus::Failed)
        );
    }

    #[test]
    fn non_opted_in_recipients_excluded() {
        let mut mgr = ContentPushManager::new();

        // No one opted in.
        let request = test_request(vec![
            NodeId::from("node-x"),
            NodeId::from("node-y"),
        ]);
        let recipients = mgr.submit_push(request);

        assert!(recipients.is_empty());
        // When no recipients are eligible, status is Failed.
        assert_eq!(
            mgr.get_status("content-001"),
            Some(PushStatus::Failed)
        );
        assert_eq!(mgr.active_push_count(), 0);
    }
}

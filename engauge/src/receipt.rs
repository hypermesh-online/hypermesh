// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Content receipt system with BLAKE3 hashing.
//!
//! Every unit of work in HyperMesh produces a [`ContentReceipt`] -- a
//! cryptographic proof that specific content was processed by a specific node
//! at a specific time.  Receipts are grouped into [`ReceiptBundle`]s for
//! batch verification and aggregation.

use chrono::{DateTime, Utc};
use hypermesh_lib::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// WorkUnits
// ---------------------------------------------------------------------------

/// Quantified work amount (bytes processed, compute cycles, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkUnits(pub u64);

impl WorkUnits {
    /// Zero work.
    pub fn zero() -> Self {
        Self(0)
    }

    /// Create from a raw count.
    pub fn new(units: u64) -> Self {
        Self(units)
    }
}

impl std::ops::Add for WorkUnits {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::fmt::Display for WorkUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}wu", self.0)
    }
}

// ---------------------------------------------------------------------------
// ContentReceipt
// ---------------------------------------------------------------------------

/// BLAKE3 hash of a work product together with provenance metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentReceipt {
    /// Unique receipt identifier.
    pub receipt_id: Uuid,
    /// BLAKE3 digest of the content that was processed.
    pub content_hash: [u8; 32],
    /// When the work was completed.
    pub timestamp: DateTime<Utc>,
    /// The node that performed the work.
    pub node_id: NodeId,
    /// Quantified work (e.g. byte count of the content).
    pub work: WorkUnits,
}

impl ContentReceipt {
    /// Create a new receipt by hashing `content` via BLAKE3.
    pub fn new(content: &[u8], node_id: NodeId) -> Self {
        let hash = blake3::hash(content);
        Self {
            receipt_id: Uuid::new_v4(),
            content_hash: *hash.as_bytes(),
            timestamp: Utc::now(),
            node_id,
            work: WorkUnits::new(content.len() as u64),
        }
    }

    /// Verify that `content` matches this receipt's stored hash.
    pub fn verify(&self, content: &[u8]) -> bool {
        let hash = blake3::hash(content);
        *hash.as_bytes() == self.content_hash
    }
}

// ---------------------------------------------------------------------------
// VerificationResult
// ---------------------------------------------------------------------------

/// Outcome of verifying a [`ReceiptBundle`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Number of receipts whose content matched.
    pub verified_count: u64,
    /// Number of receipts whose content did NOT match.
    pub failed_count: u64,
    /// Total work across verified receipts only.
    pub total_work: WorkUnits,
}

// ---------------------------------------------------------------------------
// ReceiptBundle
// ---------------------------------------------------------------------------

/// A collection of [`ContentReceipt`]s grouped by time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptBundle {
    /// Window start (inclusive).
    pub window_start: DateTime<Utc>,
    /// Window end (exclusive).
    pub window_end: DateTime<Utc>,
    /// Ordered receipts within this window.
    pub receipts: Vec<ContentReceipt>,
}

impl ReceiptBundle {
    /// Create an empty bundle for the given time window.
    pub fn new(window_start: DateTime<Utc>, window_end: DateTime<Utc>) -> Self {
        Self {
            window_start,
            window_end,
            receipts: Vec::new(),
        }
    }

    /// Add a receipt to the bundle.
    pub fn add_receipt(&mut self, receipt: ContentReceipt) {
        self.receipts.push(receipt);
    }

    /// Verify all receipts against a map of receipt_id -> content bytes.
    ///
    /// Receipts whose `receipt_id` is missing from the map count as failures.
    pub fn verify_all(&self, content_map: &HashMap<Uuid, Vec<u8>>) -> VerificationResult {
        let mut verified_count: u64 = 0;
        let mut failed_count: u64 = 0;
        let mut total_work = WorkUnits::zero();

        for receipt in &self.receipts {
            let ok = content_map
                .get(&receipt.receipt_id)
                .map(|data| receipt.verify(data))
                .unwrap_or(false);

            if ok {
                verified_count += 1;
                total_work = total_work + receipt.work;
            } else {
                failed_count += 1;
            }
        }

        VerificationResult {
            verified_count,
            failed_count,
            total_work,
        }
    }

    /// Sum of work units across every receipt in the bundle.
    pub fn total_work(&self) -> WorkUnits {
        self.receipts
            .iter()
            .fold(WorkUnits::zero(), |acc, r| acc + r.work)
    }

    /// Number of receipts.
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// Whether the bundle is empty.
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn test_node() -> NodeId {
        NodeId::from("test-node-001")
    }

    #[test]
    fn receipt_new_hashes_content() {
        let data = b"hello hypermesh";
        let receipt = ContentReceipt::new(data, test_node());

        assert_eq!(receipt.content_hash, *blake3::hash(data).as_bytes());
        assert_eq!(receipt.work, WorkUnits::new(data.len() as u64));
        assert_eq!(receipt.node_id, test_node());
    }

    #[test]
    fn receipt_verify_matching_content() {
        let data = b"some work product";
        let receipt = ContentReceipt::new(data, test_node());
        assert!(receipt.verify(data));
    }

    #[test]
    fn receipt_verify_mismatched_content() {
        let data = b"original data";
        let receipt = ContentReceipt::new(data, test_node());
        assert!(!receipt.verify(b"tampered data"));
    }

    #[test]
    fn receipt_verify_empty_content() {
        let data: &[u8] = b"";
        let receipt = ContentReceipt::new(data, test_node());
        assert!(receipt.verify(b""));
        assert!(!receipt.verify(b"not empty"));
    }

    #[test]
    fn work_units_arithmetic() {
        let a = WorkUnits::new(100);
        let b = WorkUnits::new(200);
        assert_eq!(a + b, WorkUnits::new(300));
        assert_eq!(WorkUnits::zero().0, 0);
    }

    #[test]
    fn work_units_saturating_add() {
        let max = WorkUnits::new(u64::MAX);
        let one = WorkUnits::new(1);
        assert_eq!(max + one, WorkUnits::new(u64::MAX));
    }

    #[test]
    fn bundle_add_and_total_work() {
        let now = Utc::now();
        let mut bundle = ReceiptBundle::new(now, now + Duration::hours(1));
        assert!(bundle.is_empty());

        let r1 = ContentReceipt::new(b"aaa", test_node());
        let r2 = ContentReceipt::new(b"bbbbbb", test_node());
        bundle.add_receipt(r1);
        bundle.add_receipt(r2);

        assert_eq!(bundle.len(), 2);
        assert!(!bundle.is_empty());
        assert_eq!(bundle.total_work(), WorkUnits::new(3 + 6));
    }

    #[test]
    fn bundle_verify_all_success() {
        let now = Utc::now();
        let mut bundle = ReceiptBundle::new(now, now + Duration::hours(1));

        let data1 = b"content-one";
        let data2 = b"content-two";
        let r1 = ContentReceipt::new(data1, test_node());
        let r2 = ContentReceipt::new(data2, test_node());
        let id1 = r1.receipt_id;
        let id2 = r2.receipt_id;
        bundle.add_receipt(r1);
        bundle.add_receipt(r2);

        let mut map = HashMap::new();
        map.insert(id1, data1.to_vec());
        map.insert(id2, data2.to_vec());

        let result = bundle.verify_all(&map);
        assert_eq!(result.verified_count, 2);
        assert_eq!(result.failed_count, 0);
        assert_eq!(
            result.total_work,
            WorkUnits::new(data1.len() as u64 + data2.len() as u64)
        );
    }

    #[test]
    fn bundle_verify_all_partial_failure() {
        let now = Utc::now();
        let mut bundle = ReceiptBundle::new(now, now + Duration::hours(1));

        let data1 = b"good-data";
        let r1 = ContentReceipt::new(data1, test_node());
        let r2 = ContentReceipt::new(b"will-be-missing", test_node());
        let id1 = r1.receipt_id;
        bundle.add_receipt(r1);
        bundle.add_receipt(r2);

        let mut map = HashMap::new();
        map.insert(id1, data1.to_vec());
        // r2's content is not in the map

        let result = bundle.verify_all(&map);
        assert_eq!(result.verified_count, 1);
        assert_eq!(result.failed_count, 1);
        assert_eq!(result.total_work, WorkUnits::new(data1.len() as u64));
    }

    #[test]
    fn bundle_verify_all_tampered_content() {
        let now = Utc::now();
        let mut bundle = ReceiptBundle::new(now, now + Duration::hours(1));

        let data = b"original";
        let receipt = ContentReceipt::new(data, test_node());
        let rid = receipt.receipt_id;
        bundle.add_receipt(receipt);

        let mut map = HashMap::new();
        map.insert(rid, b"tampered".to_vec());

        let result = bundle.verify_all(&map);
        assert_eq!(result.verified_count, 0);
        assert_eq!(result.failed_count, 1);
    }

    #[test]
    fn receipt_serde_roundtrip() {
        let receipt = ContentReceipt::new(b"serde-test", test_node());
        let json = serde_json::to_string(&receipt).expect("test: serialize receipt");
        let back: ContentReceipt = serde_json::from_str(&json).expect("test: deserialize receipt");
        assert_eq!(receipt.receipt_id, back.receipt_id);
        assert_eq!(receipt.content_hash, back.content_hash);
        assert_eq!(receipt.node_id, back.node_id);
        assert_eq!(receipt.work, back.work);
    }
}

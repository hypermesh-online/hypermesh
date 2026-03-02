// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy-Scoped Deduplication Verification (R4)
//!
//! Dedup behavior varies by privacy mode:
//! - **Device/Private** (tracked): Full reference tracking -- who stored,
//!   when, and access logs.
//! - **Anonymous** (untracked): Hash-only dedup -- content hash matches
//!   are recognized but no identity or timing metadata is recorded.
//!
//! Tamper detection is always active: stored BLAKE3 hashes are
//! recomputed and compared on every access.

use hypermesh_lib::PrivacyMode;
use std::collections::HashMap;

/// Hash type for content (BLAKE3 256-bit digest).
pub type ContentHash = [u8; 32];

/// Result of a dedup check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupResult {
    /// Content is unique -- not seen before.
    Unique,
    /// Duplicate detected with full reference tracking (Device/Private).
    Duplicate {
        /// Reference to the existing stored entry.
        existing_ref: StorageRef,
    },
    /// Duplicate detected via hash match only (Anonymous -- no tracking).
    DuplicateAnonymous,
    /// Stored hash does not match recomputed hash -- tamper detected.
    TamperDetected {
        expected: ContentHash,
        actual: ContentHash,
    },
}

/// Reference metadata tracked for non-anonymous dedup entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRef {
    /// Who stored this content (node identifier string).
    pub stored_by: String,
    /// Unix timestamp when content was first stored.
    pub stored_at: u64,
    /// Number of times this content has been referenced.
    pub ref_count: u64,
}

/// Internal record kept per unique content hash.
#[derive(Debug, Clone)]
struct DedupEntry {
    /// The verified BLAKE3 hash.
    _content_hash: ContentHash,
    /// Tracking metadata (None for anonymous entries).
    tracking: Option<StorageRef>,
    /// The raw data size in bytes (for tamper detection).
    _data_size: usize,
}

/// Privacy-scoped deduplication engine.
///
/// Stores entries keyed by content hash. Behavior on lookup depends
/// on the `PrivacyMode` of the caller.
pub struct PrivacyScopedDedup {
    entries: HashMap<ContentHash, DedupEntry>,
}

impl PrivacyScopedDedup {
    /// Create a new empty dedup engine.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Store content with dedup check.
    ///
    /// If the content hash already exists this is a no-op duplicate.
    /// For tracked modes the `stored_by` identity is recorded.
    pub fn store(
        &mut self,
        data: &[u8],
        privacy_mode: PrivacyMode,
        stored_by: &str,
        stored_at: u64,
    ) -> DedupResult {
        let hash = blake3_hash(data);

        if let Some(entry) = self.entries.get_mut(&hash) {
            // Content already stored -- this is a duplicate.
            if privacy_mode.tracked {
                // Tracked: increment ref count and return full reference.
                if let Some(ref mut tracking) = entry.tracking {
                    tracking.ref_count += 1;
                    return DedupResult::Duplicate {
                        existing_ref: tracking.clone(),
                    };
                }
            }
            // Anonymous or entry has no tracking.
            return DedupResult::DuplicateAnonymous;
        }

        // New unique content.
        let tracking = if privacy_mode.tracked {
            Some(StorageRef {
                stored_by: stored_by.to_string(),
                stored_at,
                ref_count: 1,
            })
        } else {
            None
        };

        self.entries.insert(
            hash,
            DedupEntry {
                _content_hash: hash,
                tracking,
                _data_size: data.len(),
            },
        );

        DedupResult::Unique
    }

    /// Check if content is a duplicate without storing it.
    ///
    /// Privacy mode controls whether tracking info is returned.
    pub fn check_duplicate(
        &self,
        data: &[u8],
        privacy_mode: PrivacyMode,
    ) -> DedupResult {
        let hash = blake3_hash(data);

        match self.entries.get(&hash) {
            None => DedupResult::Unique,
            Some(entry) => {
                if privacy_mode.tracked {
                    if let Some(ref tracking) = entry.tracking {
                        return DedupResult::Duplicate {
                            existing_ref: tracking.clone(),
                        };
                    }
                }
                DedupResult::DuplicateAnonymous
            }
        }
    }

    /// Verify stored content integrity by recomputing the BLAKE3 hash.
    ///
    /// Returns `Ok(())` if hash matches, or a `TamperDetected` result
    /// if the data has been modified.
    pub fn verify_integrity(
        &self,
        stored_hash: &ContentHash,
        current_data: &[u8],
    ) -> DedupResult {
        let computed = blake3_hash(current_data);
        if computed == *stored_hash {
            // Hash matches -- check if we know about this content.
            match self.entries.get(stored_hash) {
                Some(entry) => {
                    if let Some(ref tracking) = entry.tracking {
                        DedupResult::Duplicate {
                            existing_ref: tracking.clone(),
                        }
                    } else {
                        DedupResult::DuplicateAnonymous
                    }
                }
                None => DedupResult::Unique,
            }
        } else {
            DedupResult::TamperDetected {
                expected: *stored_hash,
                actual: computed,
            }
        }
    }

    /// Number of unique entries stored.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for PrivacyScopedDedup {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute BLAKE3 hash of data.
fn blake3_hash(data: &[u8]) -> ContentHash {
    *blake3::hash(data).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::PrivacyMode;

    #[test]
    fn test_device_private_full_tracking() {
        let mut dedup = PrivacyScopedDedup::new();
        let data = b"hello world";

        // First store: unique.
        let result = dedup.store(data, PrivacyMode::PRIVATE, "node-1", 1000);
        assert_eq!(result, DedupResult::Unique);

        // Second store: duplicate with full tracking.
        let result = dedup.store(data, PrivacyMode::PRIVATE, "node-2", 2000);
        match result {
            DedupResult::Duplicate { existing_ref } => {
                assert_eq!(existing_ref.stored_by, "node-1");
                assert_eq!(existing_ref.stored_at, 1000);
                assert_eq!(existing_ref.ref_count, 2);
            }
            other => unreachable!("expected Duplicate, got {other:?}"),
        }

        // Check without storing.
        let check = dedup.check_duplicate(data, PrivacyMode::PRIVATE);
        match check {
            DedupResult::Duplicate { existing_ref } => {
                assert_eq!(existing_ref.ref_count, 2);
            }
            other => unreachable!("expected Duplicate, got {other:?}"),
        }
    }

    #[test]
    fn test_anonymous_hash_only() {
        let mut dedup = PrivacyScopedDedup::new();
        let data = b"anonymous content";

        // Store as anonymous -- no tracking.
        let result = dedup.store(data, PrivacyMode::ANONYMOUS, "", 0);
        assert_eq!(result, DedupResult::Unique);

        // Duplicate check as anonymous -- hash-only match.
        let result = dedup.check_duplicate(data, PrivacyMode::ANONYMOUS);
        assert_eq!(result, DedupResult::DuplicateAnonymous);

        // Even from a tracked context, there's no tracking metadata to return.
        let result = dedup.check_duplicate(data, PrivacyMode::PRIVATE);
        assert_eq!(result, DedupResult::DuplicateAnonymous);
    }

    #[test]
    fn test_tamper_detection() {
        let dedup = PrivacyScopedDedup::new();
        let data = b"original data";
        let hash = blake3_hash(data);

        // Verify with correct data.
        let result = dedup.verify_integrity(&hash, data);
        // Not stored, so returns Unique (hash matches but not in store).
        assert_eq!(result, DedupResult::Unique);

        // Tampered data.
        let tampered = b"tampered data";
        let result = dedup.verify_integrity(&hash, tampered);
        match result {
            DedupResult::TamperDetected { expected, actual } => {
                assert_eq!(expected, hash);
                assert_ne!(actual, hash);
            }
            other => unreachable!("expected TamperDetected, got {other:?}"),
        }
    }

    #[test]
    fn test_public_mode_with_tracking() {
        let mut dedup = PrivacyScopedDedup::new();
        let data = b"public content";

        // Public mode is tracked (Unbounded + tracked).
        let result = dedup.store(data, PrivacyMode::PUBLIC, "node-pub", 5000);
        assert_eq!(result, DedupResult::Unique);

        let result = dedup.store(data, PrivacyMode::PUBLIC, "node-pub-2", 6000);
        match result {
            DedupResult::Duplicate { existing_ref } => {
                assert_eq!(existing_ref.stored_by, "node-pub");
                assert_eq!(existing_ref.ref_count, 2);
            }
            other => unreachable!("expected Duplicate, got {other:?}"),
        }
    }

    #[test]
    fn test_different_content_not_duplicates() {
        let mut dedup = PrivacyScopedDedup::new();

        let result1 = dedup.store(b"content-a", PrivacyMode::PRIVATE, "n1", 100);
        let result2 = dedup.store(b"content-b", PrivacyMode::PRIVATE, "n1", 200);

        assert_eq!(result1, DedupResult::Unique);
        assert_eq!(result2, DedupResult::Unique);
        assert_eq!(dedup.entry_count(), 2);
    }

    #[test]
    fn test_tamper_detection_with_stored_entry() {
        let mut dedup = PrivacyScopedDedup::new();
        let data = b"stored and verified";

        // Store the data first.
        dedup.store(data, PrivacyMode::PRIVATE, "node-1", 1000);
        let hash = blake3_hash(data);

        // Verify with correct data -- should find the tracked entry.
        let result = dedup.verify_integrity(&hash, data);
        match result {
            DedupResult::Duplicate { existing_ref } => {
                assert_eq!(existing_ref.stored_by, "node-1");
            }
            other => unreachable!("expected Duplicate, got {other:?}"),
        }

        // Verify with tampered data.
        let result = dedup.verify_integrity(&hash, b"wrong data");
        assert!(matches!(result, DedupResult::TamperDetected { .. }));
    }
}

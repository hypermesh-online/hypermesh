// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy-scoped shard deduplication policy (R4).
//!
//! Full dedup in Device/Private scope (same BLAKE3 hash = one copy).
//! No dedup in Anonymous scope (each asset gets independent copies).

use crate::bootstrap::PrivacyMode;

/// Dedup policy for shard storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupPolicy {
    /// Device + Private: full dedup, shared shards with refcount.
    Full,
    /// Anonymous: no dedup, each asset stores independent copies.
    None,
}

impl DedupPolicy {
    /// Derive dedup policy from privacy mode.
    ///
    /// Anonymous mode disables dedup (each asset stores independent copies)
    /// to prevent cross-asset correlation. All other modes enable full
    /// content-addressed deduplication.
    pub fn from_privacy_mode(mode: &PrivacyMode) -> Self {
        if *mode == PrivacyMode::ANONYMOUS {
            DedupPolicy::None
        } else {
            DedupPolicy::Full
        }
    }
}

/// Result of a dedup-aware store operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardStoreResult {
    /// Shard was newly stored.
    Stored,
    /// Shard already existed, reference count incremented.
    Deduplicated { ref_count: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_yields_no_dedup() {
        let policy = DedupPolicy::from_privacy_mode(&PrivacyMode::ANONYMOUS);
        assert_eq!(policy, DedupPolicy::None);
    }

    #[test]
    fn private_yields_full_dedup() {
        let policy = DedupPolicy::from_privacy_mode(&PrivacyMode::PRIVATE);
        assert_eq!(policy, DedupPolicy::Full);
    }

    #[test]
    fn public_yields_full_dedup() {
        let policy = DedupPolicy::from_privacy_mode(&PrivacyMode::PUBLIC);
        assert_eq!(policy, DedupPolicy::Full);
    }

    #[test]
    fn dedup_policy_clone_and_copy() {
        let a = DedupPolicy::Full;
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn shard_store_result_variants() {
        let stored = ShardStoreResult::Stored;
        let deduped = ShardStoreResult::Deduplicated { ref_count: 3 };
        assert_ne!(stored, deduped);
        assert_eq!(stored.clone(), ShardStoreResult::Stored);
        assert_eq!(
            deduped.clone(),
            ShardStoreResult::Deduplicated { ref_count: 3 }
        );
    }
}

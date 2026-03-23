// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy-scoped shard deduplication policy (R4).
//!
//! Full dedup in Device/Private scope (same BLAKE3 hash = one copy, refcount tracked).
//! HashOnly dedup in Anonymous scope (detects dupes by hash to save storage, but
//! no refcount tracking and no provider identity registration).

use crate::bootstrap::PrivacyMode;

/// Dedup policy for shard storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupPolicy {
    /// Device + Private: full dedup, shared shards with refcount.
    Full,
    /// Anonymous (legacy): no dedup, each asset stores independent copies.
    None,
    /// Anonymous (R4): detect dupes by hash to save storage, but no
    /// refcount tracking and no provider identity registration.
    HashOnly,
}

impl DedupPolicy {
    /// Derive dedup policy from privacy mode.
    ///
    /// Anonymous mode disables dedup (each asset stores independent copies)
    /// to prevent cross-asset correlation. All other modes enable full
    /// content-addressed deduplication.
    pub fn from_privacy_mode(mode: &PrivacyMode) -> Self {
        if *mode == PrivacyMode::ANONYMOUS {
            DedupPolicy::HashOnly
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

/// Status of a shard after dedup check against the local store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardStatus {
    /// Shard needs to be distributed (new or Anonymous/None policy).
    NeedsDistribution,
    /// Shard already exists in store, reference acquired.
    AlreadyStored { ref_count: u32 },
}

/// Dedup shards against the local store before distribution.
///
/// For each shard:
/// - If `DedupPolicy::Full` and shard exists: `acquire()` a reference, mark `AlreadyStored`.
/// - Otherwise: mark `NeedsDistribution`.
///
/// The content hash is computed from each shard's data using BLAKE3.
pub async fn dedup_shards(
    shards: &[crate::assets::pipeline::Shard],
    store: &super::shard_store::ShardStore,
    policy: DedupPolicy,
) -> Vec<ShardStatus> {
    let mut results = Vec::with_capacity(shards.len());

    for shard in shards {
        let hash_bytes: [u8; 32] = *blake3::hash(&shard.data).as_bytes();
        let content_hash = hypermesh_lib::ContentHash(hash_bytes);

        let status = match policy {
            DedupPolicy::Full => {
                if store.has(&content_hash).await {
                    match store.acquire(&content_hash).await {
                        Some(rc) => ShardStatus::AlreadyStored { ref_count: rc },
                        // Race: shard was removed between has() and acquire()
                        None => ShardStatus::NeedsDistribution,
                    }
                } else {
                    ShardStatus::NeedsDistribution
                }
            }
            DedupPolicy::HashOnly => {
                // Detect duplicate by hash (saves storage) but do NOT
                // acquire a reference — no refcount tracking for Anonymous.
                if store.has(&content_hash).await {
                    ShardStatus::AlreadyStored { ref_count: 1 }
                } else {
                    ShardStatus::NeedsDistribution
                }
            }
            DedupPolicy::None => ShardStatus::NeedsDistribution,
        };

        results.push(status);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_yields_hash_only_dedup() {
        let policy = DedupPolicy::from_privacy_mode(&PrivacyMode::ANONYMOUS);
        assert_eq!(policy, DedupPolicy::HashOnly);
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

    #[test]
    fn shard_status_variants() {
        let needs = ShardStatus::NeedsDistribution;
        let stored = ShardStatus::AlreadyStored { ref_count: 2 };
        assert_ne!(needs, stored);
        assert_eq!(needs.clone(), ShardStatus::NeedsDistribution);
        assert_eq!(stored.clone(), ShardStatus::AlreadyStored { ref_count: 2 });
    }

    fn make_test_shard(data: &[u8]) -> crate::assets::pipeline::Shard {
        crate::assets::pipeline::Shard {
            data: data.to_vec(),
            metadata: crate::assets::pipeline::sharding::ShardMetadata {
                index: 0,
                is_parity: false,
                size: data.len(),
                original_size: data.len(),
                hash: hex::encode(blake3::hash(data).as_bytes()),
            },
        }
    }

    #[tokio::test]
    async fn test_dedup_shards_full_policy_new_shards() {
        let store = super::super::shard_store::ShardStore::new();
        let shard = make_test_shard(b"brand-new-data");

        let results = dedup_shards(&[shard], &store, DedupPolicy::Full).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ShardStatus::NeedsDistribution);
    }

    #[tokio::test]
    async fn test_dedup_shards_full_policy_existing_shard() {
        let store = super::super::shard_store::ShardStore::new();
        let data = b"existing-data-in-store";
        let hash_bytes: [u8; 32] = *blake3::hash(data).as_bytes();
        let content_hash = hypermesh_lib::ContentHash(hash_bytes);

        // Pre-store the shard
        store.store(content_hash, data.to_vec()).await;

        let shard = make_test_shard(data);
        let results = dedup_shards(&[shard], &store, DedupPolicy::Full).await;
        assert_eq!(results.len(), 1);
        assert!(
            matches!(results[0], ShardStatus::AlreadyStored { ref_count: 2 }),
            "expected AlreadyStored with ref_count=2, got {:?}",
            results[0],
        );
    }

    #[tokio::test]
    async fn test_dedup_shards_none_policy_always_needs_distribution() {
        let store = super::super::shard_store::ShardStore::new();
        let data = b"data-for-anon";
        let hash_bytes: [u8; 32] = *blake3::hash(data).as_bytes();
        let content_hash = hypermesh_lib::ContentHash(hash_bytes);

        // Pre-store the shard
        store.store(content_hash, data.to_vec()).await;

        let shard = make_test_shard(data);
        let results = dedup_shards(&[shard], &store, DedupPolicy::None).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ShardStatus::NeedsDistribution);
    }

    #[tokio::test]
    async fn test_dedup_shards_hash_only_detects_existing() {
        let store = super::super::shard_store::ShardStore::new();
        let data = b"hash-only-existing";
        let hash_bytes: [u8; 32] = *blake3::hash(data).as_bytes();
        let content_hash = hypermesh_lib::ContentHash(hash_bytes);

        // Pre-store the shard
        store.store(content_hash, data.to_vec()).await;

        let shard = make_test_shard(data);
        let results = dedup_shards(&[shard], &store, DedupPolicy::HashOnly).await;
        assert_eq!(results.len(), 1);
        assert!(
            matches!(results[0], ShardStatus::AlreadyStored { ref_count: 1 }),
            "expected AlreadyStored with ref_count=1 (no acquire), got {:?}",
            results[0],
        );

        // Refcount should still be 1 — HashOnly does NOT call acquire()
        assert_eq!(store.ref_count(&content_hash).await, Some(1));
    }

    #[tokio::test]
    async fn test_dedup_shards_hash_only_new_shard() {
        let store = super::super::shard_store::ShardStore::new();
        let shard = make_test_shard(b"hash-only-brand-new");

        let results = dedup_shards(&[shard], &store, DedupPolicy::HashOnly).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ShardStatus::NeedsDistribution);
    }

    #[tokio::test]
    async fn test_hash_only_and_full_policies_work_independently() {
        let store = super::super::shard_store::ShardStore::new();
        let data = b"shared-data-for-policy-test";
        let hash_bytes: [u8; 32] = *blake3::hash(data).as_bytes();
        let content_hash = hypermesh_lib::ContentHash(hash_bytes);

        // Pre-store
        store.store(content_hash, data.to_vec()).await;
        assert_eq!(store.ref_count(&content_hash).await, Some(1));

        let shard = make_test_shard(data);

        // HashOnly: detects existing but does NOT bump refcount
        let results_ho = dedup_shards(&[shard.clone()], &store, DedupPolicy::HashOnly).await;
        assert!(matches!(
            results_ho[0],
            ShardStatus::AlreadyStored { ref_count: 1 }
        ));
        assert_eq!(store.ref_count(&content_hash).await, Some(1));

        // Full: detects existing AND bumps refcount via acquire()
        let results_full = dedup_shards(&[shard.clone()], &store, DedupPolicy::Full).await;
        assert!(matches!(
            results_full[0],
            ShardStatus::AlreadyStored { ref_count: 2 }
        ));
        assert_eq!(store.ref_count(&content_hash).await, Some(2));

        // None: always NeedsDistribution regardless of store contents
        let results_none = dedup_shards(&[shard], &store, DedupPolicy::None).await;
        assert_eq!(results_none[0], ShardStatus::NeedsDistribution);
    }

    #[tokio::test]
    async fn test_dedup_shards_mixed_new_and_existing() {
        let store = super::super::shard_store::ShardStore::new();

        let existing_data = b"already-here";
        let hash_bytes: [u8; 32] = *blake3::hash(existing_data as &[u8]).as_bytes();
        store
            .store(hypermesh_lib::ContentHash(hash_bytes), existing_data.to_vec())
            .await;

        let shard_existing = make_test_shard(existing_data);
        let shard_new = make_test_shard(b"never-seen-before");

        let results = dedup_shards(
            &[shard_existing, shard_new],
            &store,
            DedupPolicy::Full,
        )
        .await;

        assert_eq!(results.len(), 2);
        assert!(matches!(results[0], ShardStatus::AlreadyStored { .. }));
        assert_eq!(results[1], ShardStatus::NeedsDistribution);
    }
}

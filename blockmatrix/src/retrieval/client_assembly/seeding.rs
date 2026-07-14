// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Become-a-provider seeding hook for the client-assembly fetch path (A2).
//!
//! "Clients ARE mirrors ARE hosts." When the [`ClientAssembler`] fetches and
//! BLAKE3-verifies a shard, it must — exactly like the live IPC fetch path —
//! store it and re-announce availability so the consumer becomes a provider
//! (R12). Previously the client-assembly path skipped this; A2 folds it onto
//! the SAME [`ConsumerProviderManager`] the IPC path uses, so there is ONE
//! become-provider authority.
//!
//! The hook is a trait so the assembler stays usable in pure unit tests (which
//! pass no hook, matching the IPC path's "no manager wired" fallback of caching
//! locally without announcing). Production wires a
//! [`ConsumerProviderSeeder`] backed by the daemon's `ConsumerProviderManager`
//! plus its connected-peer broadcast.

use std::sync::Arc;

use async_trait::async_trait;
use hypermesh_lib::ContentHash;

use crate::network::consumer_provider::{broadcast_announcement, ConsumerProviderManager};

/// A sink that turns a freshly fetched, BLAKE3-verified shard into a
/// become-provider re-announce.
///
/// Implementations MUST NOT be given a shard until its data has been verified
/// to hash to `content_hash` — the seeder trusts the caller's content gate and
/// re-announces the shard to the swarm.
#[async_trait]
pub trait ShardSeeder: Send + Sync {
    /// Seed a verified shard: store it, register self as provider, and
    /// re-announce to the swarm. Returns the number of peers the announcement
    /// reached (0 when there is nothing to broadcast to).
    async fn seed(&self, content_hash: ContentHash, data: Vec<u8>) -> usize;
}

/// Production seeder: routes verified shards through the shared
/// [`ConsumerProviderManager`] (store + provider-register + build announce) and
/// broadcasts the resulting `TAG_SHARD_ANNOUNCE` payload to connected peers.
///
/// This is the SAME sequence the live IPC `shard.fetch` handler performs, so
/// both fetch paths seed identically.
pub struct ConsumerProviderSeeder {
    manager: Arc<ConsumerProviderManager>,
    connections: Vec<Arc<stoq::Connection>>,
}

impl ConsumerProviderSeeder {
    /// Create a seeder from the shared manager and the current connected-peer
    /// connection set to broadcast announcements to.
    pub fn new(
        manager: Arc<ConsumerProviderManager>,
        connections: Vec<Arc<stoq::Connection>>,
    ) -> Self {
        Self {
            manager,
            connections,
        }
    }
}

#[async_trait]
impl ShardSeeder for ConsumerProviderSeeder {
    async fn seed(&self, content_hash: ContentHash, data: Vec<u8>) -> usize {
        let result = self
            .manager
            .process_fetched_shards(vec![(content_hash, data)])
            .await;
        match result.announcement_payload {
            Some(payload) => broadcast_announcement(&payload, &self.connections).await,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::shard_store::ShardStore;
    use crate::network::swarm_provider::ShardLocationIndex;

    /// A verified shard routed through the production seeder is stored and
    /// registers the local node as a provider (become-provider), even with no
    /// peers to broadcast to.
    #[tokio::test]
    async fn test_seeder_registers_local_provider() {
        let store = Arc::new(ShardStore::new());
        let index = Arc::new(ShardLocationIndex::new());
        let manager = Arc::new(ConsumerProviderManager::new(
            store.clone(),
            index.clone(),
            "local-seed-node".to_string(),
        ));
        let seeder = ConsumerProviderSeeder::new(manager, Vec::new());

        let data = vec![0xEE; 64];
        let hash = ContentHash(*blake3::hash(&data).as_bytes());

        // No peers → 0 announce targets, but local provider registration happens.
        let reached = seeder.seed(hash, data.clone()).await;
        assert_eq!(reached, 0);

        assert_eq!(store.get(&hash).await, Some(data));
        let providers = index.get_providers(&hash).await;
        assert!(providers.contains(&"local-seed-node".to_string()));
    }
}

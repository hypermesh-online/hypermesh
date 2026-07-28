// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! World-scoped fetch gate — the mounted boundary enforcer.
//!
//! `DefaultIsolationManager` is the dormant enforcer; this is the thin,
//! single-purpose surface the live node consults on the replication/fetch
//! path (VISION.md §5.5 — worlds are emergent, nestable networks; a node is
//! responsible for the locality of its own world).
//!
//! The gate binds one node's *home world* to the enforcer and answers one
//! question: **may this node fetch/replicate a shard belonging to
//! `shard_world`?** Same-world traffic passes (a strict no-op while only
//! [`GLOBAL_WORLD`](hypermesh_lib::GLOBAL_WORLD) exists); a shard belonging to
//! a foreign world is rejected before any transfer is attempted, logged at
//! `warn!`.
//!
//! This is intentionally permissive-for-same-world: until a second world
//! forms (P6), `check_fetch` can only ever be handed the node's own world, so
//! it always accepts. The gate *exists and is consulted*; it rejects genuine
//! cross-world traffic only, of which there is none in a single-world node.

use anyhow::Result;
use hypermesh_lib::{ContentHash, NetworkId};
use std::sync::Arc;
use tracing::warn;

use super::{DefaultIsolationManager, IsolationManager, IsolationStats, IsolationViolation};
use crate::network::trust::NetworkType;

/// A mounted, world-scoped boundary gate over [`DefaultIsolationManager`].
pub struct WorldIsolationGate {
    /// The dormant enforcer, now mounted and configured for `local_world`.
    manager: Arc<DefaultIsolationManager>,
    /// This node's home world — the only world it serves shards for.
    local_world: NetworkId,
}

impl WorldIsolationGate {
    /// Mount the enforcer for a node whose home world is `local_world`.
    ///
    /// Configures the isolation manager with the home world so that
    /// same-world probes pass the packet filter. `network_type` records the
    /// world's trust posture for observability only.
    pub async fn mount(local_world: NetworkId, network_type: NetworkType) -> Result<Self> {
        let manager = Arc::new(DefaultIsolationManager::new());
        manager
            .configure_network(local_world, network_type)
            .await?;
        Ok(Self {
            manager,
            local_world,
        })
    }

    /// The node's home world.
    pub fn local_world(&self) -> NetworkId {
        self.local_world
    }

    /// Consult the boundary enforcer before fetching/replicating a shard that
    /// belongs to `shard_world`.
    ///
    /// - **Same world** (`shard_world == local_world`) → `Ok(())`. Until worlds
    ///   form this is the only reachable case, so the live path is a no-op.
    /// - **Cross world** → `Err(..)`, logged at `warn!` (INFO-visible), with
    ///   the violation recorded in the enforcer's audit log.
    pub async fn check_fetch(
        &self,
        shard_world: NetworkId,
        shard_id: &ContentHash,
    ) -> Result<()> {
        match self
            .manager
            .validate_boundary(self.local_world, shard_world)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!(
                    "world-isolation: rejected cross-world fetch of shard {} \
                     (home world {}, shard world {})",
                    hex::encode(&shard_id.0[..4]),
                    self.local_world,
                    shard_world,
                );
                Err(e)
            }
        }
    }

    /// Recorded boundary violations (audit trail for tests / observability).
    pub async fn violations(&self) -> Vec<IsolationViolation> {
        self.manager.check_violations().await
    }

    /// Enforcer statistics (packets validated/rejected, violations).
    pub async fn stats(&self) -> IsolationStats {
        self.manager.get_stats().await
    }
}

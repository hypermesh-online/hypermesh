// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DMS executor — blockmatrix's thin I/O side of the DMS seam.
//!
//! NGauge decides ([`ngauge::DmsDriver::plan`]); this executes. The
//! [`StoqDmsExecutor`] implements the ngauge-defined [`ngauge::MirrorExecutor`]
//! and [`ngauge::ReflectExecutor`] traits over the SAME `Arc`s the replication
//! service already holds — no new state, no decision logic. The mirror path is
//! the exact fetch → register-provider → set-replica-count sequence that used
//! to live inline in `poll.rs` (P6 convergence feedback), preserved 1:1.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use ngauge::{DmsError, MirrorAction, MirrorExecutor, ReflectAction, ReflectExecutor};

use crate::network::shard_transport::{ShardTransport, StoqShardTransport};
use crate::network::swarm_provider::ShardLocationIndex;

/// STOQ-backed executor for [`ngauge::DmsPlan`] actions.
///
/// Wraps the replication service's transport, provider index, node id, and
/// swarm-analytics handle — all clones of the Arcs the service already owns.
pub(super) struct StoqDmsExecutor {
    /// Shard transport used to pull replicas.
    transport: Arc<StoqShardTransport>,
    /// Shared provider index (R12) — where new providers are registered.
    index: Arc<ShardLocationIndex>,
    /// This node's id (registered as the new provider after a fetch).
    node_id: String,
    /// Swarm analytics — the replica-count convergence feedback sink.
    analytics: Arc<Mutex<ngauge::SwarmAnalytics>>,
}

impl StoqDmsExecutor {
    /// Build the executor from the replication service's existing handles.
    pub(super) fn new(
        transport: Arc<StoqShardTransport>,
        index: Arc<ShardLocationIndex>,
        node_id: String,
        analytics: Arc<Mutex<ngauge::SwarmAnalytics>>,
    ) -> Self {
        Self {
            transport,
            index,
            node_id,
            analytics,
        }
    }
}

#[async_trait]
impl MirrorExecutor for StoqDmsExecutor {
    /// Fetch a replica of the shard, register this node as a new provider, and
    /// report the new replica count. This is the P6 feedback loop: registering
    /// grows the provider count, `set_replica_count_in_network` reports it back
    /// so `ReplicationTrigger::check` sees `needed <= replicas` next tick and
    /// converges. An empty/missed fetch is a failure — no registration.
    async fn fetch_and_register(&self, action: &MirrorAction) -> Result<u32, DmsError> {
        let data = self
            .transport
            .fetch_shard(&action.source, &action.shard_id)
            .await
            .map_err(|e| DmsError::Fetch(e.to_string()))?;
        if data.is_empty() {
            return Err(DmsError::Fetch("empty shard".to_string()));
        }

        self.index
            .register_provider_in_network(action.network, &self.node_id, &[action.shard_id])
            .await;
        let replica_count = self
            .index
            .get_providers_in_network(action.network, &action.shard_id)
            .await
            .len() as u32;

        if let Ok(mut guard) = self.analytics.lock() {
            guard.set_replica_count_in_network(action.network, action.shard_id, replica_count);
        }
        Ok(replica_count)
    }
}

#[async_trait]
impl ReflectExecutor for StoqDmsExecutor {
    /// Ensure a held shard is announced by registering this node as a provider
    /// for it in the given network (the local half of the consumer-becomes-
    /// provider announce). The replication poll loop emits no reflect actions
    /// today, so this stays dormant until the Phase-4 head observer drives it.
    async fn announce(&self, action: &ReflectAction) -> Result<(), DmsError> {
        self.index
            .register_provider_in_network(action.network, &self.node_id, &[action.shard_id])
            .await;
        Ok(())
    }
}

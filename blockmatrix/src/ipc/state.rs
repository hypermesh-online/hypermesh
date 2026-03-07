// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Shared daemon state accessible to IPC handlers.

use crate::blockchain::node_chain::NodeBlockchain;
use crate::bootstrap::DnsResolver;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_store::ShardStore;
use crate::network::NetworkManager;
use crate::persistence::PersistenceManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Shared state accessible to IPC handlers when the daemon is running.
pub struct DaemonState {
    /// The node's blockchain instance.
    pub blockchain: Arc<NodeBlockchain>,
    /// Persistence layer for block storage.
    pub persistence: Arc<PersistenceManager>,
    /// Network manager (None if running in Private mode with no STOQ).
    pub network: Option<Arc<NetworkManager>>,
    /// Local shard store for serving and caching shards.
    pub shard_store: Arc<ShardStore>,
    /// This node's matrix coordinate.
    pub coordinate: MatrixCoordinate,
    /// Unique node identifier derived from the coordinate.
    pub node_id: String,
    /// On-disk data directory for this node.
    pub data_dir: PathBuf,
    /// Current privacy mode as a display string.
    pub privacy_mode: String,
    /// Timestamp when the daemon started.
    pub started_at: Instant,
    /// Channel to signal daemon shutdown from IPC.
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
    /// Bootstrap DNS resolver for name resolution.
    pub dns_resolver: DnsResolver,
}

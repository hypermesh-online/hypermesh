// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Shared daemon state accessible to IPC handlers.

use crate::blockchain::node_chain::NodeBlockchain;
use crate::bootstrap::DnsResolver;
use crate::dns::DnsPopularityTracker;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::consumer_provider::ConsumerProviderManager;
use crate::network::shard_store::ShardStore;
use crate::network::shard_transport::StoqShardTransport;
use crate::network::swarm_provider::ShardLocationIndex;
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
    /// STOQ shard transport for distributing shards to peers (None without network).
    pub shard_transport: Option<Arc<StoqShardTransport>>,
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
    /// DNS popularity tracker for engauge-driven replication.
    /// Records resolution frequency so popular names get replicated to more nodes.
    pub dns_popularity_tracker: Option<Arc<DnsPopularityTracker>>,
    /// Shard location index — same instance shared with PeerContext so
    /// TAG_SHARD_ANNOUNCE updates from peers and provider registrations from
    /// local fetches converge on a single canonical view.
    pub shard_location_index: Option<Arc<ShardLocationIndex>>,
    /// Consumer-becomes-provider manager (R12). When `Some`, IPC fetch handlers
    /// route fetched shards through `process_fetched_shards` and broadcast
    /// the resulting TAG_SHARD_ANNOUNCE payload to connected peers.
    pub consumer_provider_manager: Option<Arc<ConsumerProviderManager>>,
    /// Caesar EVP protocol instance (None if feature disabled or init failed).
    #[cfg(feature = "caesar")]
    pub caesar: Option<Arc<tokio::sync::RwLock<caesar::CaesarProtocol>>>,
    /// Engauge swarm analytics bridge (None if feature disabled or not wired).
    #[cfg(feature = "intelligence")]
    pub engauge_bridge: Option<Arc<crate::intelligence::engauge_bridge::EngaugeBridge>>,
    /// Phase F.1: federation manager (CA-side trust + key shares).
    #[cfg(feature = "intelligence")]
    pub federation_manager: Option<Arc<trustchain::ca::FederationManager>>,
    /// Phase F.1: threshold-sign coordinator (drives federated CAs).
    #[cfg(feature = "intelligence")]
    pub threshold_coordinator: Option<Arc<trustchain::crypto::ThresholdSignCoordinator>>,
    /// Phase G.1: cross-network transfer coordinator.
    ///
    /// Alpha-default inert: when `None`, `gateway.initiate_transfer` IPC
    /// returns [`GatewayError::CoordinatorNotConfigured`]. Wired by the
    /// daemon at startup once federation gating + STOQ wire transport
    /// are opted-in.
    pub transfer_coordinator: Option<Arc<crate::gateway::TransferCoordinator>>,
}

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
    /// DNS popularity tracker for ngauge-driven replication.
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
    /// NGauge swarm analytics bridge (None if feature disabled or not wired).
    #[cfg(feature = "intelligence")]
    pub ngauge_bridge: Option<Arc<crate::intelligence::ngauge_bridge::NGaugeBridge>>,
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

    /// Phase I.1: cross-chain receipt validator. Always-on (pure read
    /// structure, no security risk). Populated by the
    /// `TransferCoordinator` when receipts are written, and rebuilt
    /// from the chain at daemon startup so persisted receipts survive
    /// restart. Queried by `chain.lookup_cross_receipt` IPC handler.
    pub receipt_validator: Arc<crate::assets::cross_chain::CrossChainReceiptValidator>,

    /// Phase H.1: foundation FALCON-1024 signing identity for issuing
    /// DNS reservation grants.
    ///
    /// Alpha-default inert: when `None`, `dns.foundation_grant` IPC
    /// returns "foundation root key not configured". Operators who
    /// run a foundation node opt-in via daemon config by populating
    /// this field at startup with the foundation root identity.
    ///
    /// The corresponding public key is `key.public_key` and is what
    /// `DnsRegistrar::set_foundation_pubkey` is configured with so
    /// that grants can be verified at registration time.
    pub foundation_signing_key: Option<Arc<trustchain::FalconIdentity>>,

    /// Phase H.1: DNS registrar holding the reserved-domain enforcement
    /// + foundation grant verification logic.
    ///
    /// When `None`, the unified `dns.*` handlers fall back to the legacy
    /// flat-resolver behaviour (no reserved-domain checks beyond what is
    /// in `dns/reserved.rs`). Wired by the daemon at startup so that
    /// `dns.register` and `dns.register_with_grant` route through this
    /// registrar.
    pub dns_registrar: Option<Arc<crate::dns::DnsRegistrar>>,

    /// Phase J.1: foundation release-feed subscriber — caches signed
    /// release entries (`release.feed/v1` catalog asset) per channel
    /// and surfaces "update available" via `system.check_update`.
    ///
    /// Alpha-default inert: even when this field is `Some`, the
    /// subscriber starts with no foundation pubkey so all `ingest`
    /// calls reject with `NotConfigured` until the operator opts in.
    /// `system.check_update` returns "no foundation pubkey configured"
    /// when the subscriber is `None`.
    pub release_feed_subscriber: Option<Arc<crate::release_feed::ReleaseFeedSubscriber>>,

    /// Phase K.1: capability-token issuer (FALCON-1024 signing) for
    /// `auth.create_session`.
    ///
    /// Alpha-default inert: when `None`, `auth.create_session` returns
    /// "auth not configured". Operators opt in by populating the field
    /// at daemon startup with the daemon's FALCON identity.
    pub capability_token_issuer: Option<Arc<crate::auth::CapabilityTokenIssuer>>,

    /// Phase K.1: in-memory revocation registry consulted by capability
    /// token validation. Always-present (empty by default) so revocations
    /// from `auth.revoke_session` take effect immediately.
    pub revocation_registry: Arc<crate::auth::RevocationRegistry>,

    /// Phase K.1: optional light-mode header sync manager. When the
    /// daemon was started with `--mode light` this is `Some` and the
    /// startup path skips full block hosting and shard pipeline. K.1
    /// alpha ships the type/wiring; the production minimization
    /// (skipping `ShardStore`/`PipelineEngine`/Caesar/ngauge etc.)
    /// is staged as K.1.5.
    pub light_sync_manager: Option<Arc<crate::light_client::HeaderSyncManager>>,

    /// Phase M.4.5b: optional catalog registry provider for typedef
    /// dependency resolution.
    ///
    /// Alpha-default inert: when `None`, the `catalog.dependencies` IPC
    /// handler returns `{"status":"alpha","note":"catalog registry not
    /// wired"}` with empty arrays — never a fabricated graph. Operators
    /// opt in by wiring an adapter that bridges
    /// `catalog::registry::CatalogRegistry` into this trait object at
    /// daemon startup (wiring is M.4.5c — the catalog crate already
    /// depends on blockmatrix, so the adapter must live downstream of
    /// blockmatrix to avoid a dependency cycle).
    pub catalog_registry: Option<Arc<dyn crate::catalog::CatalogProvider>>,

    /// P3 (F5): shared inbox for received share invitations.
    ///
    /// This is the SAME `Arc` handed to the network `PeerContext`, so invites
    /// delivered over `TAG_SHARE_INVITE` land in the store that `share.inbox`
    /// reads and `share.accept` consumes.
    ///
    /// Alpha-default inert: when `None`, `share.inbox` returns an empty list
    /// and `share.accept` returns a "sharing inbox not configured" error.
    pub inbox_store: Option<Arc<crate::sharing::inbox::InboxStore>>,
}

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain synchronization manager for Device and Network scopes
//!
//! Coordinates synchronization between a node's local Device chain and
//! any Network scope chains it participates in. A node ALWAYS has a Device
//! chain (starts on boot). It can OPTIONALLY join one or more Network
//! scope chains by syncing with other participating nodes.
//!
//! PrivacyMode controls WHO can participate in a network.
//! BlockchainScope controls WHETHER chains synchronize.
//!
//! Wire-level message types and the snapshot-based block provider live in
//! the sibling `sync_protocol` module.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::bootstrap::PrivacyMode;
use hypermesh_lib::{BlockchainScope, NetworkId};

// Re-export extracted types so the public API is unchanged.
pub use super::sync_protocol::{NodeBlockchainBlockProvider, PropagationStrategyConfig, SyncMessage};

/// Provides block data to the SyncManager for responding to sync requests.
///
/// Implementations query the local blockchain (e.g. `NodeBlockchain`) and
/// return block hashes for the requested height range.  The trait is
/// intentionally synchronous -- callers are expected to pre-load data or
/// use interior async (e.g. `block_on`) when bridging from async contexts.
pub trait BlockProvider: Send + Sync {
    /// Return up to `max_blocks` block hashes starting at `from_height`.
    ///
    /// The second element of the tuple is the provider's current chain height.
    fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64);

    /// Return the FULL genesis block (index 0), when this provider has it.
    ///
    /// S3.0/B3: `handle_genesis_request` used to answer a `GenesisRequest` with
    /// a hash string stuffed into a field named `genesis_block_json`, because
    /// the provider exposed hashes only. A peer cannot adopt a hash. Providers
    /// that hold real blocks override this; hash-only providers keep the
    /// default `None` and the handler declines to answer rather than replying
    /// with something that merely looks like a genesis block.
    fn get_genesis_block(&self) -> Option<super::block::Block> {
        None
    }
}

/// Receives notifications when the SyncManager transitions a network to
/// `Synchronized` state.  This enables downstream components (e.g. the
/// `BlockPropagator`) to act on newly-synced blocks.
pub trait SyncObserver: Send {
    /// Called when `network_id` reaches `Synchronized` state at `block_height`.
    fn on_sync_complete(&self, network_id: &str, block_height: u64);
}

/// Manages blockchain synchronization between Device and Network scopes.
///
/// Each node has exactly one Device chain (always present, created at boot).
/// The SyncManager tracks zero or more Network memberships and coordinates
/// synchronization state for each.
pub struct SyncManager {
    /// Identifier for this node's device chain
    device_chain_id: String,
    /// Network chains this node participates in (keyed by canonical [`NetworkId`]).
    ///
    /// Callers still address networks by their wire string; every method maps
    /// that string to a `NetworkId` via [`NetworkId::from_wire_str`] before
    /// touching the map, and each membership retains the original wire label so
    /// sync messages echo byte-identical `network_id` strings.
    network_memberships: HashMap<NetworkId, NetworkMembership>,
    /// Sync state per network, keyed by canonical [`NetworkId`].
    sync_states: HashMap<NetworkId, SyncState>,
    /// Configuration
    config: SyncConfig,
    /// Optional observer notified on sync completion
    observer: Option<Box<dyn SyncObserver>>,
    /// S3.0/B3: verified genesis block per network, recorded on receipt of a
    /// `GenesisResponse`. Non-destructive: the device chain is untouched.
    /// Keyed by canonical [`NetworkId`].
    network_genesis: HashMap<NetworkId, super::block::Block>,
    /// Phase I.1: when `true`, [`Self::generate_sync_request`] emits
    /// [`SyncMessage::HeaderRequest`] instead of [`SyncMessage::Request`].
    ///
    /// Header-only sync is opt-in for low-bandwidth peers (the
    /// substrate for Phase K's light/thin-client tier). Headers carry
    /// `index`, `hash`, `prev_hash`, `entries_hash`, `entry_count`,
    /// and a derived timestamp via the state proof (when the full
    /// header is materialized server-side). The receiver verifies
    /// `prev_hash` chains correctly and only fetches full blocks via
    /// [`SyncMessage::BlockRequest`] when state-proof verification is
    /// required.
    ///
    /// Default `false` — preserves byte-compatible behaviour with
    /// pre-I.1 deployments.
    prefer_headers_mode: bool,
    /// Phase I.1: counter for `headers_only_sync_count` metric. Incremented
    /// every time the SyncManager produces a `HeaderRequest` (i.e.
    /// when `prefer_headers_mode` is on AND a sync is needed).
    /// Exposed via [`Self::headers_only_sync_count`].
    headers_only_sync_count: u64,
}

/// Represents membership in a Network scope chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMembership {
    /// Canonical identifier for this network.
    pub network_id: NetworkId,
    /// The wire/JSON label this network was joined under (the CLI free-form id
    /// or a domain's derived hex). Retained so `network_id` fields echoed onto
    /// the wire stay byte-identical — this is the network's JSON-wire-boundary
    /// representation, not a second identity type.
    pub network_label: String,
    /// Always `Network` for memberships (Device is implicit)
    pub scope: BlockchainScope,
    /// Privacy mode controlling participation rules
    pub privacy_mode: PrivacyMode,
    /// Timestamp (unix seconds) when this node joined the network
    pub joined_at: u64,
    /// Timestamp of the last successful sync, if any
    pub last_sync: Option<u64>,
    /// The outer network this membership nests inside, if any.
    ///
    /// `None` for a top-level / operator-declared network — it has no outer
    /// boundary. `Some(parent)` for a nested sub-network, resolved by walking
    /// the parent chain — the same nesting model DNS domains use
    /// (`dns/domain.rs`), one primitive at every scale.
    pub parent_network_id: Option<NetworkId>,
}

/// Tracks the synchronization state for a specific network
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncState {
    /// Not syncing -- Device scope only
    Disconnected,
    /// Searching for peers in the network
    Discovering,
    /// Actively syncing chain state with network peers
    Syncing {
        /// Progress as a fraction (0.0 to 1.0)
        progress: f64,
        /// Number of peers currently syncing with
        peer_count: usize,
    },
    /// Fully synchronized with the network
    Synchronized {
        /// Height of the last synced block
        last_block_height: u64,
    },
    /// Sync failed, can be retried
    Failed {
        /// Human-readable failure reason
        reason: String,
    },
}

/// Configuration for the sync manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Maximum simultaneous network memberships
    pub max_networks: usize,
    /// Milliseconds between sync checks
    pub sync_interval_ms: u64,
    /// Maximum blocks behind before forcing a full sync
    pub max_block_lag: u64,
    /// Propagation strategy for block announcements
    pub propagation_strategy: PropagationStrategyConfig,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_networks: 8,
            sync_interval_ms: 5_000,
            max_block_lag: 100,
            propagation_strategy: PropagationStrategyConfig::Broadcast,
        }
    }
}

impl SyncManager {
    /// Create a new sync manager for the given device chain
    pub fn new(device_chain_id: String, config: SyncConfig) -> Self {
        info!(
            device_chain = %device_chain_id,
            max_networks = config.max_networks,
            "SyncManager created"
        );

        Self {
            device_chain_id,
            network_memberships: HashMap::new(),
            sync_states: HashMap::new(),
            config,
            observer: None,
            network_genesis: HashMap::new(),
            prefer_headers_mode: false,
            headers_only_sync_count: 0,
        }
    }

    /// Phase I.1: enable or disable header-only sync mode.
    ///
    /// When enabled, [`Self::generate_sync_request`] returns
    /// [`SyncMessage::HeaderRequest`] instead of
    /// [`SyncMessage::Request`]. The dispatcher routes these to the
    /// header handler which returns [`SyncMessage::HeaderResponse`]
    /// carrying [`super::block::BlockHeader`] structs.
    pub fn set_prefer_headers_mode(&mut self, enabled: bool) {
        self.prefer_headers_mode = enabled;
        info!(
            prefer_headers = enabled,
            "Header-only sync mode toggled"
        );
    }

    /// Read the current header-only sync flag.
    pub fn prefer_headers_mode(&self) -> bool {
        self.prefer_headers_mode
    }

    /// Phase I.1: cumulative count of header-only sync requests
    /// generated by this SyncManager. Useful for observability /
    /// alerting on light-client wide adoption.
    pub fn headers_only_sync_count(&self) -> u64 {
        self.headers_only_sync_count
    }

    /// Register an observer that will be notified on sync completion.
    pub fn set_observer(&mut self, observer: Box<dyn SyncObserver>) {
        self.observer = Some(observer);
    }

    /// Get the device chain identifier
    pub fn device_chain_id(&self) -> &str {
        &self.device_chain_id
    }

    /// Get the sync configuration
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Join a Network scope chain as a **top-level network** — an
    /// operator-declared boundary with no outer network (`parent_network_id =
    /// None`).
    pub fn join_network(
        &mut self,
        network_id: String,
        privacy_mode: PrivacyMode,
        now_unix_secs: u64,
    ) -> Result<(), String> {
        self.join_network_nested(network_id, privacy_mode, None, now_unix_secs)
    }

    /// Join a Network scope chain, optionally nested under `parent_network_id`.
    /// A `Some(parent)` membership is a nested sub-network — resolvable by
    /// walking the parent chain. `None` is a top-level boundary.
    pub fn join_network_nested(
        &mut self,
        network_id: String,
        privacy_mode: PrivacyMode,
        parent_network_id: Option<NetworkId>,
        now_unix_secs: u64,
    ) -> Result<(), String> {
        let key = NetworkId::from_wire_str(&network_id);
        if self.network_memberships.contains_key(&key) {
            return Err(format!("Already a member of network {network_id}"));
        }

        if self.network_memberships.len() >= self.config.max_networks {
            return Err(format!(
                "Maximum network memberships ({}) reached",
                self.config.max_networks
            ));
        }

        let membership = NetworkMembership {
            network_id: key,
            network_label: network_id.clone(),
            scope: BlockchainScope::Network,
            privacy_mode,
            joined_at: now_unix_secs,
            last_sync: None,
            parent_network_id,
        };

        info!(
            network = %network_id,
            privacy = %privacy_mode,
            nested = parent_network_id.is_some(),
            "Joined network"
        );

        self.network_memberships.insert(key, membership);
        self.sync_states.insert(key, SyncState::Discovering);

        Ok(())
    }

    /// S3.0/B3: record a VERIFIED foreign genesis block for `network_id`.
    ///
    /// NON-DESTRUCTIVE by construction. This does not touch the device chain —
    /// it records "this is the genesis the network `network_id` is rooted at"
    /// alongside the membership, which is what a joiner needs in order to
    /// recognise that network's chain later. The destructive
    /// `NodeBlockchain::adopt_genesis` (which clears the local chain) is
    /// deliberately NOT called from here; a container that holds the device
    /// chain and adopted network chains side by side is S3.4's job.
    ///
    /// The caller is responsible for having verified the block first (index 0,
    /// `is_genesis`, `verify_hash`); this method re-checks those invariants
    /// because a recorded root that fails them is worse than none.
    pub fn record_network_genesis(
        &mut self,
        network_id: &str,
        genesis: super::block::Block,
    ) -> Result<(), String> {
        let key = NetworkId::from_wire_str(network_id);
        if !genesis.is_genesis() {
            return Err(format!(
                "Refusing genesis for {network_id}: index {} / previous_hash {} is not a genesis",
                genesis.index,
                &genesis.previous_hash[..16.min(genesis.previous_hash.len())],
            ));
        }
        if !genesis.verify_hash() {
            return Err(format!(
                "Refusing genesis for {network_id}: hash verification failed",
            ));
        }

        if let Some(existing) = self.network_genesis.get(&key) {
            if existing.hash != genesis.hash {
                return Err(format!(
                    "Refusing genesis for {network_id}: conflicts with the already-recorded \
                     root {} (a network has exactly one genesis)",
                    &existing.hash[..16.min(existing.hash.len())],
                ));
            }
            return Ok(());
        }

        info!(
            network = %network_id,
            genesis = %&genesis.hash[..16.min(genesis.hash.len())],
            "Recorded verified network genesis (non-destructive)",
        );
        self.network_genesis.insert(key, genesis);
        Ok(())
    }

    /// The verified genesis block recorded for `network_id`, if any.
    pub fn network_genesis(&self, network_id: &str) -> Option<&super::block::Block> {
        self.network_genesis.get(&NetworkId::from_wire_str(network_id))
    }

    /// Leave a Network scope chain
    pub fn leave_network(&mut self, network_id: &str) -> Result<(), String> {
        let key = NetworkId::from_wire_str(network_id);
        if self.network_memberships.remove(&key).is_none() {
            return Err(format!("Not a member of network {network_id}"));
        }

        self.sync_states.remove(&key);
        info!(network = %network_id, "Left network");

        Ok(())
    }

    /// Get the current sync state for a network
    pub fn sync_state(&self, network_id: &str) -> Option<&SyncState> {
        self.sync_states.get(&NetworkId::from_wire_str(network_id))
    }

    /// Get all active network memberships
    pub fn active_networks(&self) -> Vec<&NetworkMembership> {
        self.network_memberships.values().collect()
    }

    /// Get the count of active network memberships
    pub fn active_network_count(&self) -> usize {
        self.network_memberships.len()
    }

    /// Check if the node is a member of a specific network
    pub fn is_member(&self, network_id: &str) -> bool {
        self.network_memberships
            .contains_key(&NetworkId::from_wire_str(network_id))
    }

    /// Update the sync state for a network
    pub fn update_sync_state(&mut self, network_id: &str, state: SyncState) -> Result<(), String> {
        let key = NetworkId::from_wire_str(network_id);
        if !self.network_memberships.contains_key(&key) {
            return Err(format!("Not a member of network {network_id}"));
        }

        debug!(
            network = %network_id,
            state = ?state,
            "Sync state updated"
        );

        self.sync_states.insert(key, state);

        Ok(())
    }

    /// Record a successful sync timestamp for a network
    pub fn record_sync(&mut self, network_id: &str, now_unix_secs: u64) {
        if let Some(membership) = self
            .network_memberships
            .get_mut(&NetworkId::from_wire_str(network_id))
        {
            membership.last_sync = Some(now_unix_secs);
        }
    }

    /// Process an incoming sync message and return an optional response.
    ///
    /// When a `BlockProvider` is supplied, `SyncRequest` responses are
    /// populated with real block hashes from the local chain.  Without a
    /// provider the response contains an empty hash list (legacy behaviour).
    pub fn process_sync_message(&mut self, msg: SyncMessage) -> Option<SyncMessage> {
        self.process_sync_message_with_provider(msg, None)
    }

    /// Process a sync message using the given block provider for data lookup.
    pub fn process_sync_message_with_provider(
        &mut self,
        msg: SyncMessage,
        provider: Option<&dyn BlockProvider>,
    ) -> Option<SyncMessage> {
        match msg {
            SyncMessage::Request {
                network_id,
                from_height,
                max_blocks,
            } => self.handle_sync_request(&network_id, from_height, max_blocks, provider),

            SyncMessage::Announce {
                network_id,
                block_height,
                block_hash,
            } => self.handle_sync_announce(&network_id, block_height, &block_hash),

            SyncMessage::Response {
                network_id,
                block_hashes,
                peer_height,
            } => self.handle_sync_response(&network_id, block_hashes, peer_height),

            // Genesis/Header/Block request-response variants are handled
            // at the dispatch layer (SyncDispatcher), not SyncManager.
            SyncMessage::GenesisRequest { .. }
            | SyncMessage::GenesisResponse { .. }
            | SyncMessage::HeaderRequest { .. }
            | SyncMessage::HeaderResponse { .. }
            | SyncMessage::BlockRequest { .. }
            | SyncMessage::BlockResponse { .. } => {
                debug!("SyncManager: genesis/header/block messages handled by dispatcher");
                None
            }
        }
    }

    /// Handle a SyncRequest: return block hashes from the provider or empty.
    fn handle_sync_request(
        &self,
        network_id: &str,
        from_height: u64,
        max_blocks: u32,
        provider: Option<&dyn BlockProvider>,
    ) -> Option<SyncMessage> {
        if !self.is_member(network_id) {
            warn!(
                network = %network_id,
                "Received sync request for unknown network"
            );
            return None;
        }

        debug!(
            network = %network_id,
            from = from_height,
            max = max_blocks,
            "Processing sync request"
        );

        let (block_hashes, peer_height) = match provider {
            Some(bp) => bp.get_block_hashes(from_height, max_blocks),
            None => (Vec::new(), from_height),
        };

        Some(SyncMessage::Response {
            network_id: network_id.to_string(),
            block_hashes,
            peer_height,
        })
    }

    /// Handle a SyncAnnounce: trigger resync if we are too far behind.
    fn handle_sync_announce(
        &mut self,
        network_id: &str,
        block_height: u64,
        block_hash: &str,
    ) -> Option<SyncMessage> {
        if !self.is_member(network_id) {
            return None;
        }

        debug!(
            network = %network_id,
            height = block_height,
            hash = %block_hash,
            "Received block announcement"
        );

        let key = NetworkId::from_wire_str(network_id);
        if let Some(SyncState::Synchronized { last_block_height }) = self.sync_states.get(&key)
        {
            if block_height > last_block_height + self.config.max_block_lag {
                self.sync_states.insert(
                    key,
                    SyncState::Syncing {
                        progress: 0.0,
                        peer_count: 1,
                    },
                );
            }
        }

        None
    }

    /// Handle a SyncResponse: update sync progress and notify observer.
    fn handle_sync_response(
        &mut self,
        network_id: &str,
        block_hashes: Vec<String>,
        peer_height: u64,
    ) -> Option<SyncMessage> {
        if !self.is_member(network_id) {
            return None;
        }

        debug!(
            network = %network_id,
            blocks = block_hashes.len(),
            peer_height = peer_height,
            "Received sync response"
        );

        if block_hashes.is_empty() {
            self.sync_states.insert(
                NetworkId::from_wire_str(network_id),
                SyncState::Synchronized {
                    last_block_height: peer_height,
                },
            );
            self.notify_sync_complete(network_id, peer_height);
        }

        None
    }

    /// Notify the registered observer (if any) of a sync completion.
    fn notify_sync_complete(&self, network_id: &str, block_height: u64) {
        if let Some(ref obs) = self.observer {
            obs.on_sync_complete(network_id, block_height);
        }
    }

    /// Generate a sync request for a specific network
    ///
    /// Returns None if the node is not a member or already synchronized.
    ///
    /// Phase I.1: when `prefer_headers_mode` is `true`, returns
    /// [`SyncMessage::HeaderRequest`] (lightweight — peer responds
    /// with `BlockHeader` structs instead of full blocks). This is
    /// the activation path for `HeaderRequest`/`HeaderResponse` (the
    /// types existed since Phase I substrate but were never produced).
    pub fn generate_sync_request(
        &self,
        network_id: &str,
        local_height: u64,
    ) -> Option<SyncMessage> {
        if !self.is_member(network_id) {
            return None;
        }

        let state = self.sync_states.get(&NetworkId::from_wire_str(network_id))?;

        match state {
            SyncState::Discovering | SyncState::Syncing { .. } => {
                if self.prefer_headers_mode {
                    Some(SyncMessage::HeaderRequest {
                        network_id: network_id.to_string(),
                        from_height: local_height,
                        max_count: 50,
                    })
                } else {
                    Some(SyncMessage::Request {
                        network_id: network_id.to_string(),
                        from_height: local_height,
                        max_blocks: 50,
                    })
                }
            }
            _ => None,
        }
    }

    /// Phase I.1: variant of [`generate_sync_request`] that takes a
    /// `&mut self` so it can increment the
    /// [`headers_only_sync_count`](Self::headers_only_sync_count)
    /// metric. Functionally equivalent otherwise.
    pub fn generate_sync_request_metered(
        &mut self,
        network_id: &str,
        local_height: u64,
    ) -> Option<SyncMessage> {
        let req = self.generate_sync_request(network_id, local_height);
        if let Some(SyncMessage::HeaderRequest { .. }) = req {
            self.headers_only_sync_count = self.headers_only_sync_count.saturating_add(1);
        }
        req
    }

    /// Get networks that need syncing (Discovering or Syncing state).
    ///
    /// Returns each network's wire label (not its `NetworkId`) so callers echo
    /// byte-identical `network_id` strings onto the wire when they build sync
    /// requests for these networks.
    pub fn networks_needing_sync(&self) -> Vec<&str> {
        self.network_memberships
            .iter()
            .filter(|(key, _)| {
                matches!(
                    self.sync_states.get(*key),
                    Some(SyncState::Discovering) | Some(SyncState::Syncing { .. })
                )
            })
            .map(|(_, membership)| membership.network_label.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SyncConfig {
        SyncConfig {
            max_networks: 4,
            ..SyncConfig::default()
        }
    }

    #[test]
    fn test_create_sync_manager() {
        let mgr = SyncManager::new("device-chain-1".to_string(), default_config());

        assert_eq!(mgr.device_chain_id(), "device-chain-1");
        assert_eq!(mgr.active_network_count(), 0);
        assert!(mgr.active_networks().is_empty());
    }

    #[test]
    fn test_join_and_leave_network() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());

        // Join a network
        let result = mgr.join_network("net-alpha".to_string(), PrivacyMode::PUBLIC, 1000);
        assert!(result.is_ok());
        assert_eq!(mgr.active_network_count(), 1);
        assert!(mgr.is_member("net-alpha"));

        // Verify initial state is Discovering
        let state = mgr
            .sync_state("net-alpha")
            .expect("test: sync state should exist");
        assert_eq!(*state, SyncState::Discovering);

        // Leave the network
        let result = mgr.leave_network("net-alpha");
        assert!(result.is_ok());
        assert_eq!(mgr.active_network_count(), 0);
        assert!(!mgr.is_member("net-alpha"));
    }

    // ------------------------------------------------------------------
    // S3.0/B3 — record_network_genesis (QA follow-up: the refusal branches
    // were correct but untested).
    // ------------------------------------------------------------------

    fn genesis_block(x: i64) -> super::super::block::Block {
        let coord = crate::matrix::coordinate::MatrixCoordinate::new(x, 0, 0)
            .expect("test: valid coordinate");
        super::super::block::Block::genesis(coord)
    }

    #[test]
    fn test_record_network_genesis_accepts_then_is_idempotent() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());
        let genesis = genesis_block(1);

        mgr.record_network_genesis("net-1", genesis.clone())
            .expect("test: first verified root is recorded");
        assert_eq!(
            mgr.network_genesis("net-1").map(|b| b.hash.clone()),
            Some(genesis.hash.clone()),
        );

        // Re-recording the SAME root is a no-op, not an error (a peer may
        // announce the network's genesis more than once).
        mgr.record_network_genesis("net-1", genesis.clone())
            .expect("test: re-recording the identical root is idempotent");
        assert_eq!(
            mgr.network_genesis("net-1").map(|b| b.hash.clone()),
            Some(genesis.hash),
        );
    }

    #[test]
    fn test_record_network_genesis_refuses_conflicting_second_root() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());
        let first = genesis_block(1);
        let second = genesis_block(2);
        assert_ne!(first.hash, second.hash, "test setup: distinct roots");

        mgr.record_network_genesis("net-1", first.clone())
            .expect("test: first root recorded");

        // A network has exactly one genesis: a second, DIFFERENT root must be
        // refused — and must not overwrite the one already recorded.
        let err = mgr
            .record_network_genesis("net-1", second)
            .expect_err("test: conflicting second root must be refused");
        assert!(
            err.contains("conflicts with the already-recorded"),
            "error should cite the conflicting root, got: {err}",
        );
        assert_eq!(
            mgr.network_genesis("net-1").map(|b| b.hash.clone()),
            Some(first.hash),
            "the originally recorded root must be untouched",
        );
    }

    #[test]
    fn test_record_network_genesis_refuses_non_genesis_and_bad_hash() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());

        // (a) Not a genesis block (index != 0).
        let mut not_genesis = genesis_block(3);
        not_genesis.index = 7;
        let err = mgr
            .record_network_genesis("net-1", not_genesis)
            .expect_err("test: non-genesis must be refused");
        assert!(err.contains("is not a genesis"), "got: {err}");

        // (b) Genesis-shaped but the hash does not recompute.
        let mut tampered = genesis_block(4);
        tampered.hash = "0".repeat(64);
        let err = mgr
            .record_network_genesis("net-1", tampered)
            .expect_err("test: unverifiable hash must be refused");
        assert!(err.contains("hash verification failed"), "got: {err}");

        assert!(
            mgr.network_genesis("net-1").is_none(),
            "no refused block may be recorded",
        );
    }

    #[test]
    fn test_duplicate_join_rejected() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());

        mgr.join_network("net-1".to_string(), PrivacyMode::PRIVATE, 100)
            .expect("test: first join should succeed");

        let result = mgr.join_network("net-1".to_string(), PrivacyMode::PRIVATE, 200);
        assert!(result.is_err());
        assert!(result
            .expect_err("test: should have error")
            .contains("Already a member"));
    }

    #[test]
    fn test_max_networks_enforced() {
        let config = SyncConfig {
            max_networks: 2,
            ..SyncConfig::default()
        };
        let mut mgr = SyncManager::new("dev".to_string(), config);

        mgr.join_network("n1".to_string(), PrivacyMode::PUBLIC, 1)
            .expect("test: join 1");
        mgr.join_network("n2".to_string(), PrivacyMode::PUBLIC, 2)
            .expect("test: join 2");

        let result = mgr.join_network("n3".to_string(), PrivacyMode::PUBLIC, 3);
        assert!(result.is_err());
        assert!(result
            .expect_err("test: should have error")
            .contains("Maximum network memberships"));
    }

    #[test]
    fn test_leave_unknown_network_fails() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        let result = mgr.leave_network("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_state_transitions() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Transition: Discovering -> Syncing
        mgr.update_sync_state(
            "net",
            SyncState::Syncing {
                progress: 0.5,
                peer_count: 3,
            },
        )
        .expect("test: update to syncing");

        if let Some(SyncState::Syncing {
            progress,
            peer_count,
        }) = mgr.sync_state("net")
        {
            assert!((*progress - 0.5).abs() < f64::EPSILON);
            assert_eq!(*peer_count, 3);
        } else {
            unreachable!("test: expected Syncing state");
        }

        // Transition: Syncing -> Synchronized
        mgr.update_sync_state(
            "net",
            SyncState::Synchronized {
                last_block_height: 42,
            },
        )
        .expect("test: update to synchronized");

        assert_eq!(
            mgr.sync_state("net"),
            Some(&SyncState::Synchronized {
                last_block_height: 42
            })
        );
    }

    #[test]
    fn test_process_sync_announce_triggers_resync() {
        let config = SyncConfig {
            max_block_lag: 10,
            ..default_config()
        };
        let mut mgr = SyncManager::new("dev".to_string(), config);

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");
        mgr.update_sync_state(
            "net",
            SyncState::Synchronized {
                last_block_height: 50,
            },
        )
        .expect("test: set synchronized");

        // Announce a block far ahead -- should trigger resync
        let msg = SyncMessage::Announce {
            network_id: "net".to_string(),
            block_height: 200,
            block_hash: "abc123".to_string(),
        };

        let _response = mgr.process_sync_message(msg);

        // State should have transitioned to Syncing
        match mgr.sync_state("net") {
            Some(SyncState::Syncing { .. }) => {}
            other => unreachable!("test: expected Syncing, got {:?}", other),
        }
    }

    #[test]
    fn test_generate_sync_request() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Should generate request when Discovering
        let req = mgr.generate_sync_request("net", 10);
        assert!(req.is_some());

        if let Some(SyncMessage::Request {
            network_id,
            from_height,
            max_blocks,
        }) = req
        {
            assert_eq!(network_id, "net");
            assert_eq!(from_height, 10);
            assert_eq!(max_blocks, 50);
        } else {
            unreachable!("test: expected Request message");
        }

        // Should NOT generate request when Synchronized
        mgr.update_sync_state(
            "net",
            SyncState::Synchronized {
                last_block_height: 42,
            },
        )
        .expect("test: set synchronized");

        let req = mgr.generate_sync_request("net", 42);
        assert!(req.is_none());
    }

    #[test]
    fn test_networks_needing_sync() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("n1".to_string(), PrivacyMode::PUBLIC, 1)
            .expect("test: join n1");
        mgr.join_network("n2".to_string(), PrivacyMode::PRIVATE, 2)
            .expect("test: join n2");

        // Both start as Discovering
        let needing = mgr.networks_needing_sync();
        assert_eq!(needing.len(), 2);

        // Synchronize n1
        mgr.update_sync_state(
            "n1",
            SyncState::Synchronized {
                last_block_height: 100,
            },
        )
        .expect("test: synchronize n1");

        let needing = mgr.networks_needing_sync();
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0], "n2");
    }

    #[test]
    fn test_record_sync_timestamp() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Initially no last_sync
        let membership = &mgr.active_networks()[0];
        assert!(membership.last_sync.is_none());

        // Record sync
        mgr.record_sync("net", 200);

        let membership = &mgr.active_networks()[0];
        assert_eq!(membership.last_sync, Some(200));
    }

    #[test]
    fn test_membership_scope_is_network() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::ANONYMOUS, 50)
            .expect("test: join");

        let membership = &mgr.active_networks()[0];
        assert_eq!(membership.scope, BlockchainScope::Network);
        assert_eq!(membership.privacy_mode, PrivacyMode::ANONYMOUS);
        assert_eq!(membership.joined_at, 50);
    }
}

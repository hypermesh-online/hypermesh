// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core chain data structures and read-only operations.
//!
//! `NodeBlockchain` holds the in-memory chain state (blocks, indices,
//! statistics) and exposes query methods.  Mutation methods live in
//! [`super::mutations`]; genesis-auth helpers in [`super::genesis_ops`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::block::{Block, BlockHeader};
use super::genesis_auth::GenesisAuthManager;
use super::validation::ChainValidator;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::proof_of_state::validation_service::ValidationService;
use crate::proof_of_state::StateRequirements;

/// Statistics about a node's blockchain.
#[derive(Debug, Clone, Default)]
pub struct ChainStats {
    /// Total number of blocks
    pub total_blocks: u64,
    /// Current chain height (latest block index)
    pub chain_height: u64,
    /// Total data size in bytes
    pub total_data_size: usize,
}

/// A node's independent blockchain.
///
/// Each BlockMatrix node maintains its own sovereign blockchain.
/// This is NOT a shared chain -- each node has complete autonomy.
pub struct NodeBlockchain {
    /// The node's position in the matrix
    pub(crate) node_coordinate: MatrixCoordinate,

    /// The chain of blocks (index -> block)
    pub(crate) blocks: Arc<RwLock<HashMap<u64, Block>>>,

    /// Quick lookup: hash -> block index
    pub(crate) hash_index: Arc<RwLock<HashMap<String, u64>>>,

    /// Current chain head (latest block)
    pub(crate) head: Arc<RwLock<Option<Block>>>,

    /// Chain validator (structural: hash linkage, size)
    pub(crate) validator: ChainValidator,

    /// State proof validation service (four-proof: WHO/WHEN/WHERE/WHAT)
    pub(crate) state_proof_validator: Arc<ValidationService>,

    /// Chain statistics
    pub(crate) stats: Arc<RwLock<ChainStats>>,

    /// Genesis authentication manager (optional MFA)
    pub(crate) genesis_auth: Arc<RwLock<Option<GenesisAuthManager>>>,

    /// Block headers for lightweight chain verification.
    /// Stores headers for blocks we don't have full data for.
    pub(crate) headers: Arc<RwLock<HashMap<u64, BlockHeader>>>,

    /// Orphan buffer: received blocks whose predecessor is not (yet) known.
    ///
    /// Zero-trust (P1/F7): a received block is NEVER spliced into the chain
    /// on a missing or non-matching predecessor. Instead it waits here, keyed
    /// by its own `previous_hash`, until a verified predecessor with that hash
    /// is inserted — at which point the orphan is drained and linked.
    ///
    /// P6 (task #22.2): the buffer is BOUNDED. Each entry carries its arrival
    /// [`Instant`]; on insert we first evict entries older than
    /// [`ORPHAN_TTL`], then — if still at [`MAX_ORPHANS`] — evict the oldest
    /// remaining entry. Without this an authenticated peer could flood
    /// distinct-`previous_hash` orphans (each a fresh key) and grow this map
    /// without limit. Eviction only ever drops UNLINKED orphans; the
    /// buffer-then-link drain and its `content_binding_ok` re-verification are
    /// untouched (P1 invariant preserved).
    pub(crate) orphans: Arc<RwLock<HashMap<String, (Block, Instant)>>>,

    /// H3: node signer used to attach a FALCON-1024 `signed_proof` envelope to
    /// every entry of a LOCALLY-produced block (`add_block`).
    ///
    /// Threaded in by the live daemon (via [`with_signer`](Self::with_signer))
    /// so produced blocks carry a verifiable, identity-bound envelope to peers.
    /// `None` for dev/test/library chains that never publish to the network —
    /// they produce unsigned entries, which is fine because such chains are not
    /// the untrusted-remote accept path. The remote path
    /// (`insert_received_block`) verifies whatever envelope the producer
    /// attached; it does not use this signer.
    pub(crate) signer: Option<Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>>,

    /// S3.0/B1: durable write-through sink for accepted blocks.
    ///
    /// When present (live daemon), EVERY block that reaches
    /// [`insert_block`](Self::insert_block) — from `add_block`,
    /// `insert_received_block`, or an orphan drain — is written to the WAL and
    /// block storage BEFORE it becomes visible in memory. Fail-closed: a
    /// persistence error aborts the insert, so the in-memory chain can never
    /// hold a block that a restart would lose.
    ///
    /// `None` for library/test chains: behaviour is exactly as it was before
    /// S3.0 (memory only).
    pub(crate) block_sink: Option<Arc<dyn super::block_sink::BlockSink>>,

    /// S3.0 QA follow-up (FIX 2): head reservation for local appends.
    ///
    /// [`add_block`](super::mutations) reads the head to derive the next index,
    /// then builds, signs and inserts the block. Without a reservation, every
    /// concurrent caller reads the SAME head, computes the SAME index, and all
    /// but one loses at the duplicate-index check inside
    /// [`insert_block`](Self::insert_block) — its already-built, already-signed
    /// block is thrown away. S3.0's durable write-through widened that window
    /// by exactly the fsync duration (measured: 2 of 8 concurrent writers
    /// survived).
    ///
    /// Holding this mutex across read-head → build → sign → insert serialises
    /// local appends, which is what an append to a single linear chain is
    /// anyway: index N+1 cannot be computed until N exists.
    ///
    /// LOCK ORDER: this is acquired BEFORE any of `blocks`/`headers`/
    /// `hash_index`/`head`/`stats` and is never taken while one of them is
    /// held; `insert_block` acquires those itself, so the reservation holder
    /// must not (and does not) hold them across the call.
    pub(crate) append_lock: Arc<tokio::sync::Mutex<()>>,
}

/// Maximum number of buffered orphan blocks (P6 task #22.2). Beyond this the
/// oldest orphan is evicted on each new insert — bounds a distinct-prev-hash
/// flood from an authenticated peer.
pub(crate) const MAX_ORPHANS: usize = 1024;

/// Time-to-live for a buffered orphan (P6 task #22.2). An orphan whose
/// predecessor never arrives is reclaimed after this window instead of
/// lingering forever.
pub(crate) const ORPHAN_TTL: std::time::Duration = std::time::Duration::from_secs(120);

impl NodeBlockchain {
    /// Create a new blockchain for a node.
    pub fn new(node_coordinate: MatrixCoordinate) -> Self {
        Self::from_genesis(node_coordinate, Block::genesis(node_coordinate))
    }

    /// Create a new blockchain seeded with an externally-constructed genesis.
    ///
    /// The genesis PATH is deterministic since S3.0/B2 — it is a pure function
    /// of (device assessment, coordinate, genesis epoch). But
    /// [`Block::genesis`] reads a fresh `GenesisEpoch::now()` on every call, so
    /// two calls a nanosecond apart still yield different blocks. When a caller
    /// persists the genesis to disk AND holds a blockchain in memory, both must
    /// reference the *same* block — otherwise block 1's `previous_hash` (taken
    /// from the in-memory head) won't match the on-disk genesis on restart.
    /// Use [`Block::genesis_with_identity_at`] / [`Block::genesis_from_assessment`]
    /// when the epoch must be pinned (network genesis, reproducibility tests).
    pub fn from_genesis(node_coordinate: MatrixCoordinate, genesis: Block) -> Self {
        Self::from_genesis_with_service(node_coordinate, genesis, ValidationService::new())
    }

    /// Shared body for [`from_genesis`](Self::from_genesis): seed the chain
    /// with an externally-built genesis and the supplied state-proof validator.
    fn from_genesis_with_service(
        node_coordinate: MatrixCoordinate,
        genesis: Block,
        state_proof_validator: ValidationService,
    ) -> Self {
        assert!(
            genesis.is_genesis(),
            "from_genesis requires a genesis block (index 0, zero previous_hash)"
        );

        let mut blocks = HashMap::new();
        let mut hash_index = HashMap::new();

        hash_index.insert(genesis.hash.clone(), genesis.index);
        blocks.insert(genesis.index, genesis.clone());

        let stats = ChainStats {
            total_blocks: 1,
            chain_height: 0,
            total_data_size: genesis.size(),
        };

        info!(
            "Created new blockchain for node at ({}, {}, {})",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        NodeBlockchain {
            node_coordinate,
            blocks: Arc::new(RwLock::new(blocks)),
            hash_index: Arc::new(RwLock::new(hash_index)),
            head: Arc::new(RwLock::new(Some(genesis))),
            validator: ChainValidator::new(),
            state_proof_validator: Arc::new(state_proof_validator),
            stats: Arc::new(RwLock::new(stats)),
            genesis_auth: Arc::new(RwLock::new(None)),
            headers: Arc::new(RwLock::new(HashMap::new())),
            orphans: Arc::new(RwLock::new(HashMap::new())),
            signer: None,
            block_sink: None,
            append_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// H3: attach a node signer so locally-produced blocks (`add_block`) carry
    /// a FALCON-1024 `signed_proof` envelope bound to this node's identity.
    ///
    /// Builder form — the live daemon calls this right after constructing the
    /// chain (`from_genesis`/`from_blocks`) with its `FalconIdentity`. Chains
    /// without a signer produce unsigned entries.
    pub fn with_signer(
        mut self,
        signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>,
    ) -> Self {
        self.signer = Some(signer);
        self
    }

    /// S3.0/B1: attach a durable [`BlockSink`](super::block_sink::BlockSink) so
    /// every accepted block is written through to the WAL + block storage
    /// before it becomes visible in memory.
    ///
    /// Builder form, mirroring [`with_signer`](Self::with_signer) — the live
    /// daemon calls this right after constructing the chain
    /// (`from_genesis`/`from_blocks`) with its `PersistenceManager`. Chains
    /// without a sink keep the pre-S3.0 memory-only behaviour, which is
    /// correct for library and test chains that have no storage directory.
    pub fn with_persistence(
        mut self,
        sink: Arc<dyn super::block_sink::BlockSink>,
    ) -> Self {
        self.block_sink = Some(sink);
        self
    }

    /// Create a new blockchain with custom state proof requirements.
    ///
    /// Use `StateRequirements::production()` for production deployments,
    /// `StateRequirements::default()` for development/testing.
    pub fn with_requirements(
        node_coordinate: MatrixCoordinate,
        requirements: StateRequirements,
    ) -> Self {
        let mut chain = Self::new(node_coordinate);
        chain.state_proof_validator = Arc::new(ValidationService::with_requirements(requirements));
        chain
    }

    /// Reconstruct a blockchain from persisted blocks.
    ///
    /// Used on restart to rebuild the in-memory chain from blocks loaded
    /// from disk via `BlockchainStorage`. Validates chain integrity
    /// (previous_hash linkage) and rebuilds all indices.
    pub fn from_blocks(
        node_coordinate: MatrixCoordinate,
        blocks: Vec<Block>,
    ) -> Result<Self, String> {
        Self::from_blocks_with_service(node_coordinate, blocks, ValidationService::new())
    }

    /// Shared body for [`from_blocks`](Self::from_blocks): rebuild the chain
    /// from persisted blocks with the supplied state-proof validator.
    fn from_blocks_with_service(
        node_coordinate: MatrixCoordinate,
        mut blocks: Vec<Block>,
        state_proof_validator: ValidationService,
    ) -> Result<Self, String> {
        if blocks.is_empty() {
            return Err("Cannot create blockchain from empty block list".to_string());
        }

        blocks.sort_by_key(|b| b.index);

        if blocks[0].index != 0 {
            return Err(format!(
                "First block must be genesis (index 0), got index {}",
                blocks[0].index,
            ));
        }

        // Validate chain integrity: each block's previous_hash must match
        // the prior block's hash (skip genesis which has no predecessor)
        for i in 1..blocks.len() {
            if blocks[i].previous_hash != blocks[i - 1].hash {
                return Err(format!(
                    "Chain integrity violation at block {}: previous_hash {} != block {}'s hash {}",
                    blocks[i].index,
                    blocks[i].previous_hash,
                    blocks[i - 1].index,
                    blocks[i - 1].hash,
                ));
            }
        }

        // Rebuild indices
        let mut block_map = HashMap::with_capacity(blocks.len());
        let mut hash_index = HashMap::with_capacity(blocks.len());
        let mut total_data_size = 0usize;

        for block in &blocks {
            block_map.insert(block.index, block.clone());
            hash_index.insert(block.hash.clone(), block.index);
            total_data_size += block.size();
        }

        let head = blocks.last().cloned();
        let chain_height = head.as_ref().map(|b| b.index).unwrap_or(0);

        let stats = ChainStats {
            total_blocks: blocks.len() as u64,
            chain_height,
            total_data_size,
        };

        info!(
            "Restored blockchain for node ({},{},{}) -- {} blocks, height {}",
            node_coordinate.x,
            node_coordinate.y,
            node_coordinate.z,
            blocks.len(),
            chain_height,
        );

        Ok(NodeBlockchain {
            node_coordinate,
            blocks: Arc::new(RwLock::new(block_map)),
            hash_index: Arc::new(RwLock::new(hash_index)),
            head: Arc::new(RwLock::new(head)),
            validator: ChainValidator::new(),
            state_proof_validator: Arc::new(state_proof_validator),
            stats: Arc::new(RwLock::new(stats)),
            genesis_auth: Arc::new(RwLock::new(None)),
            headers: Arc::new(RwLock::new(HashMap::new())),
            orphans: Arc::new(RwLock::new(HashMap::new())),
            signer: None,
            block_sink: None,
            append_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    /// Get the node's matrix coordinate.
    pub fn node_coordinate(&self) -> &MatrixCoordinate {
        &self.node_coordinate
    }

    /// Number of blocks currently buffered as orphans (P6 task #22.2).
    ///
    /// Used to assert the orphan buffer stays bounded under a
    /// distinct-`previous_hash` flood.
    pub async fn orphan_count(&self) -> usize {
        self.orphans.read().await.len()
    }

    /// Get a block by index.
    pub async fn get_block(&self, index: u64) -> Option<Block> {
        self.blocks.read().await.get(&index).cloned()
    }

    /// Get a block by hash.
    pub async fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        let hash_index = self.hash_index.read().await;
        if let Some(&index) = hash_index.get(hash) {
            drop(hash_index);
            self.get_block(index).await
        } else {
            None
        }
    }

    /// Get the current chain head (latest block).
    pub async fn get_head(&self) -> Option<Block> {
        self.head.read().await.clone()
    }

    /// Get the chain height (index of latest block).
    pub async fn get_height(&self) -> u64 {
        self.head
            .read()
            .await
            .as_ref()
            .map(|b| b.index)
            .unwrap_or(0)
    }

    /// Get all blocks in order.
    pub async fn get_chain(&self) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        let mut chain: Vec<Block> = blocks.values().cloned().collect();
        chain.sort_by_key(|b| b.index);
        chain
    }

    /// Validate the entire chain.
    pub async fn validate_chain(&self) -> bool {
        let chain = self.get_chain().await;

        if chain.is_empty() {
            warn!("Empty chain");
            return false;
        }

        if !chain[0].is_genesis() {
            error!("First block is not genesis");
            return false;
        }

        for i in 1..chain.len() {
            if !self
                .validator
                .validate_block(&chain[i], Some(&chain[i - 1]))
            {
                error!("Block {} failed validation", chain[i].index);
                return false;
            }
        }

        debug!("Chain validation successful for {} blocks", chain.len());
        true
    }

    /// Get chain statistics.
    pub async fn get_stats(&self) -> ChainStats {
        self.stats.read().await.clone()
    }

    /// Check if a block exists in this chain.
    pub async fn has_block(&self, hash: &str) -> bool {
        self.hash_index.read().await.contains_key(hash)
    }

    /// Per-asset shard authorization anchor (F6).
    ///
    /// Returns `true` if the given `shard_id` is a shard of an asset
    /// registered on THIS chain — i.e., some block entry's
    /// `StoragePointer::Sharded` lists `shard_id` among its
    /// `shard_hashes`. Because every entry carries a validated, content-
    /// bound `StateProof`, a positive answer means the shard belongs to an
    /// asset whose registration this node has verified. Serving a shard is
    /// then bound to the asset's on-chain proof, not merely to coarse
    /// network membership.
    ///
    /// This is the authorization anchor; the requester's PoS authentication
    /// is enforced separately (see `peer_auth::verify_shard_access`).
    pub async fn authorizes_shard(&self, shard_id: &[u8; 32]) -> bool {
        use super::block::StoragePointer;
        let blocks = self.blocks.read().await;
        for block in blocks.values() {
            for entry in &block.entries {
                if let StoragePointer::Sharded { shard_hashes, .. } = &entry.storage_pointer {
                    if shard_hashes.iter().any(|h| h == shard_id) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Return the content-bound registration entry that authorizes `shard_id`.
    ///
    /// A6.6: when this node SERVES a shard, it also serves that ONE asset's
    /// registration so the fetcher can re-anchor it on its own chain (torrent
    /// model: nodes that touched the vector share-compute the registration —
    /// no whole-chain replication). This scans the same `Sharded` entries as
    /// [`authorizes_shard`] and returns a CLONE of the first entry whose
    /// `shard_hashes` lists `shard_id`. The clone carries `asset_hash`, the
    /// content-bound `state_proof`, and the `StoragePointer::Sharded`
    /// (shard_hashes + placements) — everything the fetcher needs to
    /// independently re-validate and register the same asset.
    ///
    /// Returns `None` when no on-chain asset lists this shard (the serve path
    /// then omits the registration and falls back to bare shard bytes).
    pub async fn registration_for_shard(
        &self,
        shard_id: &[u8; 32],
    ) -> Option<super::block::BlockAssetEntry> {
        use super::block::StoragePointer;
        let blocks = self.blocks.read().await;
        for block in blocks.values() {
            for entry in &block.entries {
                if let StoragePointer::Sharded { shard_hashes, .. } = &entry.storage_pointer {
                    if shard_hashes.iter().any(|h| h == shard_id) {
                        return Some(entry.clone());
                    }
                }
            }
        }
        None
    }

    /// Get the last N blocks.
    pub async fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let chain = self.get_chain().await;
        let start = chain.len().saturating_sub(count);
        chain[start..].to_vec()
    }

    /// Calculate chain's total size in bytes.
    pub async fn get_total_size(&self) -> usize {
        self.blocks.read().await.values().map(|b| b.size()).sum()
    }

    /// Adopt a foreign network's genesis block, replacing the local chain.
    /// This is used when joining a network -- the node discards its independent
    /// chain and starts from the network's genesis.
    ///
    /// Returns error if the genesis block's index is not 0 or hash verification fails.
    ///
    /// # NOT WIRED — it would desynchronise memory from disk (S3.0 QA note)
    ///
    /// This clears `blocks`, `hash_index` and `head` and reseeds them WITHOUT
    /// notifying [`block_sink`](Self::block_sink). On a chain with a durable
    /// sink attached, the on-disk chain would keep the old genesis and every
    /// old block while memory holds the new root — the exact
    /// memory-ahead-of-disk divergence the S3.0 write-through exists to
    /// prevent, only inverted. Nothing in production calls this today
    /// (`SyncManager::record_network_genesis` deliberately records a network's
    /// root NON-destructively instead). Adopting a network chain for real needs
    /// a container that holds the device chain and adopted network chains side
    /// by side, with its own durable roots — that is S3.4's job. Do not wire
    /// this into a sink-bearing chain before then.
    pub async fn adopt_genesis(&self, genesis: Block) -> Result<(), String> {
        if genesis.index != 0 {
            return Err(format!(
                "Genesis block must have index 0, got {}",
                genesis.index,
            ));
        }
        if !genesis.verify_hash() {
            return Err("Genesis block hash verification failed".to_string());
        }

        let mut blocks = self.blocks.write().await;
        let mut hash_index = self.hash_index.write().await;
        let mut head = self.head.write().await;
        let mut stats = self.stats.write().await;

        blocks.clear();
        hash_index.clear();

        blocks.insert(genesis.index, genesis.clone());
        hash_index.insert(genesis.hash.clone(), genesis.index);
        *head = Some(genesis.clone());

        *stats = ChainStats {
            total_blocks: 1,
            chain_height: 0,
            total_data_size: genesis.size(),
        };

        info!(
            "Adopted foreign genesis block (hash: {}...)",
            &genesis.hash[..8.min(genesis.hash.len())],
        );

        Ok(())
    }

    /// Insert block headers for lightweight chain verification.
    /// Headers are stored separately from full blocks -- they prove chain
    /// integrity without requiring full block data.
    ///
    /// Returns the number of new headers inserted.
    pub async fn insert_received_headers(
        &self,
        headers: Vec<BlockHeader>,
    ) -> Result<usize, String> {
        let mut stored_headers = self.headers.write().await;
        let mut count = 0;
        for header in headers {
            if !stored_headers.contains_key(&header.index) {
                stored_headers.insert(header.index, header);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Get a block header by index. Returns from full blocks first, then stored headers.
    pub async fn get_header(&self, index: u64) -> Option<BlockHeader> {
        // Check full blocks first
        let blocks = self.blocks.read().await;
        if let Some(block) = blocks.get(&index) {
            return Some(block.header());
        }
        drop(blocks);
        // Then check stored headers
        let headers = self.headers.read().await;
        headers.get(&index).cloned()
    }

    /// Get the highest known height (full blocks or headers).
    pub async fn get_known_height(&self) -> u64 {
        let chain_height = self.get_height().await;
        let header_height = {
            let headers = self.headers.read().await;
            headers.keys().max().copied().unwrap_or(0)
        };
        chain_height.max(header_height)
    }

    /// Check if a full block (not just a header) exists at the given index.
    pub async fn has_full_block(&self, index: u64) -> bool {
        self.blocks.read().await.contains_key(&index)
    }

    /// Return the contiguous range of block indices for which full blocks exist.
    ///
    /// Returns `(start, end)` inclusive. If there are gaps, this returns the
    /// smallest and largest indices that have full blocks (the node's
    /// participation span). Returns `(0, 0)` if only the genesis block exists.
    pub async fn get_participation_range(&self) -> (u64, u64) {
        let blocks = self.blocks.read().await;
        if blocks.is_empty() {
            return (0, 0);
        }
        let min = blocks.keys().min().copied().unwrap_or(0);
        let max = blocks.keys().max().copied().unwrap_or(0);
        (min, max)
    }

    /// Convert full blocks within a range to headers-only.
    ///
    /// For each block in the range that exists as a full block, its header
    /// is extracted and stored in the `headers` map, then the full block is
    /// removed from `blocks`. This allows nodes to stop participating in a
    /// segment while retaining chain integrity verification capability.
    ///
    /// The genesis block (index 0) is never pruned.
    pub async fn prune_to_headers(&self, range: std::ops::Range<u64>) {
        let mut blocks = self.blocks.write().await;
        let mut headers = self.headers.write().await;
        let mut hash_index = self.hash_index.write().await;
        let mut stats = self.stats.write().await;

        for index in range {
            // Never prune genesis
            if index == 0 {
                continue;
            }
            if let Some(block) = blocks.remove(&index) {
                // Store header for integrity verification
                headers.insert(index, block.header());
                hash_index.remove(&block.hash);
                stats.total_blocks = stats.total_blocks.saturating_sub(1);
                stats.total_data_size = stats.total_data_size.saturating_sub(block.size());
            }
        }
    }

    /// Insert a block into the chain (internal helper).
    ///
    /// S3.0/B1: this is the single insert chokepoint for `add_block`,
    /// `insert_received_block` and the orphan drain, so it is also where the
    /// durable write-through happens. The block is persisted BEFORE it is
    /// published to the in-memory maps, and a persistence failure aborts the
    /// insert (fail-closed) — memory never gets ahead of disk.
    pub(crate) async fn insert_block(&self, block: Block) -> Result<(), String> {
        let mut blocks = self.blocks.write().await;
        let mut hash_index = self.hash_index.write().await;
        let mut head = self.head.write().await;
        let mut stats = self.stats.write().await;

        if blocks.contains_key(&block.index) {
            return Err(format!("Block {} already exists", block.index));
        }

        // Durable write-through (fail-closed). The write locks are held across
        // this await deliberately: no reader may observe a block that is not
        // yet on disk, and no concurrent insert may claim the same index while
        // this one is being written.
        if let Some(sink) = self.block_sink.as_ref() {
            sink.persist_block(&block).await.map_err(|e| {
                error!(
                    "Refusing block {} — durable persistence failed: {e}",
                    block.index,
                );
                format!("Block {} persistence failed: {e}", block.index)
            })?;
        }

        blocks.insert(block.index, block.clone());
        hash_index.insert(block.hash.clone(), block.index);

        if head.as_ref().is_none_or(|h| block.index > h.index) {
            *head = Some(block.clone());
            stats.chain_height = block.index;
        }

        stats.total_blocks += 1;
        stats.total_data_size += block.size();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block::{BlockAssetEntry, StoragePointer};
    use crate::assets::core::AssetRegistration;
    use trustchain::proof_of_state::StateProof;

    fn test_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            signed_proof: None,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        }
    }

    #[tokio::test]
    async fn test_blockchain_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        assert_eq!(chain.node_coordinate(), &coord);
        assert_eq!(chain.get_height().await, 0);

        let head = chain.get_head().await.expect("test: block retrieval");
        assert!(head.is_genesis());
    }

    #[tokio::test]
    async fn test_block_retrieval() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Insert a block manually via insert_block
        let entry = test_entry(coord);
        let genesis = chain.get_head().await.expect("test: genesis");
        let block = Block::new(1, vec![entry], genesis.hash.clone());

        chain.insert_block(block.clone()).await.expect("test: insert");

        let retrieved = chain.get_block(1).await.expect("test: block retrieval");
        assert_eq!(retrieved, block);

        let retrieved = chain
            .get_block_by_hash(&block.hash)
            .await
            .expect("test: block retrieval");
        assert_eq!(retrieved, block);

        assert!(chain.has_block(&block.hash).await);
        assert!(!chain.has_block("nonexistent").await);
    }

    #[tokio::test]
    async fn test_chain_statistics() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Insert 5 blocks manually
        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let stats = chain.get_stats().await;
        assert_eq!(stats.total_blocks, 6); // Including genesis
        assert_eq!(stats.chain_height, 5);
        assert!(stats.total_data_size > 0);
    }

    #[tokio::test]
    async fn test_get_chain() {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=3u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let full_chain = chain.get_chain().await;
        assert_eq!(full_chain.len(), 4); // 3 + genesis

        for (i, block) in full_chain.iter().enumerate() {
            assert_eq!(block.index, i as u64);
        }
    }

    #[tokio::test]
    async fn test_recent_blocks() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=10u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let recent = chain.get_recent_blocks(5).await;
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].index, 6);
        assert_eq!(recent[4].index, 10);
    }

    #[tokio::test]
    async fn test_chain_validation() {
        let coord = MatrixCoordinate::new(6, 6, 6).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_total_size() {
        let coord = MatrixCoordinate::new(7, 7, 7).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=2u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let total_size = chain.get_total_size().await;
        assert!(total_size > 0);
    }

    #[tokio::test]
    async fn test_from_blocks() {
        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");

        // Build a chain manually
        let genesis = Block::genesis(coord);
        let entry1 = test_entry(coord);
        let block1 = Block::new(1, vec![entry1], genesis.hash.clone());
        let entry2 = test_entry(coord);
        let block2 = Block::new(2, vec![entry2], block1.hash.clone());

        let blocks = vec![genesis, block1, block2];
        let chain =
            NodeBlockchain::from_blocks(coord, blocks).expect("test: from_blocks");

        assert_eq!(chain.get_height().await, 2);
        assert_eq!(chain.get_stats().await.total_blocks, 3);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_from_blocks_empty_fails() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let result = NodeBlockchain::from_blocks(coord, vec![]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_adopt_genesis_replaces_chain() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Original chain has its own genesis
        let original_height = chain.get_height().await;
        assert_eq!(original_height, 0);

        // Create a foreign genesis from a different coordinate
        let foreign_coord = MatrixCoordinate::new(9, 9, 9).expect("test: valid coordinate");
        let foreign_genesis = Block::genesis(foreign_coord);

        // Adopt the foreign genesis
        chain
            .adopt_genesis(foreign_genesis.clone())
            .await
            .expect("test: adopt genesis");

        // Chain should now start from the foreign genesis
        assert_eq!(chain.get_height().await, 0);
        let head = chain.get_head().await.expect("test: head exists");
        assert_eq!(head.hash, foreign_genesis.hash);
        assert!(chain.has_block(&foreign_genesis.hash).await);

        // Stats should reflect single block
        let stats = chain.get_stats().await;
        assert_eq!(stats.total_blocks, 1);
    }

    #[tokio::test]
    async fn test_adopt_genesis_rejects_non_zero_index() {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Create a block with index 1 (not genesis)
        let entry = test_entry(coord);
        let non_genesis = Block::new(1, vec![entry], "prev".to_string());

        let result = chain.adopt_genesis(non_genesis).await;
        assert!(result.is_err());
        assert!(
            result
                .expect_err("test: should error")
                .contains("index 0"),
        );
    }

    #[tokio::test]
    async fn test_adopt_genesis_rejects_invalid_hash() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Create a genesis then tamper with its hash
        let foreign_coord = MatrixCoordinate::new(8, 8, 8).expect("test: valid coordinate");
        let mut tampered = Block::genesis(foreign_coord);
        tampered.hash = "tampered_hash_value".to_string();

        let result = chain.adopt_genesis(tampered).await;
        assert!(result.is_err());
        assert!(
            result
                .expect_err("test: should error")
                .contains("hash verification"),
        );
    }

    #[tokio::test]
    async fn test_insert_received_headers() {
        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Create some headers
        let headers = vec![
            BlockHeader {
                index: 5,
                hash: "hash_5".to_string(),
                previous_hash: "hash_4".to_string(),
                entries_hash: [0u8; 32],
                entry_count: 1,
            },
            BlockHeader {
                index: 6,
                hash: "hash_6".to_string(),
                previous_hash: "hash_5".to_string(),
                entries_hash: [0u8; 32],
                entry_count: 2,
            },
        ];

        let count = chain
            .insert_received_headers(headers)
            .await
            .expect("test: insert headers");
        assert_eq!(count, 2);

        // Inserting the same headers again should not increase count
        let headers_dup = vec![BlockHeader {
            index: 5,
            hash: "hash_5".to_string(),
            previous_hash: "hash_4".to_string(),
            entries_hash: [0u8; 32],
            entry_count: 1,
        }];
        let count2 = chain
            .insert_received_headers(headers_dup)
            .await
            .expect("test: insert dup headers");
        assert_eq!(count2, 0);
    }

    #[tokio::test]
    async fn test_get_header_from_blocks_and_headers() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Genesis block (index 0) should be retrievable as header
        let genesis_header = chain.get_header(0).await;
        assert!(genesis_header.is_some());
        assert_eq!(genesis_header.expect("test: header").index, 0);

        // Insert a stored header for index 10
        let remote_header = BlockHeader {
            index: 10,
            hash: "remote_hash_10".to_string(),
            previous_hash: "remote_hash_9".to_string(),
            entries_hash: [0u8; 32],
            entry_count: 3,
        };
        chain
            .insert_received_headers(vec![remote_header.clone()])
            .await
            .expect("test: insert header");

        // Should retrieve from stored headers
        let retrieved = chain.get_header(10).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.expect("test: header").hash, "remote_hash_10");

        // Non-existent index returns None
        assert!(chain.get_header(999).await.is_none());
    }

    #[tokio::test]
    async fn test_get_known_height() {
        let coord = MatrixCoordinate::new(6, 6, 6).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Chain height is 0 (genesis only), no headers
        assert_eq!(chain.get_known_height().await, 0);

        // Insert headers at height 50
        let header = BlockHeader {
            index: 50,
            hash: "hash_50".to_string(),
            previous_hash: "hash_49".to_string(),
            entries_hash: [0u8; 32],
            entry_count: 1,
        };
        chain
            .insert_received_headers(vec![header])
            .await
            .expect("test: insert header");

        // Known height should be 50 (max of chain=0 and headers=50)
        assert_eq!(chain.get_known_height().await, 50);

        // Add blocks up to height 3
        for i in 1..=3u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        // Known height should still be 50 (headers > chain)
        assert_eq!(chain.get_known_height().await, 50);
    }

    #[tokio::test]
    async fn test_from_blocks_broken_chain_fails() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");

        let genesis = Block::genesis(coord);
        let entry = test_entry(coord);
        // Wrong previous_hash
        let bad_block = Block::new(1, vec![entry], "wrong_hash".to_string());

        let result = NodeBlockchain::from_blocks(coord, vec![genesis, bad_block]);
        assert!(result.is_err());
    }

    // ── Selective chain reconstruction tests ───────────────────────

    #[tokio::test]
    async fn test_has_full_block() {
        let coord = MatrixCoordinate::new(1, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Genesis exists as full block
        assert!(chain.has_full_block(0).await);

        // Index 1 does not exist yet
        assert!(!chain.has_full_block(1).await);

        // Add a block and verify
        let prev = chain.get_head().await.expect("test: head");
        let entry = test_entry(coord);
        let block = Block::new(1, vec![entry], prev.hash.clone());
        chain.insert_block(block).await.expect("test: insert");

        assert!(chain.has_full_block(1).await);

        // Header-only blocks should NOT count as full blocks
        let header = BlockHeader {
            index: 5,
            hash: "hdr5".to_string(),
            previous_hash: "hdr4".to_string(),
            entries_hash: [0u8; 32],
            entry_count: 1,
        };
        chain
            .insert_received_headers(vec![header])
            .await
            .expect("test: insert header");
        assert!(!chain.has_full_block(5).await);
    }

    #[tokio::test]
    async fn test_get_participation_range_genesis_only() {
        let coord = MatrixCoordinate::new(2, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let (start, end) = chain.get_participation_range().await;
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[tokio::test]
    async fn test_get_participation_range_with_blocks() {
        let coord = MatrixCoordinate::new(3, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let (start, end) = chain.get_participation_range().await;
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    #[tokio::test]
    async fn test_prune_to_headers_basic() {
        let coord = MatrixCoordinate::new(4, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Build chain of 6 blocks (genesis + 5)
        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        assert_eq!(chain.get_stats().await.total_blocks, 6);

        // Prune blocks 1..4 (indices 1, 2, 3) to headers
        chain.prune_to_headers(1..4).await;

        // Full blocks 1, 2, 3 should be gone
        assert!(!chain.has_full_block(1).await);
        assert!(!chain.has_full_block(2).await);
        assert!(!chain.has_full_block(3).await);

        // Genesis and blocks 4, 5 should remain
        assert!(chain.has_full_block(0).await);
        assert!(chain.has_full_block(4).await);
        assert!(chain.has_full_block(5).await);

        // Headers should be available for pruned blocks
        assert!(chain.get_header(1).await.is_some());
        assert!(chain.get_header(2).await.is_some());
        assert!(chain.get_header(3).await.is_some());

        // Stats should reflect pruned blocks (6 - 3 = 3)
        assert_eq!(chain.get_stats().await.total_blocks, 3);
    }

    #[tokio::test]
    async fn test_prune_to_headers_preserves_genesis() {
        let coord = MatrixCoordinate::new(5, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let prev = chain.get_head().await.expect("test: head");
        let entry = test_entry(coord);
        let block = Block::new(1, vec![entry], prev.hash.clone());
        chain.insert_block(block).await.expect("test: insert");

        // Try to prune range including genesis
        chain.prune_to_headers(0..2).await;

        // Genesis must survive
        assert!(chain.has_full_block(0).await);

        // Block 1 should be pruned
        assert!(!chain.has_full_block(1).await);
        assert!(chain.get_header(1).await.is_some());
    }

    #[tokio::test]
    async fn test_prune_to_headers_empty_range() {
        let coord = MatrixCoordinate::new(6, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let stats_before = chain.get_stats().await;
        chain.prune_to_headers(10..10).await;
        let stats_after = chain.get_stats().await;

        assert_eq!(stats_before.total_blocks, stats_after.total_blocks);
    }

    #[tokio::test]
    async fn test_participation_range_after_prune() {
        let coord = MatrixCoordinate::new(7, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        // Prune middle blocks
        chain.prune_to_headers(2..4).await;

        // Range spans genesis (0) to block 5, even though 2 and 3 are pruned
        let (start, end) = chain.get_participation_range().await;
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    // ── F6: per-asset shard authorization anchor ────────────────────────

    /// Build an entry whose asset is stored as `Sharded` with the given
    /// shard hashes, so `authorizes_shard` can find those shards.
    fn sharded_entry(coord: MatrixCoordinate, shard_hashes: Vec<[u8; 32]>) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            signed_proof: None,
            storage_pointer: StoragePointer::Sharded {
                shard_hashes,
                placements: vec![coord],
            },
            registration: reg,
        }
    }

    #[tokio::test]
    async fn test_authorizes_shard_registered_and_unregistered() {
        let coord = MatrixCoordinate::new(8, 8, 8).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        let genesis = chain.get_head().await.expect("test: head");

        let known_shard = [0x11u8; 32];
        let other_shard = [0x22u8; 32];
        let entry = sharded_entry(coord, vec![known_shard, other_shard]);
        let block = Block::new(1, vec![entry], genesis.hash.clone());
        chain.insert_block(block).await.expect("test: insert");

        // Shards belonging to a registered sharded asset are authorized.
        assert!(chain.authorizes_shard(&known_shard).await);
        assert!(chain.authorizes_shard(&other_shard).await);

        // An unknown shard (no registered asset) is NOT authorized.
        assert!(!chain.authorizes_shard(&[0xFFu8; 32]).await);
    }

    #[tokio::test]
    async fn test_authorizes_shard_ignores_non_sharded_assets() {
        // Genesis + a Local-pointer asset carry no shard hashes, so no
        // shard is authorized by them.
        let coord = MatrixCoordinate::new(9, 9, 9).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        assert!(!chain.authorizes_shard(&[0x33u8; 32]).await);
    }
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Light node mode (Phase K.1).
//!
//! A *light node* is a HyperMesh participant that does **not** host full
//! blocks, shards, or run the asset pipeline. Instead it tracks the
//! header chain (lightweight `BlockHeader`s, ~150 bytes each) and verifies
//! state-proof commitments against headers it has received.
//!
//! Memory budget: ~256 MB.  Storage budget: ~1 GB header chain.
//!
//! ## Components
//!
//! - [`HeaderSyncManager`] — append-only header chain with previous-hash
//!   linkage validation. Receives `HeaderResponse` payloads (Phase I.1) and
//!   maintains a chain tip.
//! - [`WitnessedProofVerifier`] — verifies a state-proof's BLAKE3 commitment
//!   is included in the header chain. For K.1 alpha the verifier accepts a
//!   proof whose owning header has been successfully ingested (which already
//!   validates linkage). Inclusion-proof aggregation against
//!   `entries_hash` is reserved for K.1.5.
//! - [`LightMode`] — runtime selector (`Full | Light | ThinClient`) parsed
//!   from `--mode` CLI flag. The startup path in `commands/connect.rs`
//!   branches on this enum.
//!
//! ## Alpha-default inert
//!
//! `LightMode::Full` is the default. The `--mode light` flag is opt-in.
//! When the daemon is started in `LightMode::Light` only the
//! `HeaderSyncManager` runs; full block hosting, shard transport, asset
//! pipeline writes, Caesar EVP, and the ngauge bridge are skipped. The
//! reduced startup path is a follow-up sub-step (K.1.5) — for K.1 we
//! ship the types, the flag, the verifier, and the tests.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::blockchain::block::BlockHeader;

/// Light-client error type.
#[derive(Debug, Error)]
pub enum LightClientError {
    /// The incoming header's `previous_hash` did not match the
    /// current chain tip's `hash`, or its `index` did not equal
    /// `tip.index + 1`.
    #[error("header does not chain to current tip (got prev={got_prev} expected={expected_prev})")]
    OrphanHeader {
        /// Hash claimed by the incoming header as its predecessor.
        got_prev: String,
        /// Hash of the current chain tip (what we expected).
        expected_prev: String,
    },

    /// The incoming header's index is not exactly `tip.index + 1`.
    #[error("header index out of order (got {got} expected {expected})")]
    IndexOutOfOrder {
        /// Index reported by the incoming header.
        got: u64,
        /// Index we expected (current tip index + 1).
        expected: u64,
    },

    /// The header at the requested index is not in our local chain.
    #[error("header not found at index {0}")]
    HeaderNotFound(u64),

    /// Genesis header (index 0) is required before any subsequent
    /// header can be ingested.
    #[error("genesis header missing — ingest header at index 0 first")]
    GenesisMissing,

    /// A header with the given index is already present and conflicts
    /// with the incoming one (different hash).
    #[error("header conflict at index {index}: have {have} got {got}")]
    Conflict {
        /// Index where the conflict occurred.
        index: u64,
        /// Hash of the header we already have.
        have: String,
        /// Hash of the incoming, conflicting header.
        got: String,
    },
}

/// Runtime selector for the node startup path.
///
/// - `Full` — default behaviour: full block hosting, shard storage,
///   asset pipeline, Caesar EVP, ngauge bridge.
/// - `Light` — header-only sync, no full block hosting, no shard hosting,
///   no asset pipeline writes. Memory ~256 MB target, storage ~1 GB cap.
/// - `ThinClient` — reserved for K.2: zero local chain state, all
///   reads/writes flow through a remote daemon via capability-token
///   authenticated SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightMode {
    /// Default: full block hosting + shard storage + pipeline.
    Full,
    /// Header-only sync, no shard or asset state.
    Light,
    /// Reserved for K.2 — no local chain, remote daemon via SDK.
    ThinClient,
}

impl Default for LightMode {
    fn default() -> Self {
        Self::Full
    }
}

impl std::fmt::Display for LightMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Light => write!(f, "light"),
            Self::ThinClient => write!(f, "thin"),
        }
    }
}

impl std::str::FromStr for LightMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "light" => Ok(Self::Light),
            "thin" | "thin-client" | "thinclient" => Ok(Self::ThinClient),
            other => Err(format!(
                "unknown light mode '{}': expected one of full|light|thin",
                other
            )),
        }
    }
}

/// Append-only header chain for a light node.
///
/// Stores the full header set in memory keyed by index. Phase K.1 alpha
/// keeps the chain in RAM; K.1.5 will persist it to disk under
/// `<data_dir>/light_headers/`.
#[derive(Default)]
pub struct HeaderSyncManager {
    /// All ingested headers keyed by `BlockHeader::index`.
    headers: Arc<RwLock<HashMap<u64, BlockHeader>>>,
    /// Cached pointer to the highest-index ingested header.
    chain_tip: Arc<RwLock<Option<BlockHeader>>>,
}

impl HeaderSyncManager {
    /// Construct an empty manager. The genesis header must be the first
    /// header ingested via [`ingest_header`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a single header.
    ///
    /// Validation rules:
    /// - The first header must have index 0 (genesis).
    /// - Subsequent headers must have `index == tip.index + 1` and
    ///   `previous_hash == tip.hash`.
    /// - If a header at the same index already exists, it must have the
    ///   identical hash; otherwise a [`LightClientError::Conflict`] is
    ///   returned (defends against malicious peers).
    pub async fn ingest_header(
        &self,
        header: BlockHeader,
    ) -> Result<(), LightClientError> {
        let mut tip = self.chain_tip.write().await;
        let mut headers = self.headers.write().await;

        // Conflict check first — same index, different hash is fatal.
        if let Some(existing) = headers.get(&header.index) {
            if existing.hash == header.hash {
                debug!(
                    "header at index {} already ingested (idempotent)",
                    header.index
                );
                return Ok(());
            }
            return Err(LightClientError::Conflict {
                index: header.index,
                have: existing.hash.clone(),
                got: header.hash.clone(),
            });
        }

        match tip.as_ref() {
            None => {
                // First ever ingest must be genesis.
                if header.index != 0 {
                    return Err(LightClientError::GenesisMissing);
                }
                info!(
                    "light client: ingested genesis header (hash={})",
                    &header.hash[..16.min(header.hash.len())]
                );
            }
            Some(current_tip) => {
                let expected_index = current_tip.index + 1;
                if header.index != expected_index {
                    return Err(LightClientError::IndexOutOfOrder {
                        got: header.index,
                        expected: expected_index,
                    });
                }
                if header.previous_hash != current_tip.hash {
                    return Err(LightClientError::OrphanHeader {
                        got_prev: header.previous_hash.clone(),
                        expected_prev: current_tip.hash.clone(),
                    });
                }
                debug!(
                    "light client: ingested header #{} (hash={})",
                    header.index,
                    &header.hash[..16.min(header.hash.len())]
                );
            }
        }

        headers.insert(header.index, header.clone());
        *tip = Some(header);
        Ok(())
    }

    /// Return the current chain tip (highest-index ingested header).
    pub async fn chain_tip(&self) -> Option<BlockHeader> {
        self.chain_tip.read().await.clone()
    }

    /// Return the header at the given index, if known.
    pub async fn header_at(&self, index: u64) -> Option<BlockHeader> {
        self.headers.read().await.get(&index).cloned()
    }

    /// Number of headers currently held.
    pub async fn header_count(&self) -> usize {
        self.headers.read().await.len()
    }

    /// Tip height (`tip.index + 1`), or 0 if no headers ingested.
    pub async fn tip_height(&self) -> u64 {
        self.chain_tip
            .read()
            .await
            .as_ref()
            .map(|h| h.index + 1)
            .unwrap_or(0)
    }
}

/// Verifies state-proof commitments against the header chain.
///
/// Phase K.1 alpha verification: a header's `entries_hash` is
/// `BLAKE3(asset_hash || proof_hash, ...)` over each entry. Without
/// the full block we cannot reconstruct a Merkle inclusion proof at K.1
/// (no per-entry path is shipped over the wire today). Therefore the
/// alpha check is "the header at the given index was successfully
/// ingested" — which already proves the header chains correctly to
/// genesis. K.1.5 will extend `HeaderResponse` with per-entry inclusion
/// witnesses and the verifier will then accept only proofs whose
/// `proof_hash` appears in the witness path.
///
/// This is documented as accepted alpha behaviour in `papers/HYPERMESH.md`
/// §10 (Light Client Tier).
pub struct WitnessedProofVerifier {
    sync_manager: Arc<HeaderSyncManager>,
}

impl WitnessedProofVerifier {
    /// Construct a verifier that consults the given header chain.
    pub fn new(sync_manager: Arc<HeaderSyncManager>) -> Self {
        Self { sync_manager }
    }

    /// Verify a state-proof commitment against the header at `block_index`.
    ///
    /// Returns `Ok(true)` when the header is known (i.e. chains correctly
    /// to genesis via earlier `ingest_header` calls). Returns
    /// [`LightClientError::HeaderNotFound`] for unknown indices so the
    /// caller can decide to fetch the missing header before retrying.
    ///
    /// The `proof_hash` is currently used only for diagnostic logging in
    /// alpha — see module docs for K.1.5 inclusion-proof aggregation.
    pub async fn verify_proof(
        &self,
        block_index: u64,
        proof_hash: &[u8],
    ) -> Result<bool, LightClientError> {
        let header = self
            .sync_manager
            .header_at(block_index)
            .await
            .ok_or(LightClientError::HeaderNotFound(block_index))?;

        debug!(
            "light client verify_proof index={} entries_hash={} given_proof_hash={}",
            block_index,
            hex::encode(header.entries_hash),
            hex::encode(proof_hash),
        );

        // K.1 alpha: header presence (which implies linkage to genesis)
        // is the witness. K.1.5 will compute the Merkle path against
        // header.entries_hash from the wire-supplied witness.
        Ok(true)
    }

    /// Verify the proof against the *tip* header. Convenience wrapper for
    /// callers that just want "did this proof commit to the latest known
    /// state".
    pub async fn verify_proof_at_tip(
        &self,
        proof_hash: &[u8],
    ) -> Result<bool, LightClientError> {
        let tip = self
            .sync_manager
            .chain_tip()
            .await
            .ok_or(LightClientError::GenesisMissing)?;
        self.verify_proof(tip.index, proof_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_header(index: u64, hash: &str, prev: &str) -> BlockHeader {
        BlockHeader {
            index,
            hash: hash.to_string(),
            previous_hash: prev.to_string(),
            entries_hash: *blake3::hash(format!("entries-{index}").as_bytes()).as_bytes(),
            entry_count: 1,
        }
    }

    #[test]
    fn light_mode_default_is_full() {
        assert_eq!(LightMode::default(), LightMode::Full);
    }

    #[test]
    fn light_mode_parses() {
        assert_eq!("full".parse::<LightMode>().expect("test: parse full"), LightMode::Full);
        assert_eq!("light".parse::<LightMode>().expect("test: parse light"), LightMode::Light);
        assert_eq!("thin".parse::<LightMode>().expect("test: parse thin"), LightMode::ThinClient);
        assert!("nope".parse::<LightMode>().is_err());
    }

    #[tokio::test]
    async fn ingest_genesis_then_one() {
        let mgr = HeaderSyncManager::new();
        mgr.ingest_header(fake_header(0, "g", "0")).await.expect("test: genesis");
        mgr.ingest_header(fake_header(1, "h1", "g")).await.expect("test: ingest 1");
        let tip = mgr.chain_tip().await.expect("test: tip");
        assert_eq!(tip.index, 1);
        assert_eq!(tip.hash, "h1");
    }

    #[tokio::test]
    async fn missing_genesis_rejected() {
        let mgr = HeaderSyncManager::new();
        let err = mgr
            .ingest_header(fake_header(1, "h1", "g"))
            .await
            .unwrap_err();
        matches!(err, LightClientError::GenesisMissing);
    }
}

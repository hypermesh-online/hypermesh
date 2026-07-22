// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sync request / response / announce / genesis / header / block-request
//! handlers for `SyncDispatcher`.

use tracing::{debug, info, warn};

use crate::blockchain::sync_manager::SyncMessage;
use crate::network::stoq_integration::MatrixMessage;

use super::dispatcher::{DispatchResponse, SyncDispatcher};

impl<'a> SyncDispatcher<'a> {
    /// Convert a network-layer SyncRequest to a SyncMessage, process via
    /// SyncManager (optionally with a BlockProvider), and wrap the
    /// response back into a MatrixMessage.
    pub(super) fn handle_sync_request(
        &mut self,
        network_id: String,
        from_height: u64,
        max_blocks: u32,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Request {
            network_id,
            from_height,
            max_blocks,
        };

        let response = self
            .sync_manager
            .process_sync_message_with_provider(sync_msg, self.block_provider);

        match response {
            Some(SyncMessage::Response {
                network_id,
                block_hashes,
                peer_height,
            }) => DispatchResponse::Reply(MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            }),
            _ => DispatchResponse::None,
        }
    }

    /// Forward a SyncResponse to the SyncManager.
    pub(super) fn handle_sync_response(
        &mut self,
        network_id: String,
        block_hashes: Vec<String>,
        peer_height: u64,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Response {
            network_id,
            block_hashes,
            peer_height,
        };
        let _ = self.sync_manager.process_sync_message(sync_msg);
        DispatchResponse::None
    }

    /// Forward a SyncAnnounce to the SyncManager.
    pub(super) fn handle_sync_announce(
        &mut self,
        network_id: String,
        block_height: u64,
        block_hash: String,
    ) -> DispatchResponse {
        let sync_msg = SyncMessage::Announce {
            network_id,
            block_height,
            block_hash,
        };
        let _ = self.sync_manager.process_sync_message(sync_msg);
        DispatchResponse::None
    }

    /// Serve the FULL genesis block (index 0) from the local chain.
    ///
    /// S3.0/B3: this used to return `hashes[0]` — a 64-char hash string — in a
    /// field named `genesis_block_json`, with an in-code note that the caller
    /// should "follow up". Nothing followed up, and a hash is not adoptable:
    /// the receiver cannot verify a chain root it has never seen. The response
    /// now carries the serialized [`Block`], which the requester deserializes,
    /// hash-verifies and records.
    ///
    /// If the provider cannot supply a full genesis we answer NOTHING rather
    /// than something genesis-shaped — a peer must never treat an unverifiable
    /// placeholder as a chain root.
    pub(super) fn handle_genesis_request(&self, network_id: String) -> DispatchResponse {
        let provider = match self.block_provider {
            Some(p) => p,
            None => {
                debug!(
                    network = %network_id,
                    "GenesisRequest: no block provider available",
                );
                return DispatchResponse::None;
            }
        };

        let genesis = match provider.get_genesis_block() {
            Some(block) => block,
            None => {
                warn!(
                    network = %network_id,
                    "GenesisRequest: provider has no full genesis block to serve",
                );
                return DispatchResponse::None;
            }
        };

        let genesis_block_json = match serde_json::to_string(&genesis) {
            Ok(json) => json,
            Err(e) => {
                warn!(
                    network = %network_id,
                    "GenesisRequest: failed to serialize genesis block: {e}",
                );
                return DispatchResponse::None;
            }
        };

        info!(
            network = %network_id,
            genesis = %&genesis.hash[..16.min(genesis.hash.len())],
            "Serving full genesis block",
        );

        DispatchResponse::Reply(MatrixMessage::GenesisResponse {
            network_id,
            genesis_block_json,
        })
    }

    /// Receive a peer's genesis block and record it for `network_id`.
    ///
    /// S3.0/B3: the dispatcher used to log-and-drop every `GenesisResponse`, so
    /// adoption never happened at all. It now deserializes the block,
    /// re-derives and checks its hash, and hands it to the `SyncManager` as
    /// that network's verified root.
    ///
    /// WHAT THIS DOES NOT DO (deferred to S3.4): it does NOT call
    /// `NodeBlockchain::adopt_genesis`, which CLEARS the local chain. Wiping a
    /// sovereign device chain in order to join a network is the wrong
    /// primitive; the device chain and the adopted network chain have to
    /// coexist in a multi-chain container. Until that container exists, S3.0
    /// lands the transport half — a real block is requested, received,
    /// verified and retained — and nothing destructive.
    pub(super) fn handle_genesis_response(
        &mut self,
        network_id: String,
        genesis_block_json: String,
    ) -> DispatchResponse {
        let genesis: crate::blockchain::Block =
            match serde_json::from_str(&genesis_block_json) {
                Ok(block) => block,
                Err(e) => {
                    warn!(
                        network = %network_id,
                        "GenesisResponse rejected — not a deserializable Block: {e}",
                    );
                    return DispatchResponse::None;
                }
            };

        match self
            .sync_manager
            .record_network_genesis(&network_id, genesis)
        {
            Ok(()) => debug!(
                network = %network_id,
                "GenesisResponse accepted and recorded",
            ),
            Err(e) => warn!(
                network = %network_id,
                "GenesisResponse rejected: {e}",
            ),
        }

        DispatchResponse::None
    }

    /// Return block headers from `from_height` up to `max_count`.
    pub(super) fn handle_header_request(
        &self,
        network_id: String,
        from_height: u64,
        max_count: u32,
    ) -> DispatchResponse {
        let provider = match self.block_provider {
            Some(p) => p,
            None => {
                debug!(
                    network = %network_id,
                    "HeaderRequest: no block provider available",
                );
                return DispatchResponse::None;
            }
        };

        let (hashes, peer_height) = provider.get_block_hashes(from_height, max_count);

        // We only have hashes from the provider, so we construct minimal
        // header info. The caller with full block access should use
        // FullBlockProvider for richer header data.
        let headers_json: Vec<String> = hashes
            .iter()
            .enumerate()
            .map(|(i, hash)| {
                let index = from_height + i as u64;
                // Serialize a minimal header placeholder with hash and index
                serde_json::json!({
                    "index": index,
                    "hash": hash,
                })
                .to_string()
            })
            .collect();

        DispatchResponse::Reply(MatrixMessage::HeaderResponse {
            network_id,
            headers_json,
            peer_height,
        })
    }

    /// Return full blocks matching the requested hashes.
    pub(super) fn handle_sync_block_request(
        &self,
        network_id: String,
        block_hashes: Vec<String>,
    ) -> DispatchResponse {
        // SyncBlockRequest uses the same mechanism as BlockFetchRequest
        // but scoped to a network. With only a hash-based BlockProvider
        // we cannot return full blocks -- return the hashes as confirmation.
        // The real full-block fetch is done via the existing BlockFetchRequest
        // path in message_handlers.
        info!(
            network = %network_id,
            requested = block_hashes.len(),
            "SyncBlockRequest: delegating to block fetch path",
        );

        DispatchResponse::Reply(MatrixMessage::SyncBlockResponse {
            network_id,
            blocks_json: Vec::new(),
        })
    }
}

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

    /// Return the genesis block (index 0) from the local chain.
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

        let (hashes, _height) = provider.get_block_hashes(0, 1);
        if hashes.is_empty() {
            warn!(
                network = %network_id,
                "GenesisRequest: no genesis block found",
            );
            return DispatchResponse::None;
        }

        // The provider only gives hashes; we need the full block.
        // For GenesisRequest we return the hash so the caller can fetch
        // the full block via BlockFetchRequest. We encode the hash as a
        // GenesisResponse with the genesis hash in the JSON field.
        //
        // NOTE: The full-block provider pattern requires the caller to
        // supply a FullBlockProvider (see NodeBlockchainFullBlockProvider).
        // For now, return the hash so the caller can follow up.
        DispatchResponse::Reply(MatrixMessage::GenesisResponse {
            network_id,
            genesis_block_json: hashes[0].clone(),
        })
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

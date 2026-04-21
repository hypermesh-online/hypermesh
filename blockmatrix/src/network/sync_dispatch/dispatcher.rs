// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `SyncDispatcher` struct, top-level dispatch routing, and
//! `DispatchResponse` enum.

use tracing::debug;

use crate::blockchain::sync_manager::{BlockProvider, SyncManager};
use crate::network::reflector_pool::ReflectorPool;
use crate::network::stoq_integration::MatrixMessage;

use hypermesh_lib::MatrixPosition;

/// Coordinates message dispatch between the network layer and the
/// blockchain sync / reflector subsystems.
///
/// Holds mutable references to the subsystems and an optional
/// `BlockProvider` for populating sync responses with real data.
pub struct SyncDispatcher<'a> {
    /// Sync manager handling chain synchronisation state.
    pub sync_manager: &'a mut SyncManager,
    /// Reflector pool tracking block-serving peers.
    pub reflector_pool: &'a mut ReflectorPool,
    /// Optional provider for looking up local block data.
    pub block_provider: Option<&'a dyn BlockProvider>,
}

/// Response produced by the dispatcher, ready to be serialised and
/// sent back through the STOQ transport.
#[derive(Debug)]
pub enum DispatchResponse {
    /// A sync message that should be sent back to the requesting peer.
    Reply(MatrixMessage),
    /// No response needed.
    None,
}

impl<'a> SyncDispatcher<'a> {
    /// Dispatch a single `MatrixMessage` to the correct subsystem.
    ///
    /// Returns a `DispatchResponse` that the caller should send back
    /// over the STOQ connection (if it is a `Reply`).
    pub fn dispatch(
        &mut self,
        msg: MatrixMessage,
        sender_node_id: &str,
        sender_position: MatrixPosition,
    ) -> DispatchResponse {
        match msg {
            MatrixMessage::SyncRequest {
                network_id,
                from_height,
                max_blocks,
            } => self.handle_sync_request(network_id, from_height, max_blocks),

            MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            } => self.handle_sync_response(network_id, block_hashes, peer_height),

            MatrixMessage::SyncAnnounce {
                network_id,
                block_height,
                block_hash,
            } => self.handle_sync_announce(network_id, block_height, block_hash),

            MatrixMessage::ReflectorHeartbeat {
                network_id,
                block_height,
                health_score,
            } => self.handle_reflector_heartbeat(
                &network_id,
                sender_node_id,
                sender_position,
                block_height,
                health_score,
            ),

            MatrixMessage::GenesisRequest { network_id } => {
                self.handle_genesis_request(network_id)
            }

            MatrixMessage::HeaderRequest {
                network_id,
                from_height,
                max_count,
            } => self.handle_header_request(network_id, from_height, max_count),

            MatrixMessage::SyncBlockRequest {
                network_id,
                block_hashes,
            } => self.handle_sync_block_request(network_id, block_hashes),

            // Response variants are handled by the caller, not dispatched
            MatrixMessage::GenesisResponse { .. }
            | MatrixMessage::HeaderResponse { .. }
            | MatrixMessage::SyncBlockResponse { .. } => {
                debug!("SyncDispatcher: response message handled by caller");
                DispatchResponse::None
            }

            other => {
                debug!("SyncDispatcher ignoring non-sync message: {:?}", other);
                DispatchResponse::None
            }
        }
    }
}

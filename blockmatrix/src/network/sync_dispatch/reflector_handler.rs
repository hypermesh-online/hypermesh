// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reflector heartbeat handling for `SyncDispatcher`.

use tracing::debug;

use crate::bootstrap::PrivacyMode;
use crate::network::reflector_pool::Reflector;
use hypermesh_lib::MatrixPosition;

use super::dispatcher::{DispatchResponse, SyncDispatcher};

impl<'a> SyncDispatcher<'a> {
    /// Register or update a reflector from a heartbeat message.
    pub(super) fn handle_reflector_heartbeat(
        &mut self,
        network_id: &str,
        sender_node_id: &str,
        sender_position: MatrixPosition,
        block_height: u64,
        health_score: f64,
    ) -> DispatchResponse {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let reflector = Reflector {
            node_id: sender_node_id.to_string(),
            position: sender_position,
            last_seen: now_secs,
            block_height,
            health_score: health_score.clamp(0.0, 1.0),
            privacy_mode: PrivacyMode::PUBLIC,
        };

        self.reflector_pool
            .register_reflector(network_id, reflector);

        debug!(
            network = %network_id,
            node = %sender_node_id,
            height = block_height,
            health = health_score,
            "Processed reflector heartbeat"
        );

        DispatchResponse::None
    }
}

// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Wire-format messages exchanged between reflector pool nodes.
//!
//! Each message is serialized as a 4-byte little-endian length prefix
//! followed by a bincode payload. All timestamps are stored as `u64`
//! unix epoch seconds because `SystemTime` does not implement `Serialize`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use hypermesh_lib::{MatrixPosition, NetworkId};

/// Messages exchanged between reflector pool nodes for Network-scope
/// blockchain synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReflectorMessage {
    /// Periodic health signal advertising a node's availability.
    Heartbeat {
        /// Identifier of the sending node.
        node_id: String,
        /// Network this heartbeat applies to.
        network_id: NetworkId,
        /// Sender's current block height.
        block_height: u64,
        /// Self-reported health score (0.0 to 1.0).
        health_score: f64,
        /// Sender's position in the Block-MATRIX topology.
        position: MatrixPosition,
        /// Unix epoch seconds when the heartbeat was created.
        timestamp: u64,
    },

    /// Request blocks from a peer for chain synchronization.
    SyncRequest {
        /// Network to synchronize.
        network_id: NetworkId,
        /// Start syncing from this block height.
        from_height: u64,
        /// Maximum number of blocks to return.
        max_blocks: u32,
        /// Node that is requesting the sync.
        requesting_node: String,
    },

    /// Response containing available block hashes.
    SyncResponse {
        /// Network the response pertains to.
        network_id: NetworkId,
        /// Block hashes available in the requested range.
        block_hashes: Vec<[u8; 32]>,
        /// Responding peer's current chain height.
        peer_height: u64,
        /// Node that produced this response.
        responding_node: String,
    },

    /// Announce a new block to the reflector pool.
    BlockAnnounce {
        /// Network the block belongs to.
        network_id: NetworkId,
        /// Height of the announced block.
        block_height: u64,
        /// Hash of the announced block.
        block_hash: [u8; 32],
        /// Node making the announcement.
        announcing_node: String,
    },

    /// Quorum confirmation that a peer has validated a block height.
    QuorumConfirm {
        /// Network the confirmation applies to.
        network_id: NetworkId,
        /// Block height being confirmed.
        block_height: u64,
        /// Node confirming the block.
        confirming_node: String,
    },
}

impl ReflectorMessage {
    /// Serialize this message with a 4-byte LE length prefix.
    pub fn serialize_message(&self) -> Result<Vec<u8>> {
        let payload = bincode::serialize(self)
            .context("failed to serialize ReflectorMessage")?;
        let len = payload.len() as u32;
        let mut buf = Vec::with_capacity(4 + payload.len());
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&payload);
        Ok(buf)
    }

    /// Deserialize a length-prefixed message from raw bytes.
    ///
    /// The first 4 bytes must be a little-endian `u32` indicating the
    /// length of the bincode payload that follows.
    pub fn deserialize_message(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            anyhow::bail!("message too short: need at least 4 bytes for length prefix");
        }
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let payload = data
            .get(4..4 + len)
            .context("message payload shorter than declared length")?;
        let msg: Self = bincode::deserialize(payload)
            .context("failed to deserialize ReflectorMessage payload")?;
        Ok(msg)
    }

    /// Human-readable type tag for logging and metrics.
    pub fn message_type(&self) -> &str {
        match self {
            Self::Heartbeat { .. } => "heartbeat",
            Self::SyncRequest { .. } => "sync_request",
            Self::SyncResponse { .. } => "sync_response",
            Self::BlockAnnounce { .. } => "block_announce",
            Self::QuorumConfirm { .. } => "quorum_confirm",
        }
    }

    /// Extract the `NetworkId` from any variant.
    pub fn network_id(&self) -> &NetworkId {
        match self {
            Self::Heartbeat { network_id, .. }
            | Self::SyncRequest { network_id, .. }
            | Self::SyncResponse { network_id, .. }
            | Self::BlockAnnounce { network_id, .. }
            | Self::QuorumConfirm { network_id, .. } => network_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_network_id() -> NetworkId {
        NetworkId([0xAA; 16])
    }

    fn test_position() -> MatrixPosition {
        MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }
    }

    #[test]
    fn test_heartbeat_round_trip() {
        let msg = ReflectorMessage::Heartbeat {
            node_id: "node-1".to_string(),
            network_id: test_network_id(),
            block_height: 42,
            health_score: 0.95,
            position: test_position(),
            timestamp: 1700000000,
        };

        let bytes = msg.serialize_message().expect("test: serialize");
        let decoded = ReflectorMessage::deserialize_message(&bytes).expect("test: deserialize");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_sync_request_round_trip() {
        let msg = ReflectorMessage::SyncRequest {
            network_id: test_network_id(),
            from_height: 100,
            max_blocks: 50,
            requesting_node: "requester".to_string(),
        };

        let bytes = msg.serialize_message().expect("test: serialize");
        let decoded = ReflectorMessage::deserialize_message(&bytes).expect("test: deserialize");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_sync_response_round_trip() {
        let msg = ReflectorMessage::SyncResponse {
            network_id: test_network_id(),
            block_hashes: vec![[0xBB; 32], [0xCC; 32]],
            peer_height: 200,
            responding_node: "responder".to_string(),
        };

        let bytes = msg.serialize_message().expect("test: serialize");
        let decoded = ReflectorMessage::deserialize_message(&bytes).expect("test: deserialize");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_block_announce_round_trip() {
        let msg = ReflectorMessage::BlockAnnounce {
            network_id: test_network_id(),
            block_height: 77,
            block_hash: [0xDD; 32],
            announcing_node: "announcer".to_string(),
        };

        let bytes = msg.serialize_message().expect("test: serialize");
        let decoded = ReflectorMessage::deserialize_message(&bytes).expect("test: deserialize");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_quorum_confirm_round_trip() {
        let msg = ReflectorMessage::QuorumConfirm {
            network_id: test_network_id(),
            block_height: 88,
            confirming_node: "confirmer".to_string(),
        };

        let bytes = msg.serialize_message().expect("test: serialize");
        let decoded = ReflectorMessage::deserialize_message(&bytes).expect("test: deserialize");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_message_type_names() {
        let hb = ReflectorMessage::Heartbeat {
            node_id: String::new(),
            network_id: test_network_id(),
            block_height: 0,
            health_score: 0.0,
            position: test_position(),
            timestamp: 0,
        };
        assert_eq!(hb.message_type(), "heartbeat");

        let sr = ReflectorMessage::SyncRequest {
            network_id: test_network_id(),
            from_height: 0,
            max_blocks: 0,
            requesting_node: String::new(),
        };
        assert_eq!(sr.message_type(), "sync_request");

        let sp = ReflectorMessage::SyncResponse {
            network_id: test_network_id(),
            block_hashes: vec![],
            peer_height: 0,
            responding_node: String::new(),
        };
        assert_eq!(sp.message_type(), "sync_response");

        let ba = ReflectorMessage::BlockAnnounce {
            network_id: test_network_id(),
            block_height: 0,
            block_hash: [0u8; 32],
            announcing_node: String::new(),
        };
        assert_eq!(ba.message_type(), "block_announce");

        let qc = ReflectorMessage::QuorumConfirm {
            network_id: test_network_id(),
            block_height: 0,
            confirming_node: String::new(),
        };
        assert_eq!(qc.message_type(), "quorum_confirm");
    }

    #[test]
    fn test_network_id_extraction() {
        let net = NetworkId([0x11; 16]);
        let msg = ReflectorMessage::Heartbeat {
            node_id: "n".to_string(),
            network_id: net,
            block_height: 0,
            health_score: 0.0,
            position: test_position(),
            timestamp: 0,
        };
        assert_eq!(*msg.network_id(), net);
    }

    #[test]
    fn test_deserialize_too_short() {
        let short = [0u8; 2];
        let result = ReflectorMessage::deserialize_message(&short);
        assert!(result.is_err());
    }
}

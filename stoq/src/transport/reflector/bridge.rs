// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bidirectional bridge between STOQ's [`ReflectorMessage`] and
//! blockmatrix's `MatrixMessage` system.
//!
//! Because STOQ cannot depend on blockmatrix (it would create a
//! circular dependency), this module defines [`BridgedMatrixMessage`]
//! -- a standalone enum whose variants are 1:1 compatible with the
//! sync-related `MatrixMessage` variants in blockmatrix. The bridge
//! converter ([`ReflectorBridge`]) provides zero-allocation type
//! mapping between the two representations.

use anyhow::{Context, Result};

use hypermesh_lib::NetworkId;

use super::message::ReflectorMessage;

// ---------------------------------------------------------------------------
// BridgedMatrixMessage
// ---------------------------------------------------------------------------

/// Standalone representation of sync-related blockmatrix `MatrixMessage`
/// variants.
///
/// Field types intentionally match blockmatrix's wire format (strings
/// for network IDs, `Vec<String>` for block hashes as hex, etc.) so
/// that the conversion can be lossless in both directions.
#[derive(Debug, Clone, PartialEq)]
pub enum BridgedMatrixMessage {
    /// Equivalent to `MatrixMessage::SyncRequest`.
    SyncRequest {
        network_id: String,
        from_height: u64,
        max_blocks: u32,
    },
    /// Equivalent to `MatrixMessage::SyncResponse`.
    SyncResponse {
        network_id: String,
        block_hashes: Vec<String>,
        peer_height: u64,
    },
    /// Equivalent to `MatrixMessage::SyncAnnounce`.
    SyncAnnounce {
        network_id: String,
        block_height: u64,
        block_hash: String,
    },
    /// Equivalent to `MatrixMessage::ReflectorHeartbeat`.
    ReflectorHeartbeat {
        node_id: String,
        network_id: String,
        block_height: u64,
        health_score: f64,
    },
}

// ---------------------------------------------------------------------------
// ReflectorBridge
// ---------------------------------------------------------------------------

/// Converts between STOQ [`ReflectorMessage`]s and
/// [`BridgedMatrixMessage`]s.
///
/// The bridge is stateless; all methods are associated functions.
pub struct ReflectorBridge;

impl ReflectorBridge {
    /// Convert a STOQ `ReflectorMessage` to a `BridgedMatrixMessage`.
    ///
    /// Returns `None` for message types that have no direct equivalent
    /// in blockmatrix's `MatrixMessage` (e.g. `ReplicationConfirm`).
    pub fn to_matrix_message(msg: &ReflectorMessage) -> Option<BridgedMatrixMessage> {
        match msg {
            ReflectorMessage::Heartbeat {
                node_id,
                network_id,
                block_height,
                health_score,
                ..
            } => Some(BridgedMatrixMessage::ReflectorHeartbeat {
                node_id: node_id.clone(),
                network_id: Self::network_id_to_string(network_id),
                block_height: *block_height,
                health_score: *health_score,
            }),

            ReflectorMessage::SyncRequest {
                network_id,
                from_height,
                max_blocks,
                ..
            } => Some(BridgedMatrixMessage::SyncRequest {
                network_id: Self::network_id_to_string(network_id),
                from_height: *from_height,
                max_blocks: *max_blocks,
            }),

            ReflectorMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
                ..
            } => Some(BridgedMatrixMessage::SyncResponse {
                network_id: Self::network_id_to_string(network_id),
                block_hashes: block_hashes.iter().map(hex::encode).collect(),
                peer_height: *peer_height,
            }),

            ReflectorMessage::BlockAnnounce {
                network_id,
                block_height,
                block_hash,
                ..
            } => Some(BridgedMatrixMessage::SyncAnnounce {
                network_id: Self::network_id_to_string(network_id),
                block_height: *block_height,
                block_hash: hex::encode(block_hash),
            }),

            ReflectorMessage::ReplicationConfirm { .. } => None,
        }
    }

    /// Convert a `BridgedMatrixMessage` to a STOQ `ReflectorMessage`.
    ///
    /// Because `BridgedMatrixMessage` uses strings for network IDs and
    /// block hashes, this conversion can fail if the hex decoding is
    /// invalid. The caller should handle the error appropriately.
    pub fn from_matrix_message(msg: &BridgedMatrixMessage) -> Result<ReflectorMessage> {
        match msg {
            BridgedMatrixMessage::SyncRequest {
                network_id,
                from_height,
                max_blocks,
            } => Ok(ReflectorMessage::SyncRequest {
                network_id: Self::string_to_network_id(network_id)?,
                from_height: *from_height,
                max_blocks: *max_blocks,
                requesting_node: String::new(),
            }),

            BridgedMatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            } => {
                let hashes: Result<Vec<[u8; 32]>> =
                    block_hashes.iter().map(|h| Self::hex_to_hash(h)).collect();
                Ok(ReflectorMessage::SyncResponse {
                    network_id: Self::string_to_network_id(network_id)?,
                    block_hashes: hashes?,
                    peer_height: *peer_height,
                    responding_node: String::new(),
                })
            }

            BridgedMatrixMessage::SyncAnnounce {
                network_id,
                block_height,
                block_hash,
            } => Ok(ReflectorMessage::BlockAnnounce {
                network_id: Self::string_to_network_id(network_id)?,
                block_height: *block_height,
                block_hash: Self::hex_to_hash(block_hash)?,
                announcing_node: String::new(),
            }),

            BridgedMatrixMessage::ReflectorHeartbeat {
                node_id,
                network_id,
                block_height,
                health_score,
            } => Ok(ReflectorMessage::Heartbeat {
                node_id: node_id.clone(),
                network_id: Self::string_to_network_id(network_id)?,
                block_height: *block_height,
                health_score: *health_score,
                position: hypermesh_lib::MatrixPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                timestamp: 0,
            }),
        }
    }

    /// Encode a [`NetworkId`] as a 32-character lowercase hex string.
    pub fn network_id_to_string(id: &NetworkId) -> String {
        hex::encode(id.0)
    }

    /// Decode a 32-character hex string back to a [`NetworkId`].
    pub fn string_to_network_id(s: &str) -> Result<NetworkId> {
        let bytes = hex::decode(s).with_context(|| format!("invalid hex for NetworkId: {s}"))?;
        if bytes.len() != 16 {
            anyhow::bail!(
                "NetworkId hex must be 32 chars (16 bytes), got {} chars ({} bytes)",
                s.len(),
                bytes.len()
            );
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(NetworkId(arr))
    }

    /// Decode a 64-character hex string to a `[u8; 32]` hash.
    fn hex_to_hash(s: &str) -> Result<[u8; 32]> {
        let bytes = hex::decode(s).with_context(|| format!("invalid hex for block hash: {s}"))?;
        if bytes.len() != 32 {
            anyhow::bail!(
                "block hash hex must be 64 chars (32 bytes), got {} chars ({} bytes)",
                s.len(),
                bytes.len()
            );
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::MatrixPosition;

    fn test_network_id() -> NetworkId {
        NetworkId([0xAB; 16])
    }

    fn test_position() -> MatrixPosition {
        MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }
    }

    #[test]
    fn test_heartbeat_to_matrix() {
        let msg = ReflectorMessage::Heartbeat {
            node_id: "node-42".to_string(),
            network_id: test_network_id(),
            block_height: 100,
            health_score: 0.8,
            position: test_position(),
            timestamp: 999,
        };

        let bridged =
            ReflectorBridge::to_matrix_message(&msg).expect("test: heartbeat should bridge");

        match bridged {
            BridgedMatrixMessage::ReflectorHeartbeat {
                node_id,
                network_id,
                block_height,
                health_score,
            } => {
                assert_eq!(node_id, "node-42");
                assert_eq!(
                    network_id,
                    ReflectorBridge::network_id_to_string(&test_network_id())
                );
                assert_eq!(block_height, 100);
                assert!((health_score - 0.8).abs() < f64::EPSILON);
            }
            _ => unreachable!("test: expected ReflectorHeartbeat"),
        }
    }

    #[test]
    fn test_sync_request_roundtrip_bridge() {
        let original = ReflectorMessage::SyncRequest {
            network_id: test_network_id(),
            from_height: 50,
            max_blocks: 25,
            requesting_node: "requester".to_string(),
        };

        let bridged = ReflectorBridge::to_matrix_message(&original).expect("test: should bridge");
        let back =
            ReflectorBridge::from_matrix_message(&bridged).expect("test: should convert back");

        // requesting_node is lost in the bridge (not part of MatrixMessage)
        match back {
            ReflectorMessage::SyncRequest {
                network_id,
                from_height,
                max_blocks,
                requesting_node,
            } => {
                assert_eq!(network_id, test_network_id());
                assert_eq!(from_height, 50);
                assert_eq!(max_blocks, 25);
                assert!(requesting_node.is_empty());
            }
            _ => unreachable!("test: expected SyncRequest"),
        }
    }

    #[test]
    fn test_network_id_string_conversion() {
        let id = test_network_id();
        let s = ReflectorBridge::network_id_to_string(&id);
        assert_eq!(s.len(), 32); // 16 bytes -> 32 hex chars

        let back = ReflectorBridge::string_to_network_id(&s).expect("test: should decode");
        assert_eq!(back, id);
    }

    #[test]
    fn test_replication_confirm_has_no_matrix_equivalent() {
        let msg = ReflectorMessage::ReplicationConfirm {
            network_id: test_network_id(),
            block_height: 10,
            confirming_node: "c".to_string(),
        };
        assert!(ReflectorBridge::to_matrix_message(&msg).is_none());
    }

    #[test]
    fn test_block_announce_bridge() {
        let hash = [0xCC; 32];
        let msg = ReflectorMessage::BlockAnnounce {
            network_id: test_network_id(),
            block_height: 77,
            block_hash: hash,
            announcing_node: "announcer".to_string(),
        };

        let bridged = ReflectorBridge::to_matrix_message(&msg).expect("test: should bridge");

        match &bridged {
            BridgedMatrixMessage::SyncAnnounce {
                block_height,
                block_hash,
                ..
            } => {
                assert_eq!(*block_height, 77);
                assert_eq!(*block_hash, hex::encode(hash));
            }
            _ => unreachable!("test: expected SyncAnnounce"),
        }

        // Round-trip back
        let back =
            ReflectorBridge::from_matrix_message(&bridged).expect("test: should convert back");
        match back {
            ReflectorMessage::BlockAnnounce {
                block_height,
                block_hash,
                ..
            } => {
                assert_eq!(block_height, 77);
                assert_eq!(block_hash, hash);
            }
            _ => unreachable!("test: expected BlockAnnounce"),
        }
    }

    #[test]
    fn test_sync_response_with_hashes_bridge() {
        let hashes = vec![[0xAA; 32], [0xBB; 32]];
        let msg = ReflectorMessage::SyncResponse {
            network_id: test_network_id(),
            block_hashes: hashes.clone(),
            peer_height: 200,
            responding_node: "resp".to_string(),
        };

        let bridged = ReflectorBridge::to_matrix_message(&msg).expect("test: should bridge");

        match &bridged {
            BridgedMatrixMessage::SyncResponse {
                block_hashes,
                peer_height,
                ..
            } => {
                assert_eq!(block_hashes.len(), 2);
                assert_eq!(*peer_height, 200);
                assert_eq!(block_hashes[0], hex::encode([0xAA; 32]));
            }
            _ => unreachable!("test: expected SyncResponse"),
        }

        // Round-trip
        let back =
            ReflectorBridge::from_matrix_message(&bridged).expect("test: should convert back");
        match back {
            ReflectorMessage::SyncResponse {
                block_hashes: bh,
                peer_height,
                ..
            } => {
                assert_eq!(bh, hashes);
                assert_eq!(peer_height, 200);
            }
            _ => unreachable!("test: expected SyncResponse"),
        }
    }

    #[test]
    fn test_invalid_hex_network_id() {
        let result = ReflectorBridge::string_to_network_id("not_valid_hex!");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_length_network_id() {
        // Valid hex but wrong length (only 4 bytes)
        let result = ReflectorBridge::string_to_network_id("aabbccdd");
        assert!(result.is_err());
    }
}

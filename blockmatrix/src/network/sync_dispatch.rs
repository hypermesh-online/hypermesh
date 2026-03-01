// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Dispatches sync-related `MatrixMessage` variants to the appropriate
//! subsystems (`SyncManager`, `ReflectorPool`).
//!
//! This wiring layer bridges Gaps 1, 2, 4, and 5 by:
//! - Converting `MatrixMessage::SyncRequest/SyncResponse/SyncAnnounce`
//!   into `SyncMessage` values and forwarding them to `SyncManager`.
//! - Converting `MatrixMessage::ReflectorHeartbeat` into a
//!   `register_reflector` / `update_health` call on `ReflectorPool`.

use tracing::debug;

use crate::blockchain::sync_manager::{BlockProvider, SyncManager, SyncMessage};
use crate::network::reflector_pool::{Reflector, ReflectorPool};
use crate::network::stoq_integration::MatrixMessage;

use crate::bootstrap::PrivacyMode;
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

            other => {
                debug!("SyncDispatcher ignoring non-sync message: {:?}", other);
                DispatchResponse::None
            }
        }
    }

    /// Convert a network-layer SyncRequest to a SyncMessage, process via
    /// SyncManager (optionally with a BlockProvider), and wrap the
    /// response back into a MatrixMessage.
    fn handle_sync_request(
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
    fn handle_sync_response(
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
    fn handle_sync_announce(
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

    /// Register or update a reflector from a heartbeat message.
    fn handle_reflector_heartbeat(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::sync_manager::SyncConfig;
    use crate::network::reflector_pool::ReflectorConfig;

    /// A trivial BlockProvider that returns predictable hashes.
    struct FakeBlockProvider {
        chain_height: u64,
    }

    impl BlockProvider for FakeBlockProvider {
        fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64) {
            let end = (from_height + max_blocks as u64).min(self.chain_height);
            let hashes: Vec<String> = (from_height..end).map(|h| format!("hash_{h}")).collect();
            (hashes, self.chain_height)
        }
    }

    fn make_sync_manager() -> SyncManager {
        SyncManager::new("device-chain".to_string(), SyncConfig::default())
    }

    fn make_reflector_pool() -> ReflectorPool {
        ReflectorPool::new(ReflectorConfig::default())
    }

    fn zero_position() -> MatrixPosition {
        MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    // ------------------------------------------------------------------
    // Gap 5 tests: Message dispatch routing
    // ------------------------------------------------------------------

    #[test]
    fn test_dispatch_sync_request_without_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "net-1".to_string(),
            from_height: 0,
            max_blocks: 10,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::SyncResponse {
                network_id,
                block_hashes,
                peer_height,
            }) => {
                assert_eq!(network_id, "net-1");
                assert!(block_hashes.is_empty());
                assert_eq!(peer_height, 0);
            }
            other => unreachable!("test: expected SyncResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_sync_request_with_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        let provider = FakeBlockProvider { chain_height: 20 };

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: Some(&provider),
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "net-1".to_string(),
            from_height: 5,
            max_blocks: 10,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        match resp {
            DispatchResponse::Reply(MatrixMessage::SyncResponse {
                block_hashes,
                peer_height,
                ..
            }) => {
                assert_eq!(block_hashes.len(), 10);
                assert_eq!(block_hashes[0], "hash_5");
                assert_eq!(peer_height, 20);
            }
            other => unreachable!("test: expected SyncResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_sync_request_unknown_network() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncRequest {
            network_id: "unknown".to_string(),
            from_height: 0,
            max_blocks: 5,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));
    }

    #[test]
    fn test_dispatch_sync_response_updates_state() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncResponse {
            network_id: "net-1".to_string(),
            block_hashes: Vec::new(),
            peer_height: 42,
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));

        // SyncManager should now be Synchronized at height 42
        use crate::blockchain::sync_manager::SyncState;
        assert_eq!(
            sm.sync_state("net-1"),
            Some(&SyncState::Synchronized {
                last_block_height: 42
            })
        );
    }

    #[test]
    fn test_dispatch_sync_announce_triggers_resync() {
        let config = SyncConfig {
            max_block_lag: 5,
            ..SyncConfig::default()
        };
        let mut sm = SyncManager::new("dev".to_string(), config);
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");
        sm.update_sync_state(
            "net-1",
            crate::blockchain::sync_manager::SyncState::Synchronized {
                last_block_height: 10,
            },
        )
        .expect("test: set synced");

        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::SyncAnnounce {
            network_id: "net-1".to_string(),
            block_height: 100,
            block_hash: "abc".to_string(),
        };

        let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
        assert!(matches!(resp, DispatchResponse::None));

        // Should have transitioned to Syncing
        use crate::blockchain::sync_manager::SyncState;
        assert!(matches!(
            sm.sync_state("net-1"),
            Some(SyncState::Syncing { .. })
        ));
    }

    // ------------------------------------------------------------------
    // Gap 2 tests: ReflectorPool receives heartbeats
    // ------------------------------------------------------------------

    #[test]
    fn test_dispatch_reflector_heartbeat_registers() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "net-1".to_string(),
            block_height: 50,
            health_score: 0.8,
        };

        let pos = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        let resp = dispatcher.dispatch(msg, "reflector-node-1", pos);
        assert!(matches!(resp, DispatchResponse::None));

        assert_eq!(rp.total_count("net-1"), 1);
        let best = rp.get_best_reflectors("net-1", 1);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].node_id, "reflector-node-1");
        assert_eq!(best[0].block_height, 50);
    }

    #[test]
    fn test_dispatch_reflector_heartbeat_updates_existing() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let pos = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        // First heartbeat
        {
            let mut dispatcher = SyncDispatcher {
                sync_manager: &mut sm,
                reflector_pool: &mut rp,
                block_provider: None,
            };
            let msg = MatrixMessage::ReflectorHeartbeat {
                network_id: "net-1".to_string(),
                block_height: 10,
                health_score: 0.5,
            };
            dispatcher.dispatch(msg, "node-A", pos);
        }

        // Second heartbeat with updated data
        {
            let mut dispatcher = SyncDispatcher {
                sync_manager: &mut sm,
                reflector_pool: &mut rp,
                block_provider: None,
            };
            let msg = MatrixMessage::ReflectorHeartbeat {
                network_id: "net-1".to_string(),
                block_height: 25,
                health_score: 0.9,
            };
            dispatcher.dispatch(msg, "node-A", pos);
        }

        // Still one reflector, with updated values
        assert_eq!(rp.total_count("net-1"), 1);
        let best = rp.get_best_reflectors("net-1", 1);
        assert_eq!(best[0].block_height, 25);
        assert!((best[0].health_score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dispatch_ignores_non_sync_messages() {
        let mut sm = make_sync_manager();
        let mut rp = make_reflector_pool();

        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };

        let msg = MatrixMessage::Heartbeat {
            coordinate: crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
                .expect("test: valid coord"),
            timestamp: 12345,
        };

        let resp = dispatcher.dispatch(msg, "peer", zero_position());
        assert!(matches!(resp, DispatchResponse::None));
    }

    // ------------------------------------------------------------------
    // Gap 4 tests: SyncObserver notification
    // ------------------------------------------------------------------

    #[test]
    fn test_sync_observer_notified_on_completion() {
        use crate::blockchain::sync_manager::SyncObserver;
        use std::sync::{Arc, Mutex};

        struct TestObserver {
            events: Arc<Mutex<Vec<(String, u64)>>>,
        }

        impl SyncObserver for TestObserver {
            fn on_sync_complete(&self, network_id: &str, block_height: u64) {
                self.events
                    .lock()
                    .expect("test: lock")
                    .push((network_id.to_string(), block_height));
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let observer = TestObserver {
            events: events.clone(),
        };

        let mut sm = make_sync_manager();
        sm.set_observer(Box::new(observer));
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Process an empty SyncResponse (triggers Synchronized)
        sm.process_sync_message(SyncMessage::Response {
            network_id: "net-1".to_string(),
            block_hashes: Vec::new(),
            peer_height: 99,
        });

        let captured = events.lock().expect("test: lock");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, "net-1");
        assert_eq!(captured[0].1, 99);
    }

    // ------------------------------------------------------------------
    // Gap 1 tests: SyncManager uses BlockProvider
    // ------------------------------------------------------------------

    #[test]
    fn test_sync_manager_with_block_provider() {
        let mut sm = make_sync_manager();
        sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let provider = FakeBlockProvider { chain_height: 50 };

        let request = SyncMessage::Request {
            network_id: "net-1".to_string(),
            from_height: 10,
            max_blocks: 20,
        };

        let response = sm.process_sync_message_with_provider(request, Some(&provider));
        match response {
            Some(SyncMessage::Response {
                block_hashes,
                peer_height,
                ..
            }) => {
                assert_eq!(block_hashes.len(), 20);
                assert_eq!(block_hashes[0], "hash_10");
                assert_eq!(block_hashes[19], "hash_29");
                assert_eq!(peer_height, 50);
            }
            other => unreachable!("test: expected Response, got {:?}", other),
        }
    }
}

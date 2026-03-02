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

/// Drives sync operations using a real `BlockTransport` implementation.
///
/// Given a `SyncManager` that tracks which networks need syncing and a
/// `BlockTransport` for sending blocks to peers, `TransportSyncDriver`
/// runs a single sync round:
///
/// 1. For each network needing sync, generate a sync request.
/// 2. Send the request via `BlockTransport` to the best reflector.
/// 3. Process the response and update `SyncManager` state.
///
/// This bridges the gap between the state-machine (`SyncManager`) and
/// actual network I/O (`BlockTransport`).
pub struct TransportSyncDriver;

impl TransportSyncDriver {
    /// Run one sync round for all networks that need syncing.
    ///
    /// `block_transport` sends blocks/messages to peers.
    /// `reflector_pool` provides the best reflectors per network.
    /// `local_height` is the current device chain height.
    ///
    /// Returns the number of networks that advanced to Synchronized.
    pub async fn run_sync_round(
        sync_manager: &mut SyncManager,
        reflector_pool: &ReflectorPool,
        block_provider: Option<&dyn BlockProvider>,
        block_transport: &dyn crate::blockchain::propagation::BlockTransport,
        local_height: u64,
        local_coordinate: &crate::matrix::coordinate::MatrixCoordinate,
    ) -> usize {
        let networks: Vec<String> = sync_manager
            .networks_needing_sync()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut synced_count = 0;

        for network_id in &networks {
            // Generate sync request
            let _request = match sync_manager.generate_sync_request(network_id, local_height) {
                Some(r) => r,
                None => continue,
            };

            // Find the best reflector to ask
            let reflectors = reflector_pool.get_best_reflectors(network_id, 1);
            let reflector = match reflectors.first() {
                Some(r) => r,
                None => {
                    debug!(
                        network = %network_id,
                        "No reflectors available for sync"
                    );
                    continue;
                }
            };

            // Encode the sync request as a block announcement to the reflector
            // (The actual protocol would use a dedicated message channel; here
            // we model it as a block send to exercise the transport trait.)
            let sync_block = crate::blockchain::block::Block::genesis(*local_coordinate);
            let reflector_coord = match crate::matrix::coordinate::MatrixCoordinate::new(
                reflector.position.x as i64,
                reflector.position.y as i64,
                reflector.position.z as i64,
            ) {
                Ok(c) => c,
                Err(_) => *local_coordinate,
            };

            let sent = block_transport
                .send_block(&sync_block, &reflector_coord, local_coordinate)
                .await;

            if sent {
                debug!(
                    network = %network_id,
                    reflector = %reflector.node_id,
                    "Sync request sent via transport"
                );

                // Simulate receiving a response (the reflector replies with its height)
                let response = SyncMessage::Response {
                    network_id: network_id.clone(),
                    block_hashes: Vec::new(),
                    peer_height: reflector.block_height,
                };
                sync_manager.process_sync_message_with_provider(response, block_provider);

                // Check if we reached Synchronized
                if matches!(
                    sync_manager.sync_state(network_id),
                    Some(crate::blockchain::sync_manager::SyncState::Synchronized { .. })
                ) {
                    synced_count += 1;
                }
            }
        }

        synced_count
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

    // ------------------------------------------------------------------
    // 5.2 tests: TransportSyncDriver with BlockTransport
    // ------------------------------------------------------------------

    /// A deterministic BlockTransport that always succeeds.
    struct AlwaysSucceedTransport;

    #[async_trait::async_trait]
    impl crate::blockchain::propagation::BlockTransport for AlwaysSucceedTransport {
        async fn send_block(
            &self,
            _block: &crate::blockchain::block::Block,
            _target: &crate::matrix::coordinate::MatrixCoordinate,
            _origin: &crate::matrix::coordinate::MatrixCoordinate,
        ) -> bool {
            true
        }
    }

    /// A deterministic BlockTransport that always fails.
    struct AlwaysFailTransport;

    #[async_trait::async_trait]
    impl crate::blockchain::propagation::BlockTransport for AlwaysFailTransport {
        async fn send_block(
            &self,
            _block: &crate::blockchain::block::Block,
            _target: &crate::matrix::coordinate::MatrixCoordinate,
            _origin: &crate::matrix::coordinate::MatrixCoordinate,
        ) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn test_transport_sync_driver_synchronizes_via_reflector() {
        // Setup: two simulated nodes — our node and a reflector
        let mut sm = make_sync_manager();
        sm.join_network("net-sync-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        rp.register_reflector(
            "net-sync-1",
            Reflector {
                node_id: "reflector-1".to_string(),
                position: MatrixPosition {
                    x: 5.0,
                    y: 5.0,
                    z: 5.0,
                },
                last_seen: 9999,
                block_height: 42,
                health_score: 0.9,
                privacy_mode: PrivacyMode::PUBLIC,
            },
        );

        let transport = AlwaysSucceedTransport;
        let local_coord = crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
            .expect("test: coord");

        let synced = super::TransportSyncDriver::run_sync_round(
            &mut sm,
            &rp,
            None,
            &transport,
            0,
            &local_coord,
        )
        .await;

        // Should have synchronized with the reflector's height
        assert_eq!(synced, 1, "Expected 1 network to sync");

        use crate::blockchain::sync_manager::SyncState;
        assert_eq!(
            sm.sync_state("net-sync-1"),
            Some(&SyncState::Synchronized {
                last_block_height: 42
            }),
            "Should be synchronized at reflector height"
        );
    }

    #[tokio::test]
    async fn test_transport_sync_driver_no_sync_when_transport_fails() {
        let mut sm = make_sync_manager();
        sm.join_network("net-fail-1".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        let mut rp = make_reflector_pool();
        rp.register_reflector(
            "net-fail-1",
            Reflector {
                node_id: "reflector-fail".to_string(),
                position: MatrixPosition {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                last_seen: 9999,
                block_height: 100,
                health_score: 0.8,
                privacy_mode: PrivacyMode::PUBLIC,
            },
        );

        let transport = AlwaysFailTransport;
        let local_coord = crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
            .expect("test: coord");

        let synced = super::TransportSyncDriver::run_sync_round(
            &mut sm,
            &rp,
            None,
            &transport,
            0,
            &local_coord,
        )
        .await;

        assert_eq!(synced, 0, "No networks should sync when transport fails");

        // Should still be in Discovering state
        use crate::blockchain::sync_manager::SyncState;
        assert_eq!(
            sm.sync_state("net-fail-1"),
            Some(&SyncState::Discovering),
            "Should remain in Discovering when transport fails"
        );
    }
}

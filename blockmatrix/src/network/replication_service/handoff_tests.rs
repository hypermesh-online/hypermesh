// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Component tests for the Catalog->NGauge DMS handoff (split from handoff.rs
//! to keep that file within the <=500-line bar). Included via `#[path]`.

    use super::*;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;
    use hypermesh_lib::DEFAULT_NETWORK;
    use ngauge::{DmsDriver, ShardCandidates};

    /// Phase-4 DMS handoff, daemon/component level, with ZERO catalog import and
    /// NO manual seeding:
    ///
    /// 1. The node HOSTS a version through the real store/host path (the `store`
    ///    IPC entrypoint), which shards + stores locally and records the shard
    ///    set on-chain as `StoragePointer::Sharded`.
    /// 2. The daemon-wired [`ChainHeadObserver::poll_once_from_chain`] reads the
    ///    new asset head from `&NodeBlockchain` ALONE and seeds its on-chain shard
    ///    set into the live swarm handles.
    /// 3. Assert the shard set is registered in the live [`ShardLocationIndex`]
    ///    AND [`DmsDriver::plan`] produces a mirror for it.
    ///
    /// In-process/component (not 2-node QUIC): the handoff seam is entirely local
    /// state (chain read → index/analytics writes → plan), so a component test
    /// exercises the exact daemon code path deterministically without transport
    /// flakiness.
    #[tokio::test]
    async fn daemon_observer_mirrors_a_hosted_version_from_chain() {
        let state = crate::ipc::handlers::tests::test_state().await;

        // (1) Host a version through the node store/host path (IPC `store`).
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let file_path = tmp.path().join("version.bin");
        let data = b"HYPERMESH-DMS-PHASE4-HANDOFF-".repeat(500);
        std::fs::write(&file_path, &data).expect("test: write payload");

        let mut handler = RequestHandler::new();
        crate::ipc::handlers::store::register(&mut handler, &state);
        let resp = handler
            .dispatch(RpcRequest::new(
                "store",
                serde_json::json!({ "path": file_path.to_string_lossy() }),
            ))
            .await;
        assert!(resp.error.is_none(), "store must succeed: {:?}", resp.error);
        let result = resp.result.expect("test: store result present");
        let shard_hashes: Vec<[u8; 32]> = result["shard_hashes"]
            .as_array()
            .expect("test: shard_hashes array")
            .iter()
            .map(|h| {
                let hex = h.as_str().expect("test: shard hash is hex");
                <[u8; 32]>::try_from(hex::decode(hex).expect("test: decode hex"))
                    .expect("test: 32-byte hash")
            })
            .collect();
        assert!(!shard_hashes.is_empty(), "asset must produce shards");

        // Build the daemon's swarm handles (test_state carries none of these).
        let network = DEFAULT_NETWORK;
        let index = Arc::new(ShardLocationIndex::new());
        let analytics = Arc::new(Mutex::new(SwarmAnalytics::new()));
        let consumer_provider = Arc::new(ConsumerProviderManager::new(
            state.shard_store.clone(),
            index.clone(),
            state.node_id.clone(),
            network,
        ));
        let seeder = SwarmSeeder::new(
            consumer_provider,
            index.clone(),
            analytics.clone(),
            state.shard_store.clone(),
            state.coordinate,
            node_id_from_hex(&state.node_id),
        );
        let observer = ChainHeadObserver::new();

        // (2) The EXACT daemon-wired path: read heads from &NodeBlockchain only,
        // seed the on-chain shard set. No manual seeding, no catalog.
        let asset_hash = *blake3::hash(&data).as_bytes();
        let seeded = observer
            .poll_once_from_chain(&state.blockchain, &seeder, &[network])
            .await;
        assert!(
            seeded.iter().any(|a| *a == asset_hash),
            "the hosted version's asset head must be seeded from the chain",
        );

        // (3a) The shard set is registered in the LIVE ShardLocationIndex.
        let provider_id = node_id_from_hex(&state.node_id).to_hex();
        for shard in &shard_hashes {
            let providers = index
                .get_providers_in_network(network, &ContentHash(*shard))
                .await;
            assert!(
                providers.contains(&provider_id),
                "shard {} must register this node as provider",
                hex::encode(&shard[..4]),
            );
        }

        // (3b) DmsDriver::plan mirrors the seeded set. Gather candidates from the
        // SAME live index (no hand-seeding), then plan under the analytics guard.
        let mut bundles = Vec::new();
        for shard in &shard_hashes {
            let providers = index
                .get_providers_in_network(network, &ContentHash(*shard))
                .await;
            bundles.push(ShardCandidates {
                shard_id: ContentHash(*shard),
                positioned: Vec::new(),
                all_ids: providers,
            });
        }
        let plan = {
            let guard = analytics.lock().expect("test: analytics lock");
            DmsDriver::plan(&guard, network, &bundles, 0.5)
        };
        assert!(
            !plan.mirror.is_empty(),
            "DmsDriver must mirror the under-replicated on-chain shard set",
        );
    }

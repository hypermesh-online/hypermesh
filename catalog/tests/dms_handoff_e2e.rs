// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase 4 DMS handoff — end-to-end, in-process.
//!
//! Proves the wiring the roadmap asks for: when Catalog registers a version on
//! the asset's own chain, blockmatrix OBSERVES the new head and SEEDS the
//! version's shard set into the SAME swarm the replication loop drives, so
//! NGauge mirrors it and the serve path reflects it. Every component is REAL —
//! a real signing `NodeBlockchain`, real `CatalogRegistry`, real
//! `ShardLocationIndex` / `ShardStore` / `ConsumerProviderManager`, the real RS
//! `Sharder`, the real `DmsDriver` planner, and the real `StoqDmsExecutor`.
//!
//! IN-PROCESS boundary (stated honestly): the ONLY substituted piece is the
//! shard byte-transfer for the mirror fetch. A true mirror pull is a 2-node QUIC
//! round-trip; introducing that here would be a flaky harness. Instead the
//! executor is fed an in-memory `MockShardTransport` (which implements the same
//! `ShardTransport` trait the STOQ transport does) pre-loaded with the source
//! node's bytes. The DECISION (DmsDriver plan), the REGISTER (provider index),
//! the replica-count FEEDBACK, and the REFLECT (index resolution) are all the
//! real production code — only the wire read is local.

use std::sync::{Arc, Mutex};

use blockmatrix::blockchain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::consumer_provider::ConsumerProviderManager;
use blockmatrix::network::replication_service::{
    shard_content, ChainHeadObserver, StoqDmsExecutor, SwarmSeeder, VersionContent,
    VersionContentSource,
};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::MockShardTransport;
use blockmatrix::network::swarm_provider::ShardLocationIndex;

use catalog::registry::asset_type::AssetTypeDefinition;
use catalog::registry::catalog_registry::{CatalogRegistry, RegistryConfig, TrustPolicy};

use blockmatrix::proof_of_state::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof,
};
use blockmatrix::assets::StateProof;

use async_trait::async_trait;
use hypermesh_lib::{ContentHash, NetworkId, NodeId, PrivacyMode, DEFAULT_NETWORK};
use ngauge::{DmsDriver, MirrorExecutor, ShardCandidates, SwarmAnalytics};
use serde_json::json;
use std::time::Duration;

/// The publisher/seed node id — registered as the first provider by the seed.
const SEED_NODE: &str = "node-A-seed";
/// The pool-mirror node id — the executor that pulls a second replica.
const MIRROR_NODE: &str = "node-B-mirror";

/// Structurally-valid author proof (PoStake = WHO, never a magnitude).
fn author_proof(who: &str) -> StateProof {
    StateProof::new(
        StakeProof::new(who.to_string(), format!("{who}-id")),
        TimeProof::new(Duration::from_secs(5)),
        SpaceProof::new(who.to_string(), format!("/{who}"), 1024),
        WorkProof::from_work(who.to_string(), "register".to_string(), b"work"),
    )
}

fn type_def(name: &str, version: &str) -> AssetTypeDefinition {
    let schema = json!({ "type": "object", "id": name, "version": version });
    let mut td = AssetTypeDefinition::new(name.to_string(), schema, author_proof(name));
    td.metadata.version = version.to_string();
    td
}

/// Test content source: returns the version's distributable bytes for the one
/// asset under test. In the daemon this is the catalog-backed adapter; here it
/// stands in for "the producer holds the authored content".
struct FixedContentSource {
    asset_hash: [u8; 32],
    network: NetworkId,
    bytes: Vec<u8>,
}

#[async_trait]
impl VersionContentSource for FixedContentSource {
    async fn content_for(&self, asset_hash: &[u8; 32]) -> Option<VersionContent> {
        if *asset_hash == self.asset_hash {
            Some(VersionContent {
                network: self.network,
                bytes: self.bytes.clone(),
            })
        } else {
            None
        }
    }
}

/// The full handoff: catalog write → observer seed → NGauge mirror → reflect.
#[tokio::test]
async fn dms_handoff_new_version_mirrors_and_reflects() {
    let network = DEFAULT_NETWORK;
    let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");

    // --- REAL core components ------------------------------------------------
    let chain = Arc::new(NodeBlockchain::new(coord));
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    )
    .with_chain(chain.clone());

    let index = Arc::new(ShardLocationIndex::new());
    let store = Arc::new(ShardStore::new());
    let consumer_provider = Arc::new(ConsumerProviderManager::new(
        store.clone(),
        index.clone(),
        SEED_NODE.to_string(),
        network,
    ));
    let analytics = Arc::new(Mutex::new(SwarmAnalytics::new()));
    let seeder = SwarmSeeder::new(
        consumer_provider,
        index.clone(),
        analytics.clone(),
        coord,
        NodeId::from_public_key(SEED_NODE.as_bytes()),
    );

    // --- Phase 3 write: register a Catalog asset version ---------------------
    registry
        .register_type(type_def("dataset", "1.0.0"))
        .await
        .expect("test: register dataset v1 on-chain");
    let asset_hash = registry
        .asset_hash_for_name("dataset")
        .await
        .expect("test: dataset resolves to a genesis asset_hash");

    // The version's distributable content (what the producer holds).
    let content = b"dataset-v1-distributable-payload-bytes-for-the-swarm".to_vec();
    let source = FixedContentSource {
        asset_hash,
        network,
        bytes: content.clone(),
    };

    // --- Observer: new asset head → seed the swarm ---------------------------
    let observer = ChainHeadObserver::new();
    let seeded = observer.poll_once(&chain, &seeder, &source).await;
    assert!(
        seeded.contains(&asset_hash),
        "observer seeds the newly-headed version into the swarm"
    );

    // A second pass is a no-op — the head has not advanced.
    let again = observer.poll_once(&chain, &seeder, &source).await;
    assert!(again.is_empty(), "no re-seed when the head is unchanged");

    // --- SEED assertion: shards under the asset's (network, content) key ------
    let shards = shard_content(&content).expect("test: reshard content to learn shard ids");
    assert!(!shards.is_empty(), "content shards into >= 1 shard");
    for (shard_id, _) in &shards {
        let providers = index.get_providers_in_network(network, shard_id).await;
        assert!(
            providers.contains(&SEED_NODE.to_string()),
            "seed registered the publisher as a provider under (network, shard)"
        );
        assert!(store.has(shard_id).await, "shard bytes are stored locally");
    }

    // --- MIRROR half: NGauge plans, the executor pulls a second replica -------
    // Model a pool peer (SEED_NODE) that already holds the bytes; MIRROR_NODE
    // pulls from it. Only this byte-transfer is the in-memory stand-in.
    let mock = Arc::new(MockShardTransport::new());
    let seed_node_id = NodeId::from_public_key(SEED_NODE.as_bytes());
    for (shard_id, data) in &shards {
        mock.insert_shard(&seed_node_id, shard_id, data.clone()).await;
    }
    let executor = StoqDmsExecutor::new(
        mock.clone(),
        index.clone(),
        MIRROR_NODE.to_string(),
        analytics.clone(),
    );

    // Gather candidates from the REAL index and let the REAL planner decide.
    let bundles = candidate_bundles(&index, network, &shards).await;
    let plan = {
        let guard = analytics.lock().expect("test: analytics guard");
        let plan = DmsDriver::plan(&guard, network, &bundles, 0.5);
        drop(guard); // !Send guard dropped before any await (feed.rs:28 rule).
        plan
    };
    assert!(
        !plan.mirror.is_empty(),
        "NGauge produces a mirror plan for the under-replicated version"
    );

    let action = &plan.mirror[0];
    let before = index
        .get_providers_in_network(network, &action.shard_id)
        .await
        .len();
    let replica_count = executor
        .fetch_and_register(action)
        .await
        .expect("test: mirror fetch + register succeeds");
    assert!(
        replica_count as usize > before,
        "mirror increased the replica count ({before} -> {replica_count})"
    );
    assert!(
        index
            .get_providers_in_network(network, &action.shard_id)
            .await
            .contains(&MIRROR_NODE.to_string()),
        "the mirroring node is now a registered provider (consumer-becomes-provider)"
    );

    // --- REFLECT half: a consumer request resolves the shard to a provider ----
    let resolved = index
        .get_providers_in_network(network, &action.shard_id)
        .await;
    assert!(
        !resolved.is_empty(),
        "the announced/served shard resolves for a consumer via the location index"
    );
}

/// Build the per-shard candidate bundles the planner consumes: every provider
/// the location index knows for a shard EXCEPT the mirroring node (a node cannot
/// mirror from itself) — the exact filter the poll loop applies.
async fn candidate_bundles(
    index: &ShardLocationIndex,
    network: NetworkId,
    shards: &[(ContentHash, Vec<u8>)],
) -> Vec<ShardCandidates> {
    let mut bundles = Vec::new();
    for (shard_id, _) in shards {
        let all_ids: Vec<String> = index
            .get_providers_in_network(network, shard_id)
            .await
            .into_iter()
            .filter(|id| id != MIRROR_NODE)
            .collect();
        if all_ids.is_empty() {
            continue;
        }
        bundles.push(ShardCandidates {
            shard_id: *shard_id,
            positioned: Vec::new(),
            all_ids,
        });
    }
    bundles
}

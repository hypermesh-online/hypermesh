//! Component seam only: keyed mock transport is not network/auth proof.
use std::sync::{Arc, Mutex};

use blockmatrix::network::{
    replication_service::StoqDmsExecutor,
    shard_store::ShardStore,
    shard_transport::{handle_shard_message, parse_shard_response, MockShardTransport},
    swarm_provider::ShardLocationIndex,
};
use hypermesh_lib::{ContentHash, MatrixPosition, NodeId, DEFAULT_NETWORK};
use ngauge::{DmsDriver, MirrorAction, MirrorExecutor, ShardCandidates, SwarmAnalytics};

struct Harness {
    remote: NodeId,
    local: String,
    shard: ContentHash,
    data: Vec<u8>,
    transport: Arc<MockShardTransport>,
    index: Arc<ShardLocationIndex>,
    store: Arc<ShardStore>,
    analytics: Arc<Mutex<SwarmAnalytics>>,
    executor: StoqDmsExecutor,
}

impl Harness {
    fn new() -> Self {
        let remote = NodeId::from_bytes([0xab; 32]);
        let local = NodeId::from_bytes([0xcd; 32]).to_hex();
        let data = b"canonical-index-planner-transport".to_vec();
        let shard = ContentHash(*blake3::hash(&data).as_bytes());
        let transport = Arc::new(MockShardTransport::new());
        let index = Arc::new(ShardLocationIndex::new());
        let store = Arc::new(ShardStore::new());
        let mut demand = SwarmAnalytics::new();
        demand.record_request_in_network(
            DEFAULT_NETWORK,
            shard,
            remote,
            MatrixPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            0,
        );
        demand.set_replica_count_in_network(DEFAULT_NETWORK, shard, 0);
        let analytics = Arc::new(Mutex::new(demand));
        let executor = StoqDmsExecutor::new(
            transport.clone(),
            store.clone(),
            index.clone(),
            local.clone(),
            analytics.clone(),
        );
        Self {
            remote,
            local,
            shard,
            data,
            transport,
            index,
            store,
            analytics,
            executor,
        }
    }

    async fn action(&self, provider: NodeId) -> MirrorAction {
        self.index
            .register_provider_in_network(DEFAULT_NETWORK, &provider.to_hex(), &[self.shard])
            .await;
        let ids = self
            .index
            .get_providers_in_network(DEFAULT_NETWORK, &self.shard)
            .await;
        let guard = self.analytics.lock().expect("analytics");
        let plan = DmsDriver::plan(
            &guard,
            DEFAULT_NETWORK,
            &[ShardCandidates {
                shard_id: self.shard,
                positioned: Vec::new(),
                all_ids: ids,
            }],
            0.5,
        );
        assert_eq!(plan.mirror.len(), 1);
        assert_eq!(plan.mirror[0].source, provider);
        plan.mirror[0].clone()
    }

    async fn assert_not_advertised(&self) {
        let providers = self
            .index
            .get_providers_in_network(DEFAULT_NETWORK, &self.shard)
            .await;
        assert!(!providers.contains(&self.local));
        assert_eq!(providers.len(), 1, "only the original remote hint remains");
        assert_eq!(
            self.analytics
                .lock()
                .expect("analytics")
                .get_replica_count(&self.shard),
            0
        );
    }
}

#[tokio::test]
async fn index_identity_survives_planner_and_keyed_fetch_then_serves() {
    let h = Harness::new();
    h.transport
        .insert_shard(&h.remote, &h.shard, h.data.clone())
        .await;
    let action = h.action(h.remote).await;
    assert_eq!(
        h.executor
            .fetch_and_register(&action)
            .await
            .expect("fetch and register"),
        2
    );
    let providers = h
        .index
        .get_providers_in_network(DEFAULT_NETWORK, &h.shard)
        .await;
    assert!(providers.contains(&h.local));
    assert_eq!(
        h.analytics
            .lock()
            .expect("analytics")
            .get_replica_count(&h.shard),
        2
    );
    let mut request = vec![0x02];
    request.extend_from_slice(&h.shard.0);
    let response = handle_shard_message(&request, &h.store, None)
        .await
        .expect("serve handler")
        .expect("response");
    assert_eq!(parse_shard_response(&response, &h.shard).0, h.data);
}

#[tokio::test]
async fn unknown_valid_identity_cannot_fetch_or_advertise() {
    let h = Harness::new();
    h.transport
        .insert_shard(&h.remote, &h.shard, h.data.clone())
        .await;
    let action = h.action(NodeId::from_bytes([0x11; 32])).await;
    assert!(h.executor.fetch_and_register(&action).await.is_err());
    h.assert_not_advertised().await;
    assert!(h.store.get(&h.shard).await.is_none());
}

#[tokio::test]
async fn empty_fetch_cannot_advertise() {
    let h = Harness::new();
    h.transport
        .insert_shard(&h.remote, &h.shard, Vec::new())
        .await;
    let action = h.action(h.remote).await;
    assert!(h.executor.fetch_and_register(&action).await.is_err());
    h.assert_not_advertised().await;
    assert!(h.store.get(&h.shard).await.is_none());
}

#[tokio::test]
async fn corrupt_fetch_cannot_advertise() {
    let h = Harness::new();
    h.transport
        .insert_shard(&h.remote, &h.shard, b"corrupt".to_vec())
        .await;
    let action = h.action(h.remote).await;
    assert!(h.executor.fetch_and_register(&action).await.is_err());
    h.assert_not_advertised().await;
    assert!(h.store.get(&h.shard).await.is_none());
}

#[tokio::test]
async fn valid_fetch_with_conflicting_unservable_store_cannot_advertise() {
    let h = Harness::new();
    h.transport
        .insert_shard(&h.remote, &h.shard, h.data.clone())
        .await;
    h.store
        .store(h.shard, b"conflicting-local-bytes".to_vec())
        .await;
    let action = h.action(h.remote).await;
    assert!(h.executor.fetch_and_register(&action).await.is_err());
    h.assert_not_advertised().await;
    assert_eq!(
        h.store.get(&h.shard).await,
        Some(b"conflicting-local-bytes".to_vec())
    );
}

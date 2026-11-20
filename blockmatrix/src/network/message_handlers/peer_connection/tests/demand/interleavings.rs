use super::super::super::fetch::{authorize_shard_fetch, commit_authorized_fetch_demand};
use super::*;

#[tokio::test]
async fn replacement_during_async_asset_authorization_records_nothing() {
    let fixture = Arc::new(Fixture::new());
    fixture.allow_shard().await;
    fixture.register("A").await;
    let guard = fixture.chain.asset_index.write().await;
    let f = fixture.clone();
    let fetch = tokio::spawn(async move {
        prepare_shard_fetch(&request(), "same-node", "A", &f.context()).await
    });
    tokio::task::yield_now().await;
    fixture.register("B").await;
    drop(guard);
    assert!(!fetch.await.unwrap());
    assert!(fixture.tracker.snapshot().await.is_empty());
    assert!(prepare_shard_fetch(&request(), "same-node", "B", &fixture.context()).await);
    fixture.assert_one().await;
}

#[tokio::test]
async fn replacement_after_authorization_before_commit_records_nothing() {
    let fixture = Fixture::new();
    fixture.allow_shard().await;
    fixture.register("A").await;
    assert!(authorize_shard_fetch(&[7; 32], "same-node", "A", &fixture.context()).await);
    fixture.register("B").await;
    assert!(!commit_authorized_fetch_demand([7; 32], "same-node", "A", &fixture.context()).await);
    assert!(fixture.tracker.snapshot().await.is_empty());
    assert!(prepare_shard_fetch(&request(), "same-node", "B", &fixture.context()).await);
    fixture.assert_one().await;
}

#[tokio::test]
async fn successful_commit_before_replacement_keeps_historical_network() {
    let fixture = Fixture::new();
    fixture.allow_shard().await;
    fixture.register("A").await;
    assert!(prepare_shard_fetch(&request(), "same-node", "A", &fixture.context()).await);
    let mut replacement = peer("B");
    replacement.network_id = hypermesh_lib::NetworkId::from_wire_str("other");
    assert!(peer_auth::register_authenticated_peer(&fixture.peers, replacement).await);
    assert!(!prepare_shard_fetch(&request(), "same-node", "A", &fixture.context()).await);
    fixture.assert_one().await;
    assert!(fixture
        .tracker
        .snapshot_in_network(hypermesh_lib::NetworkId::from_wire_str("other"))
        .await
        .is_empty());
}

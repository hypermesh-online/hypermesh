use super::super::fetch::{prepare_shard_fetch, FetchContext};
use super::super::*;
use super::fetch_fixture::{coordinate, peer, request};

#[tokio::test]
async fn asset_denied_fetch_leaves_demand_unchanged() {
    let peers = peer_auth::new_authenticated_peers();
    let tracker = crate::network::SwarmDemandTracker::new();
    let chain = crate::blockchain::node_chain::NodeBlockchain::new(coordinate());
    assert!(peer_auth::register_authenticated_peer(&peers, peer("A")).await);
    assert!(peer_auth::verify_peer_access(&peers, "same-node", "A", "net-1").await);
    assert!(peer_auth::verify_shard_proof_binding(&peers, "same-node", "A", "net-1").await);
    assert!(!chain.authorizes_shard(&[7; 32]).await);
    let ctx = FetchContext {
        authenticated_peers: &peers,
        network_id: "net-1",
        blockchain: &chain,
        swarm_demand_tracker: &tracker,
    };
    assert!(!prepare_shard_fetch(&request(), "same-node", "A", &ctx).await);
    assert!(
        tracker.snapshot().await.is_empty(),
        "asset-denied fetch must not mutate demand"
    );
}

use super::fetch_fixture::Fixture;

#[tokio::test]
async fn valid_current_fetch_records_exactly_one() {
    let fixture = Fixture::new();
    fixture.allow_shard().await;
    fixture.register("A").await;
    assert!(prepare_shard_fetch(&request(), "same-node", "A", &fixture.context()).await);
    fixture.assert_one().await;
}

#[derive(Clone, Copy)]
enum InvalidPeer {
    Missing,
    EmptyConnection,
    NoProof,
    WrongSession,
    WrongNetwork,
    Expired,
}

fn invalid_peer(case: InvalidPeer) -> Option<peer_auth::AuthenticatedPeer> {
    let mut entry = peer("A");
    match case {
        InvalidPeer::Missing => return None,
        InvalidPeer::EmptyConnection => entry.connection_id.clear(),
        InvalidPeer::NoProof => entry.proof_bytes.clear(),
        InvalidPeer::WrongSession => entry.connection_id = "B".into(),
        InvalidPeer::WrongNetwork => {
            entry.network_id = hypermesh_lib::NetworkId::from_wire_str("other")
        }
        InvalidPeer::Expired => entry.authenticated_at -= std::time::Duration::from_secs(7200),
    }
    Some(entry)
}

#[tokio::test]
async fn invalid_or_ineligible_fetches_have_no_demand_side_effect() {
    let cases = [
        InvalidPeer::Missing,
        InvalidPeer::EmptyConnection,
        InvalidPeer::NoProof,
        InvalidPeer::WrongSession,
        InvalidPeer::WrongNetwork,
        InvalidPeer::Expired,
    ];
    for case in cases {
        let fixture = Fixture::new();
        fixture.allow_shard().await;
        if let Some(entry) = invalid_peer(case) {
            fixture
                .peers
                .write()
                .await
                .insert("same-node".into(), entry);
        }
        assert!(!prepare_shard_fetch(&request(), "same-node", "A", &fixture.context()).await);
        assert!(fixture.tracker.snapshot().await.is_empty());
    }
}

#[tokio::test]
async fn malformed_or_empty_session_fetches_have_no_demand_side_effect() {
    let fixture = Fixture::new();
    fixture.allow_shard().await;
    fixture.register("A").await;
    for data in [
        vec![],
        vec![TAG_SHARD_FETCH],
        vec![TAG_SHARD_FETCH; 32],
        vec![0; 33],
    ] {
        assert!(!prepare_shard_fetch(&data, "same-node", "A", &fixture.context()).await);
    }
    assert!(!prepare_shard_fetch(&request(), "same-node", "", &fixture.context()).await);
    assert!(fixture.tracker.snapshot().await.is_empty());
}

mod interleavings;

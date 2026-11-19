use super::super::*;

pub(super) fn peer(connection_id: &str) -> peer_auth::AuthenticatedPeer {
    peer_auth::AuthenticatedPeer {
        node_id: "same-node".into(),
        connection_id: connection_id.into(),
        pubkey: vec![1],
        coordinate: (2, 2, 2),
        network_id: hypermesh_lib::NetworkId::from_wire_str("net-1"),
        authenticated_at: std::time::Instant::now(),
        proof_bytes: vec![2],
    }
}

pub(super) fn coordinate() -> MatrixCoordinate {
    MatrixCoordinate::new(2, 2, 2).unwrap()
}

pub(super) fn request() -> Vec<u8> {
    let mut data = vec![TAG_SHARD_FETCH];
    data.extend_from_slice(&[7; 32]);
    data
}

pub(super) struct Fixture {
    pub peers: AuthenticatedPeers,
    pub tracker: crate::network::SwarmDemandTracker,
    pub chain: crate::blockchain::node_chain::NodeBlockchain,
}
impl Fixture {
    pub fn new() -> Self {
        Self {
            peers: peer_auth::new_authenticated_peers(),
            tracker: crate::network::SwarmDemandTracker::new(),
            chain: crate::blockchain::node_chain::NodeBlockchain::new(coordinate()),
        }
    }
    pub fn context(&self) -> super::super::fetch::FetchContext<'_> {
        super::super::fetch::FetchContext {
            authenticated_peers: &self.peers,
            network_id: "net-1",
            blockchain: &self.chain,
            swarm_demand_tracker: &self.tracker,
        }
    }
    pub async fn allow_shard(&self) {
        use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
        let entry = BlockAssetEntry::new_bound(
            [9; 32],
            &trustchain::proof_of_state::StateProof::new_for_testing(),
            StoragePointer::Sharded {
                shard_hashes: vec![[7; 32]],
                placements: vec![coordinate()],
            },
            crate::assets::core::AssetRegistration::genesis(coordinate()),
        );
        // Synthetic local fixture through the real add/index API, not remote
        // cryptographic admission or a physical-network proof.
        self.chain.add_block(vec![entry]).await.unwrap();
        assert!(self.chain.authorizes_shard(&[7; 32]).await);
    }
    pub async fn register(&self, id: &str) {
        assert!(peer_auth::register_authenticated_peer(&self.peers, peer(id)).await);
    }
    pub async fn assert_one(&self) {
        let entries = self
            .tracker
            .snapshot_in_network(hypermesh_lib::NetworkId::from_wire_str("net-1"))
            .await;
        let entry = &entries[&hypermesh_lib::ContentHash([7; 32])];
        assert_eq!(entry.request_count, 1);
        assert_eq!(entry.requester_ids.len(), 1);
        assert!(entry.requester_ids.contains("same-node"));
    }
}

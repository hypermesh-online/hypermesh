use super::*;
use crate::blockchain::block::Block;
use crate::blockchain::propagation::{BlockPropagator, BlockTransport, PropagationStrategy};

struct GuardProbe(ConnectedPeerRoutes);
#[async_trait::async_trait]
impl BlockTransport for GuardProbe {
    async fn send_block(
        &self,
        _block: &Block,
        _target: &MatrixCoordinate,
        _origin: &MatrixCoordinate,
    ) -> bool {
        assert!(
            self.0.try_write().is_ok(),
            "routing read guard must be released before transport IO"
        );
        true
    }
}

#[tokio::test]
async fn propagation_await_uses_detached_snapshot_without_routing_guard() {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish(&nodes, &routes, "one", "A").await;
    let targets = coordinate_snapshot(&routes).await;
    let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
    let propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::NearestN(6),
        Arc::new(GuardProbe(routes)),
    );
    let result = propagator
        .propagate_block(&Block::genesis(origin), &targets)
        .await;
    assert_eq!(result.reached_nodes, vec![coord()]);
}

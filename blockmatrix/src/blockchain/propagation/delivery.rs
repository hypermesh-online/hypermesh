use super::*;

impl BlockPropagator {
    pub(super) fn log_propagation_targets(&self, block: &Block, target_count: usize) {
        info!(
            "Propagating block {} from ({},{},{}) to {} targets",
            block.index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
            target_count
        );
    }

    pub(super) async fn propagate_to_target(
        &self,
        block: &Block,
        target: &MatrixCoordinate,
        reached_nodes: &mut Vec<MatrixCoordinate>,
        failed_nodes: &mut Vec<MatrixCoordinate>,
    ) {
        if self.should_propagate_to(&block.hash, target).await {
            if self
                .transport
                .send_block(block, target, &self.node_coordinate)
                .await
            {
                reached_nodes.push(*target);
                self.mark_block_seen(&block.hash, target).await;
            } else {
                failed_nodes.push(*target);
            }
        }
    }
}

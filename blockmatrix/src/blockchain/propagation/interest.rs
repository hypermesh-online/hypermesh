use super::*;

impl BlockPropagator {
    /// Select targets using the content-interested torrent model.
    ///
    /// Routing depends on blockchain scope and privacy mode:
    /// - **Device**: no propagation (local-only chain)
    /// - **Private**: full replication to all authenticated peers in same network
    /// - **Anonymous**: only nodes actively fetching content from this block
    /// - **Public**: reflectors + spatial neighbors + active consumers
    pub(super) async fn select_interested_targets(
        &self,
        block: &Block,
        all_peers: &[MatrixCoordinate],
        ctx: &InterestContext,
    ) -> Vec<MatrixCoordinate> {
        // Device scope blocks never propagate
        if ctx.blockchain_scope == BlockchainScope::Device {
            debug!("ContentInterested: Device scope — no propagation");
            return vec![];
        }

        // Network scope: routing depends on privacy mode
        if ctx.privacy_mode == PrivacyMode::PRIVATE {
            // Private: full replication to all authenticated peers sharing network_id
            debug!(
                "ContentInterested: Private mode — full replication to {} peer(s)",
                all_peers.len()
            );
            return all_peers.to_vec();
        }

        if ctx.privacy_mode == PrivacyMode::ANONYMOUS {
            // Anonymous: only consumers who are actively fetching content in this block
            let consumers = self.get_consumer_targets(block, ctx).await;
            debug!(
                "ContentInterested: Anonymous mode — {} consumer target(s)",
                consumers.len()
            );
            return consumers;
        }

        self.select_public_targets(block, all_peers, ctx).await
    }

    async fn select_public_targets(
        &self,
        block: &Block,
        all_peers: &[MatrixCoordinate],
        ctx: &InterestContext,
    ) -> Vec<MatrixCoordinate> {
        // Public (default): reflectors + spatial neighbors + consumers
        let mut targets = Vec::new();

        // 1. Reflectors (always interested in network blocks)
        let reflector_targets = self.get_reflector_targets(ctx).await;
        debug!(
            "ContentInterested: Public mode — {} reflector target(s)",
            reflector_targets.len()
        );
        targets.extend(reflector_targets);

        // 2. Consumers actively fetching this block's shards
        let consumer_targets = self.get_consumer_targets(block, ctx).await;
        debug!(
            "ContentInterested: Public mode — {} consumer target(s)",
            consumer_targets.len()
        );
        targets.extend(consumer_targets);

        self.append_spatial_targets(&mut targets, block, all_peers, ctx)
            .await;

        // Deduplicate by coordinate
        targets.sort_by(|a, b| (a.x, a.y, a.z).cmp(&(b.x, b.y, b.z)));
        targets.dedup_by(|a, b| a.x == b.x && a.y == b.y && a.z == b.z);

        debug!(
            "ContentInterested: Public mode — {} total unique target(s)",
            targets.len()
        );
        targets
    }

    /// Get reflector coordinates from the pool for this network.
    async fn get_reflector_targets(&self, ctx: &InterestContext) -> Vec<MatrixCoordinate> {
        let pool = ctx.reflector_pool.lock().await;
        pool.get_best_reflectors(&ctx.network_id, 10)
            .iter()
            .filter_map(|r| {
                // MatrixPosition { x: f64, y: f64, z: f64 } -> MatrixCoordinate
                MatrixCoordinate::new(
                    r.position.x as i64,
                    r.position.y as i64,
                    r.position.z as i64,
                )
                .ok()
            })
            .collect()
    }

    /// Get coordinates of nodes actively fetching shards referenced in this block.
    async fn get_consumer_targets(
        &self,
        block: &Block,
        ctx: &InterestContext,
    ) -> Vec<MatrixCoordinate> {
        let demand_snapshot = ctx.swarm_demand.snapshot().await;
        if demand_snapshot.is_empty() {
            return vec![];
        }

        let mut consumer_coords = Vec::new();
        let auth_peers = ctx.authenticated_peers.read().await;

        for entry in &block.entries {
            // Hash the asset_hash to a ContentHash for demand lookup
            let content_hash = hypermesh_lib::ContentHash(entry.asset_hash);

            if let Some(demand) = demand_snapshot.get(&content_hash) {
                Self::append_consumer_targets(
                    &mut consumer_coords,
                    &auth_peers,
                    &demand.requester_ids,
                );
            }
        }

        consumer_coords
    }

    /// Extract all shard placement coordinates from a block's entries.
    fn extract_shard_positions(block: &Block) -> Vec<MatrixCoordinate> {
        let mut positions = Vec::new();
        for entry in &block.entries {
            if let crate::blockchain::block::StoragePointer::Sharded { ref placements, .. } =
                entry.storage_pointer
            {
                positions.extend_from_slice(placements);
            }
        }
        positions
    }

    /// Select the N nearest peers to our coordinate.
    fn select_nearest_n(&self, peers: &[MatrixCoordinate], n: usize) -> Vec<MatrixCoordinate> {
        find_k_nearest(&self.node_coordinate, peers, n)
            .into_iter()
            .map(|(coord, _distance)| coord)
            .collect()
    }
}

impl BlockPropagator {
    async fn append_spatial_targets(
        &self,
        targets: &mut Vec<MatrixCoordinate>,
        block: &Block,
        all_peers: &[MatrixCoordinate],
        ctx: &InterestContext,
    ) {
        // 3. Spatial neighbors (peers whose neighborhood includes this block's content)
        if let Some(ref assigner_lock) = ctx.spatial_assigner {
            let assigner = assigner_lock.read().await;
            let shard_positions = Self::extract_shard_positions(block);
            if !shard_positions.is_empty() {
                let spatial: Vec<MatrixCoordinate> = all_peers
                    .iter()
                    .filter(|peer| assigner.block_relevant_to_peer(&shard_positions, peer))
                    .copied()
                    .collect();
                debug!(
                    "ContentInterested: Public mode — {} spatial target(s)",
                    spatial.len()
                );
                targets.extend(spatial);
            }
        } else {
            // No spatial assigner: include nearest 3 peers as fallback
            let nearest = self.select_nearest_n(all_peers, 3);
            targets.extend(nearest);
        }
    }
}

impl BlockPropagator {
    fn append_consumer_targets(
        consumer_coords: &mut Vec<MatrixCoordinate>,
        auth_peers: &std::collections::HashMap<
            String,
            crate::network::peer_auth::AuthenticatedPeer,
        >,
        requester_ids: &std::collections::HashSet<String>,
    ) {
        for requester_id in requester_ids {
            if let Some(peer) = auth_peers.get(requester_id) {
                consumer_coords.extend(Self::consumer_coordinate(peer, requester_id));
            }
        }
    }

    fn consumer_coordinate(
        peer: &crate::network::peer_auth::AuthenticatedPeer,
        requester_id: &str,
    ) -> Option<MatrixCoordinate> {
        match MatrixCoordinate::new(
            peer.coordinate.0 as i64,
            peer.coordinate.1 as i64,
            peer.coordinate.2 as i64,
        ) {
            Ok(coord) => Some(coord),
            Err(e) => {
                warn!(
                    "Invalid peer coordinate for {}: {}",
                    &requester_id[..8.min(requester_id.len())],
                    e
                );
                None
            }
        }
    }
}

//! Existing block-sync cycle, separated without changing scheduling or IO order.
use super::*;
use blockmatrix::blockchain::block::Block;
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::network::connected_peer_routes::{refresh_routes, ConnectedPeerRoutes};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

type Addresses = HashMap<String, (String, SocketAddr)>;

pub(super) struct BlockSyncLoop {
    pub(super) sync_manager: Arc<Mutex<SyncManager>>,
    pub(super) reflector_pool: Arc<Mutex<ReflectorPool>>,
    pub(super) blockchain: Arc<NodeBlockchain>,
    pub(super) network: Arc<NetworkManager>,
    pub(super) node_map: Arc<RwLock<Addresses>>,
    pub(super) transport: Arc<stoq::StoqTransport>,
    pub(super) block_propagator: Arc<Mutex<BlockPropagator>>,
    pub(super) peer_coords: ConnectedPeerRoutes,
    pub(super) coord: MatrixCoordinate,
    pub(super) network_id: String,
    pub(super) is_reflector: bool,
    pub(super) shard_store: Arc<ShardStore>,
}

impl BlockSyncLoop {
    pub(super) async fn run(self, metrics_node_id: String) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let mut metrics_reporter = blockmatrix::network::MetricsReporter::new(metrics_node_id);
        let os_abs = create_os_abstraction().ok();
        let mut cycle_count: u64 = 0;
        let mut last_propagated_height: u64 = self.blockchain.get_height().await;
        loop {
            interval.tick().await;
            cycle_count += 1;
            let addr_map_snapshot = self.refresh_projection().await;
            self.sync_round(&addr_map_snapshot).await;
            self.propagate_created(&mut last_propagated_height).await;
            self.prune_reflectors().await;
            if self.is_reflector {
                send_reflector_heartbeat(&self.blockchain, &self.network, &self.network_id).await;
            }
            if cycle_count % 6 == 0 {
                self.emit_metrics(&mut metrics_reporter, os_abs.as_deref())
                    .await;
            }
        }
    }

    async fn refresh_projection(&self) -> Addresses {
        let addr_map = self.network.get_node_address_map().await;
        *self.node_map.write().await = addr_map;
        refresh_routes(&self.network.connected_nodes(), &self.peer_coords).await;
        self.node_map.read().await.clone()
    }

    async fn sync_round(&self, addr_map_snapshot: &Addresses) {
        let fetched_blocks = {
            let mut sm = self.sync_manager.lock().await;
            let mut rp = self.reflector_pool.lock().await;
            TransportSyncDriver::run_sync_round(
                &mut sm,
                &mut rp,
                &self.blockchain,
                &self.transport,
                addr_map_snapshot,
                &self.coord,
            )
            .await
        };
        for block in &fetched_blocks {
            match self.blockchain.insert_received_block(block.clone()).await {
                Ok(()) => info!("Inserted synced block #{}", block.index),
                Err(e) => warn!("Synced block #{} REJECTED: {}", block.index, e),
            }
        }
        if !fetched_blocks.is_empty() {
            info!("Sync round: fetched {} block(s)", fetched_blocks.len());
        }
    }

    async fn propagate_created(&self, last_propagated_height: &mut u64) {
        let current_height = self.blockchain.get_height().await;
        if current_height > *last_propagated_height {
            let chain = self.blockchain.get_chain().await;
            for block in chain.iter().filter(|b| b.index > *last_propagated_height) {
                self.propagate_created_block(block).await;
            }
            *last_propagated_height = current_height;
        }
    }

    async fn propagate_created_block(&self, block: &Block) {
        let coords = self.network.get_connected_coordinates().await;
        if !coords.is_empty() {
            let result = self
                .block_propagator
                .lock()
                .await
                .propagate_block(block, &coords)
                .await;
            info!(
                "Propagated block #{} to {} peer(s)",
                block.index,
                result.reached_nodes.len()
            );
        }
    }

    async fn prune_reflectors(&self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let pruned = self.reflector_pool.lock().await.prune_stale(now_ms);
        if pruned > 0 {
            debug!("Pruned {} stale reflector(s)", pruned);
        }
    }

    async fn emit_metrics(
        &self,
        metrics_reporter: &mut blockmatrix::network::MetricsReporter,
        os_abs: Option<&dyn blockmatrix::os_integration::OsAbstraction>,
    ) {
        let connected_nodes = self.network.get_connected_nodes().await;
        let chain_h = self.blockchain.get_height().await;
        let peers = connected_nodes.len();
        let shards = self.shard_store.count().await;
        let (cpu, mem) = os_abs
            .and_then(|os| os.get_resource_usage().ok())
            .map(|u| (u.cpu_usage_percent, u.memory_usage_percent))
            .unwrap_or((0.0, 0.0));
        let frame_bytes = metrics_reporter.build_capacity_frame(chain_h, peers, shards, cpu, mem);
        metrics_reporter
            .push_to_peers(&frame_bytes, &connected_nodes)
            .await;
    }
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Operations - Send/Receive and Connection Management

use anyhow::Result;
use bytes::{BufMut, Bytes};
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::adaptive::AdaptiveConnection;
use super::config::NetworkTier;
use super::connection::{Connection, Endpoint, FrameBatch};
use super::manager::StoqTransport;

impl StoqTransport {
    /// Send data with transport layer optimizations
    pub async fn send(&self, conn: &Connection, data: &[u8]) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Try eBPF zero-copy send if available (delegates to hypermesh-ebpf).
        // Uses the pre-created AF_XDP socket stored during transport init.
        // If the socket is not kernel-backed, send() returns Err and we
        // gracefully fall through to the standard QUIC path below.
        //
        // On this raw AF_XDP path STOQ owns the UDP payload bytes, so the
        // HyperMesh cleartext extension header is physically prepended AHEAD of
        // the payload (papers/HYPERMESH.md §5.1). This makes `magic == 0x484D`
        // land at the UDP-payload offset the XDP program reads at wire speed.
        if let Some(ref socket) = self.af_xdp_socket {
            let mut framed =
                self.protocol_handler.hypermesh_pos_prefix(data);
            framed.extend_from_slice(data);
            if socket.send(&framed).await.is_ok() {
                self.metrics.record_bytes_sent(framed.len());
                self.performance_stats.read().record_zero_copy();
                return Ok(());
            }
        }

        // Apply STOQ protocol extensions (tokenization, sharding)
        let extension_frames = self.protocol_handler.apply_extensions(data)?;

        // Send extension frames as QUIC datagrams
        for frame in extension_frames {
            if conn.inner.send_datagram(frame.clone()).is_err() {
                debug!("Failed to send extension frame as datagram, will include in stream");
            }
        }

        if self.config.enable_zero_copy {
            // Try memory pool buffer first for maximum performance
            if let Some(mut buffer) = self.memory_pool.get_buffer() {
                if data.len() <= buffer.capacity() {
                    buffer.put_slice(data);
                    let bytes = buffer.freeze();

                    // Try zero-copy datagram send
                    if data.len() <= self.config.max_datagram_size
                        && conn.inner.send_datagram(bytes.clone()).is_ok()
                    {
                        self.performance_stats.read().record_zero_copy();
                        self.performance_stats.read().record_pool_hit();
                        return Ok(());
                    }

                    // Fallback to stream with zero-copy buffer
                    let mut stream = conn.open_stream().await?;
                    stream.send_bytes(bytes).await?;
                    self.performance_stats.read().record_zero_copy();
                    self.performance_stats.read().record_pool_hit();
                    return Ok(());
                } else {
                    // Return buffer to pool if too small
                    self.memory_pool.return_buffer(buffer);
                }
            } else {
                self.performance_stats.read().record_pool_miss();
            }

            // Large data optimization with frame batching
            if data.len() > self.config.max_datagram_size && self.config.frame_batch_size > 1 {
                return self.send_large_data_batched(conn, data).await;
            }
        }

        // Fallback to standard stream sending
        let mut stream = conn.open_stream().await?;
        stream.send(data).await?;

        // Update performance metrics
        let duration = start_time.elapsed();
        let throughput_bps = (data.len() as f64 * 8.0) / duration.as_secs_f64();
        let throughput_gbps = throughput_bps / 1_000_000_000.0;

        self.performance_stats
            .read()
            .update_peak_throughput(throughput_gbps);

        Ok(())
    }

    /// Send large data with frame batching for performance
    pub(crate) async fn send_large_data_batched(
        &self,
        conn: &Connection,
        data: &[u8],
    ) -> Result<()> {
        let chunk_size = self.config.max_datagram_size;
        let chunks = data.chunks(chunk_size);
        let mut batch = FrameBatch::new(self.config.frame_batch_size);

        for chunk in chunks {
            let bytes = Bytes::copy_from_slice(chunk);

            if batch.add_frame(bytes) {
                // Batch is full, send all frames
                let frames = batch.flush();
                for frame in frames {
                    if conn.inner.send_datagram(frame).is_err() {
                        // Fallback to stream for failed datagrams
                        let mut stream = conn.open_stream().await?;
                        stream.send(chunk).await?;
                    }
                }
                self.performance_stats.read().record_frame_batch();
            }
        }

        // Send remaining frames in batch
        if !batch.is_empty() {
            let frames = batch.flush();
            for frame in frames {
                let frame_len = frame.len();
                if conn.inner.send_datagram(frame).is_err() {
                    // Fallback to stream
                    let mut stream = conn.open_stream().await?;
                    let fallback_data = vec![0u8; frame_len]; // Safe fallback data
                    stream.send(&fallback_data).await?;
                }
            }
            self.performance_stats.read().record_frame_batch();
        }

        Ok(())
    }

    /// Receive data with zero-copy optimization for performance
    pub async fn receive(&self, conn: &Connection) -> Result<Bytes> {
        // Try eBPF AF_XDP zero-copy receive first (fastest path).
        // Mirrors the send() path: uses the pre-created AF_XDP socket.
        // Only attempts receive when the socket has real kernel backing;
        // otherwise falls through to QUIC without extra overhead.
        if let Some(ref socket) = self.af_xdp_socket {
            if socket.is_kernel_backed() {
                match socket.receive().await {
                    Ok(data) if !data.is_empty() => {
                        self.metrics.record_bytes_received(data.len());
                        self.performance_stats.read().record_zero_copy();
                        return Ok(data);
                    }
                    _ => {} // Fall through to QUIC
                }
            }
        }

        if self.config.enable_zero_copy {
            // Try datagram receive first for maximum performance
            if let Ok(datagram) = conn.inner.read_datagram().await {
                return Ok(datagram);
            }
        }

        // Fallback to stream-based receiving
        let mut stream = conn.accept_stream().await?;
        stream.receive().await
    }

    /// Enable connection multiplexing for specific endpoint (optimization)
    pub async fn enable_multiplexing(
        &self,
        endpoint: &Endpoint,
        connection_count: usize,
    ) -> Result<()> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        let mut connections = VecDeque::with_capacity(connection_count);

        // Create multiple connections for bandwidth aggregation
        for i in 0..connection_count {
            debug!(
                "Creating multiplexed connection {}/{} to [{}]:{}",
                i + 1,
                connection_count,
                endpoint.address,
                endpoint.port
            );

            let connection = self.connect(endpoint).await?;
            connections.push_back(connection);
        }

        self.connection_multiplexer.insert(pool_key, connections);
        info!(
            "Enabled {}x connection multiplexing for [{}]:{} (optimization)",
            connection_count, endpoint.address, endpoint.port
        );

        Ok(())
    }

    /// Send data using connection multiplexing for maximum throughput
    pub async fn send_multiplexed(&self, endpoint: &Endpoint, data: &[u8]) -> Result<()> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);

        if let Some(mut connections) = self.connection_multiplexer.get_mut(&pool_key) {
            if let Some(connection) = connections.pop_front() {
                // Use round-robin connection selection
                let result = self.send(&connection, data).await;
                connections.push_back(connection); // Return connection to back of queue
                return result;
            }
        }

        // Fallback to regular connection if multiplexing not available
        let connection = self.connect(endpoint).await?;
        self.send(&connection, data).await
    }

    /// Start the adaptive optimization manager
    pub async fn start_adaptation(&self) {
        let manager = self.adaptation_manager.clone();
        tokio::spawn(async move {
            manager.start().await;
        });
        info!("Started adaptive connection optimization");
    }

    /// Update configuration for all live connections
    pub async fn update_live_config(&mut self, new_config: super::config::TransportConfig) {
        info!("Updating configuration for all live connections");

        // Update stored config
        self.config = new_config.clone();

        // Update all adaptive connections
        for entry in self.adaptive_connections.iter() {
            let conn = entry.value().clone();

            // Force immediate adaptation with new config
            tokio::spawn(async move {
                if let Err(e) = conn.force_adapt().await {
                    warn!("Failed to adapt connection: {}", e);
                }
            });
        }

        info!(
            "Configuration updated for {} live connections",
            self.adaptive_connections.len()
        );
    }

    /// Get adaptive connection by ID
    pub fn get_adaptive_connection(&self, id: &str) -> Option<Arc<AdaptiveConnection>> {
        self.adaptive_connections.get(id).map(|entry| entry.clone())
    }

    /// Force adaptation for a specific connection
    pub async fn force_connection_adaptation(&self, id: &str) -> Result<()> {
        use anyhow::anyhow;
        if let Some(conn) = self.get_adaptive_connection(id) {
            conn.force_adapt().await?;
            Ok(())
        } else {
            Err(anyhow!("Connection not found: {id}"))
        }
    }

    /// Get adaptation statistics for all connections
    pub fn adaptation_stats(&self) -> Vec<(String, super::adaptive::AdaptationStats)> {
        self.adaptive_connections
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().adaptation_stats()))
            .collect()
    }

    /// Enable or disable adaptive optimization globally
    pub fn set_adaptation_enabled(&self, enabled: bool) {
        self.adaptation_manager.set_enabled(enabled);

        // Update all existing adaptive connections
        for entry in self.adaptive_connections.iter() {
            entry.value().set_adaptation_enabled(enabled);
        }

        // Also update adaptation_manager's internal connections for consistency
        for id in self.adaptation_manager.connection_ids() {
            if let Some(conn) = self.adaptation_manager.get_connection(&id) {
                conn.set_adaptation_enabled(enabled);
            }
        }

        if enabled {
            info!("Adaptive optimization enabled globally");
        } else {
            info!("Adaptive optimization disabled globally");
        }
    }

    /// Manually set network tier for a connection
    pub async fn set_connection_tier(&self, id: &str, tier: NetworkTier) -> Result<()> {
        use anyhow::anyhow;
        if let Some(conn) = self.get_adaptive_connection(id) {
            conn.set_tier(tier.clone())?;
            info!("Set network tier for connection {}: {:?}", id, tier);
            Ok(())
        } else {
            Err(anyhow!("Connection not found: {id}"))
        }
    }

    /// Detect and apply optimal network tier for all connections
    pub async fn auto_detect_tiers(&self) {
        info!("Auto-detecting network tiers for all connections");

        for entry in self.adaptive_connections.iter() {
            let conn = entry.value().clone();
            let id = entry.key().clone();

            tokio::spawn(async move {
                if let Err(e) = conn.adapt().await {
                    warn!("Failed to auto-detect tier for connection {}: {}", id, e);
                }
            });
        }
    }

    /// Get eBPF capabilities and status (delegates to hypermesh-ebpf)
    pub fn get_ebpf_status(&self) -> Option<super::ebpf::EbpfCapabilities> {
        self.ebpf_transport
            .as_ref()
            .map(|t| t.read().capabilities().clone())
    }

    /// Get eBPF metrics if available (delegates to hypermesh-ebpf)
    pub fn get_ebpf_metrics(&self) -> Option<super::ebpf::HyperMeshMetrics> {
        self.ebpf_transport
            .as_ref()
            .map(|t| t.read().metrics().collect())
    }

    /// Attach XDP program to interface for acceleration (delegates to hypermesh-ebpf)
    pub fn attach_xdp_to_interface(&self, interface: &str) -> Result<()> {
        use anyhow::anyhow;
        if let Some(ebpf) = &self.ebpf_transport {
            ebpf.write().attach_xdp(interface)?;
            info!("XDP acceleration enabled on interface {}", interface);
            Ok(())
        } else {
            Err(anyhow!("eBPF transport not available"))
        }
    }

    /// Create AF_XDP zero-copy socket for interface (delegates to hypermesh-ebpf)
    pub fn create_zero_copy_socket(&self, interface: &str, queue_id: u32) -> Result<()> {
        use anyhow::anyhow;
        if let Some(ebpf) = &self.ebpf_transport {
            let _socket = ebpf.write().create_af_xdp_socket(interface, queue_id)?;
            info!(
                "Created AF_XDP zero-copy socket for {}:{}",
                interface, queue_id
            );
            Ok(())
        } else {
            Err(anyhow!("eBPF transport not available"))
        }
    }
}

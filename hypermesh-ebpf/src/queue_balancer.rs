// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Queue AF_XDP Load Balancing
//!
//! Provides load balancing across multiple AF_XDP queues on a single
//! network interface. Three built-in strategies are available:
//!
//! - [`RoundRobinBalancer`] — simple cycling for uniform distribution
//! - [`LeastLoadedBalancer`] — selects queue with most free ring space
//! - [`FlowHashBalancer`] — flow-affine steering preserving packet order
//!
//! The [`MultiQueueManager`] wraps multiple [`AfXdpSocket`]s and uses a
//! [`QueueBalancer`] strategy to route packets to the appropriate queue.

use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Result, anyhow};
use bytes::Bytes;

use crate::af_xdp::{AfXdpManager, AfXdpSocket, AfXdpStats};

// -----------------------------------------------------------------------
// Packet hint and queue metrics
// -----------------------------------------------------------------------

/// Metadata about a packet to help the balancer select a queue.
#[derive(Debug, Clone, Copy)]
pub struct PacketHint {
    /// 5-tuple flow hash for flow-affine steering
    pub flow_hash: u32,
    /// Packet size in bytes
    pub packet_size: u32,
    /// Priority: 0=best-effort, 1=priority, 2=control
    pub priority: u8,
}

impl Default for PacketHint {
    fn default() -> Self {
        Self {
            flow_hash: 0,
            packet_size: 0,
            priority: 0,
        }
    }
}

/// Per-queue health/load metrics used by balancer strategies.
#[derive(Debug, Clone, Default)]
pub struct QueueMetrics {
    /// Queue identifier
    pub queue_id: u32,
    /// Available RX buffer slots (fill ring free space)
    pub fill_ring_free: u32,
    /// Available TX slots
    pub tx_ring_free: u32,
    /// Total packets processed on this queue
    pub packets_processed: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Packets dropped on this queue
    pub drops: u64,
}

// -----------------------------------------------------------------------
// QueueBalancer trait
// -----------------------------------------------------------------------

/// Trait for queue selection strategies.
///
/// Implementations must be `Send + Sync` so the balancer can be shared
/// across async tasks.
pub trait QueueBalancer: Send + Sync {
    /// Select which queue index to use for a packet.
    ///
    /// Returns a queue index in `0..queue_metrics.len()`.
    fn select_queue(&self, hint: &PacketHint, queue_metrics: &[QueueMetrics]) -> u32;

    /// Human-readable strategy name.
    fn name(&self) -> &str;
}

// -----------------------------------------------------------------------
// RoundRobinBalancer
// -----------------------------------------------------------------------

/// Cycles through queues sequentially using an atomic counter.
///
/// O(1) selection. Ignores packet hints and queue load.
pub struct RoundRobinBalancer {
    counter: AtomicU32,
}

impl RoundRobinBalancer {
    /// Create a new round-robin balancer starting at index 0.
    pub fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }
}

impl Default for RoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueBalancer for RoundRobinBalancer {
    fn select_queue(&self, _hint: &PacketHint, queue_metrics: &[QueueMetrics]) -> u32 {
        let count = queue_metrics.len() as u32;
        if count == 0 {
            return 0;
        }
        self.counter.fetch_add(1, Ordering::Relaxed) % count
    }

    fn name(&self) -> &str {
        "round-robin"
    }
}

// -----------------------------------------------------------------------
// LeastLoadedBalancer
// -----------------------------------------------------------------------

/// Selects the queue with the most free ring space.
///
/// O(n) scan where n is the number of queues. Tie-breaks on fewest
/// `packets_processed` (prefer less-utilized queues).
pub struct LeastLoadedBalancer;

impl LeastLoadedBalancer {
    /// Create a new least-loaded balancer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastLoadedBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueBalancer for LeastLoadedBalancer {
    fn select_queue(&self, _hint: &PacketHint, queue_metrics: &[QueueMetrics]) -> u32 {
        if queue_metrics.is_empty() {
            return 0;
        }

        let mut best_idx: u32 = 0;
        let mut best_free: u32 = 0;
        let mut best_processed: u64 = u64::MAX;

        for (i, m) in queue_metrics.iter().enumerate() {
            let free = m.fill_ring_free.saturating_add(m.tx_ring_free);
            if free > best_free || (free == best_free && m.packets_processed < best_processed) {
                best_idx = i as u32;
                best_free = free;
                best_processed = m.packets_processed;
            }
        }

        best_idx
    }

    fn name(&self) -> &str {
        "least-loaded"
    }
}

// -----------------------------------------------------------------------
// FlowHashBalancer
// -----------------------------------------------------------------------

/// Steers packets by flow hash, ensuring same-flow packets always land
/// on the same queue and preserving packet ordering within a flow.
///
/// O(1) selection. Uses `hint.flow_hash % queue_count`.
pub struct FlowHashBalancer;

impl FlowHashBalancer {
    /// Create a new flow-hash balancer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlowHashBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueBalancer for FlowHashBalancer {
    fn select_queue(&self, hint: &PacketHint, queue_metrics: &[QueueMetrics]) -> u32 {
        let count = queue_metrics.len() as u32;
        if count == 0 {
            return 0;
        }
        hint.flow_hash % count
    }

    fn name(&self) -> &str {
        "flow-hash"
    }
}

// -----------------------------------------------------------------------
// MultiQueueManager
// -----------------------------------------------------------------------

/// Manages multiple AF_XDP sockets across queues with load-balanced
/// packet steering.
///
/// Wraps a vector of [`AfXdpSocket`]s (one per queue) and uses a
/// [`QueueBalancer`] strategy to select the target queue for each packet.
pub struct MultiQueueManager {
    sockets: Vec<AfXdpSocket>,
    balancer: Box<dyn QueueBalancer>,
    interface: String,
    /// Round-robin counter for `receive_any` polling
    rx_poll_counter: AtomicU32,
}

impl MultiQueueManager {
    /// Create a multi-queue manager by allocating one AF_XDP socket per
    /// queue on the given interface.
    ///
    /// `queue_count` must be >= 1. Queues are numbered 0..queue_count.
    pub fn new(
        af_xdp_manager: &mut AfXdpManager,
        balancer: Box<dyn QueueBalancer>,
        interface: &str,
        queue_count: u32,
    ) -> Result<Self> {
        if queue_count == 0 {
            return Err(anyhow!("queue_count must be >= 1"));
        }

        let mut sockets = Vec::with_capacity(queue_count as usize);
        for q in 0..queue_count {
            let socket = af_xdp_manager
                .create_socket(interface, q)
                .map_err(|e| anyhow!("failed to create socket for {}:{}: {}", interface, q, e))?;
            sockets.push(socket);
        }

        tracing::info!(
            "MultiQueueManager created: interface={}, queues={}, strategy={}",
            interface,
            queue_count,
            balancer.name(),
        );

        Ok(Self {
            sockets,
            balancer,
            interface: interface.to_string(),
            rx_poll_counter: AtomicU32::new(0),
        })
    }

    /// Send a packet using the balancer to select the queue.
    pub async fn send(&self, data: &[u8], hint: &PacketHint) -> Result<()> {
        let metrics = self.collect_queue_metrics();
        let idx = self.balancer.select_queue(hint, &metrics) as usize;
        let idx = idx % self.sockets.len();
        self.sockets[idx].send(data).await
    }

    /// Send a batch of packets, grouping by selected queue and sending
    /// per-queue batches. Returns the total number of packets sent.
    pub async fn send_batch(&self, packets: &[(&[u8], PacketHint)]) -> Result<usize> {
        if packets.is_empty() {
            return Ok(0);
        }

        let metrics = self.collect_queue_metrics();
        let queue_count = self.sockets.len();

        // Group packets by target queue
        let mut per_queue: Vec<Vec<&[u8]>> = vec![Vec::new(); queue_count];
        for (data, hint) in packets {
            let idx = self.balancer.select_queue(hint, &metrics) as usize;
            let idx = idx % queue_count;
            per_queue[idx].push(*data);
        }

        let mut total_sent = 0usize;
        for (q, batch) in per_queue.iter().enumerate() {
            if batch.is_empty() {
                continue;
            }
            match self.sockets[q].send_batch(batch).await {
                Ok(n) => total_sent += n,
                Err(e) => {
                    tracing::debug!(
                        "send_batch on queue {} failed (sent {} so far): {}",
                        q, total_sent, e,
                    );
                }
            }
        }

        Ok(total_sent)
    }

    /// Receive packets from any queue using round-robin polling.
    ///
    /// Tries each queue once (starting from a rotating offset) until
    /// `max_packets` are collected or all queues are exhausted.
    pub async fn receive_any(&self, max_packets: usize) -> Result<Vec<Bytes>> {
        if max_packets == 0 {
            return Ok(Vec::new());
        }

        let queue_count = self.sockets.len();
        let start = self.rx_poll_counter.fetch_add(1, Ordering::Relaxed) as usize;
        let mut collected = Vec::new();

        for offset in 0..queue_count {
            let idx = (start + offset) % queue_count;
            let remaining = max_packets - collected.len();

            match self.sockets[idx].receive_batch(remaining).await {
                Ok(pkts) => collected.extend(pkts),
                Err(_) => {
                    // Queue empty or not kernel-backed — try next queue
                }
            }

            if collected.len() >= max_packets {
                break;
            }
        }

        Ok(collected)
    }

    /// Collect per-queue metrics from all sockets.
    pub fn collect_queue_metrics(&self) -> Vec<QueueMetrics> {
        self.sockets
            .iter()
            .map(|sock| {
                let stats = sock.get_stats();
                QueueMetrics {
                    queue_id: sock.queue_id(),
                    fill_ring_free: sock.fill_ring_free(),
                    tx_ring_free: sock.tx_ring_free(),
                    packets_processed: stats.packets_sent + stats.packets_received,
                    bytes_processed: stats.bytes_sent + stats.bytes_received,
                    drops: stats.tx_ring_full + stats.rx_ring_empty,
                }
            })
            .collect()
    }

    /// Number of queues managed.
    pub fn queue_count(&self) -> u32 {
        self.sockets.len() as u32
    }

    /// Aggregated stats across all queues.
    pub fn total_stats(&self) -> AfXdpStats {
        let mut total = AfXdpStats::default();
        for sock in &self.sockets {
            let s = sock.get_stats();
            total.packets_sent += s.packets_sent;
            total.packets_received += s.packets_received;
            total.bytes_sent += s.bytes_sent;
            total.bytes_received += s.bytes_received;
            total.tx_ring_full += s.tx_ring_full;
            total.rx_ring_empty += s.rx_ring_empty;
            total.invalid_descriptors += s.invalid_descriptors;
        }
        total
    }

    /// Get the balancer strategy name.
    pub fn strategy(&self) -> &str {
        self.balancer.name()
    }

    /// Get the interface name.
    pub fn interface(&self) -> &str {
        &self.interface
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- PacketHint / QueueMetrics defaults --

    #[test]
    fn test_packet_hint_default() {
        let hint = PacketHint::default();
        assert_eq!(hint.flow_hash, 0);
        assert_eq!(hint.packet_size, 0);
        assert_eq!(hint.priority, 0);
    }

    #[test]
    fn test_queue_metrics_default() {
        let m = QueueMetrics::default();
        assert_eq!(m.queue_id, 0);
        assert_eq!(m.fill_ring_free, 0);
        assert_eq!(m.tx_ring_free, 0);
        assert_eq!(m.packets_processed, 0);
        assert_eq!(m.bytes_processed, 0);
        assert_eq!(m.drops, 0);
    }

    // -- RoundRobinBalancer --

    #[test]
    fn test_round_robin_cycles() {
        let balancer = RoundRobinBalancer::new();
        let metrics = vec![
            QueueMetrics { queue_id: 0, ..Default::default() },
            QueueMetrics { queue_id: 1, ..Default::default() },
            QueueMetrics { queue_id: 2, ..Default::default() },
        ];
        let hint = PacketHint::default();

        let q0 = balancer.select_queue(&hint, &metrics);
        let q1 = balancer.select_queue(&hint, &metrics);
        let q2 = balancer.select_queue(&hint, &metrics);
        let q3 = balancer.select_queue(&hint, &metrics);
        let q4 = balancer.select_queue(&hint, &metrics);
        let q5 = balancer.select_queue(&hint, &metrics);

        assert_eq!(q0, 0);
        assert_eq!(q1, 1);
        assert_eq!(q2, 2);
        assert_eq!(q3, 0);
        assert_eq!(q4, 1);
        assert_eq!(q5, 2);
    }

    // -- LeastLoadedBalancer --

    #[test]
    fn test_least_loaded_selects_emptiest() {
        let balancer = LeastLoadedBalancer::new();
        let hint = PacketHint::default();

        let metrics = vec![
            QueueMetrics {
                queue_id: 0,
                fill_ring_free: 10,
                tx_ring_free: 5,
                packets_processed: 100,
                ..Default::default()
            },
            QueueMetrics {
                queue_id: 1,
                fill_ring_free: 200,
                tx_ring_free: 100,
                packets_processed: 50,
                ..Default::default()
            },
            QueueMetrics {
                queue_id: 2,
                fill_ring_free: 50,
                tx_ring_free: 20,
                packets_processed: 80,
                ..Default::default()
            },
        ];

        // Queue 1 has most free space (300 total)
        assert_eq!(balancer.select_queue(&hint, &metrics), 1);
    }

    #[test]
    fn test_least_loaded_tiebreak() {
        let balancer = LeastLoadedBalancer::new();
        let hint = PacketHint::default();

        let metrics = vec![
            QueueMetrics {
                queue_id: 0,
                fill_ring_free: 100,
                tx_ring_free: 50,
                packets_processed: 500,
                ..Default::default()
            },
            QueueMetrics {
                queue_id: 1,
                fill_ring_free: 100,
                tx_ring_free: 50,
                packets_processed: 200,
                ..Default::default()
            },
            QueueMetrics {
                queue_id: 2,
                fill_ring_free: 100,
                tx_ring_free: 50,
                packets_processed: 800,
                ..Default::default()
            },
        ];

        // All have 150 free, queue 1 has fewest packets_processed
        assert_eq!(balancer.select_queue(&hint, &metrics), 1);
    }

    // -- FlowHashBalancer --

    #[test]
    fn test_flow_hash_affinity() {
        let balancer = FlowHashBalancer::new();
        let metrics = vec![
            QueueMetrics { queue_id: 0, ..Default::default() },
            QueueMetrics { queue_id: 1, ..Default::default() },
            QueueMetrics { queue_id: 2, ..Default::default() },
            QueueMetrics { queue_id: 3, ..Default::default() },
        ];

        let hint = PacketHint {
            flow_hash: 0xDEAD_BEEF,
            packet_size: 1500,
            priority: 0,
        };

        // Same flow hash must always go to the same queue
        let first = balancer.select_queue(&hint, &metrics);
        for _ in 0..100 {
            assert_eq!(
                balancer.select_queue(&hint, &metrics),
                first,
                "flow-hash affinity violated",
            );
        }
    }

    #[test]
    fn test_flow_hash_distribution() {
        let balancer = FlowHashBalancer::new();
        let metrics = vec![
            QueueMetrics { queue_id: 0, ..Default::default() },
            QueueMetrics { queue_id: 1, ..Default::default() },
            QueueMetrics { queue_id: 2, ..Default::default() },
            QueueMetrics { queue_id: 3, ..Default::default() },
        ];

        // Collect which queues get selected for different hashes
        let mut seen = std::collections::HashSet::new();
        for i in 0..100u32 {
            let hint = PacketHint {
                flow_hash: i * 7919, // prime multiplier for spread
                packet_size: 100,
                priority: 0,
            };
            seen.insert(balancer.select_queue(&hint, &metrics));
        }

        // With 100 different hashes across 4 queues, all should be hit
        assert_eq!(seen.len(), 4, "flow hashes should distribute across all queues");
    }

    // -- Single queue: all strategies work --

    #[test]
    fn test_single_queue_all_strategies() {
        let metrics = vec![QueueMetrics { queue_id: 0, ..Default::default() }];
        let hint = PacketHint {
            flow_hash: 42,
            packet_size: 100,
            priority: 1,
        };

        let rr = RoundRobinBalancer::new();
        assert_eq!(rr.select_queue(&hint, &metrics), 0);
        assert_eq!(rr.name(), "round-robin");

        let ll = LeastLoadedBalancer::new();
        assert_eq!(ll.select_queue(&hint, &metrics), 0);
        assert_eq!(ll.name(), "least-loaded");

        let fh = FlowHashBalancer::new();
        assert_eq!(fh.select_queue(&hint, &metrics), 0);
        assert_eq!(fh.name(), "flow-hash");
    }

    // -- MultiQueueManager --

    #[test]
    fn test_multi_queue_creation() {
        let mut mgr = AfXdpManager::new().expect("test: create AfXdpManager");
        let balancer = Box::new(RoundRobinBalancer::new());
        let mqm = MultiQueueManager::new(&mut mgr, balancer, "eth0", 4);
        assert!(mqm.is_ok());
        let mqm = mqm.expect("test: create MultiQueueManager");
        assert_eq!(mqm.queue_count(), 4);
        assert_eq!(mqm.strategy(), "round-robin");
        assert_eq!(mqm.interface(), "eth0");
    }

    #[test]
    fn test_multi_queue_stats_aggregation() {
        let mut mgr = AfXdpManager::new().expect("test: create AfXdpManager");
        let balancer = Box::new(FlowHashBalancer::new());
        let mqm = MultiQueueManager::new(&mut mgr, balancer, "eth0", 3)
            .expect("test: create MultiQueueManager");

        let total = mqm.total_stats();
        // Fresh sockets have all-zero stats
        assert_eq!(total.packets_sent, 0);
        assert_eq!(total.packets_received, 0);
        assert_eq!(total.bytes_sent, 0);
        assert_eq!(total.bytes_received, 0);
        assert_eq!(total.tx_ring_full, 0);
        assert_eq!(total.rx_ring_empty, 0);
        assert_eq!(total.invalid_descriptors, 0);
    }

    #[test]
    fn test_multi_queue_zero_queues_rejected() {
        let mut mgr = AfXdpManager::new().expect("test: create AfXdpManager");
        let balancer = Box::new(RoundRobinBalancer::new());
        let result = MultiQueueManager::new(&mut mgr, balancer, "eth0", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_queue_collect_metrics() {
        let mut mgr = AfXdpManager::new().expect("test: create AfXdpManager");
        let balancer = Box::new(LeastLoadedBalancer::new());
        let mqm = MultiQueueManager::new(&mut mgr, balancer, "eth0", 2)
            .expect("test: create MultiQueueManager");

        let metrics = mqm.collect_queue_metrics();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].queue_id, 0);
        assert_eq!(metrics[1].queue_id, 1);
    }

    // -- Empty metrics edge case --

    #[test]
    fn test_balancers_with_empty_metrics() {
        let empty: Vec<QueueMetrics> = Vec::new();
        let hint = PacketHint::default();

        assert_eq!(RoundRobinBalancer::new().select_queue(&hint, &empty), 0);
        assert_eq!(LeastLoadedBalancer::new().select_queue(&hint, &empty), 0);
        assert_eq!(FlowHashBalancer::new().select_queue(&hint, &empty), 0);
    }
}

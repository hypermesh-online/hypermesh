// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP (Address Family XDP) zero-copy socket implementation
//!
//! Provides zero-copy packet I/O bypassing the kernel network stack
//! for maximum performance with STOQ transport.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use bytes::Bytes;

/// AF_XDP socket for zero-copy packet I/O
///
/// AF_XDP zero-copy requires the `xsk-rs` crate and kernel 4.18+.
/// This struct provides the tracking interface; real zero-copy I/O
/// will be enabled when xsk-rs integration is complete.
pub struct AfXdpSocket {
    interface: String,
    queue_id: u32,
    stats: Arc<RwLock<AfXdpStats>>,
    /// Whether this socket has real kernel AF_XDP backing
    kernel_backed: bool,
}

/// AF_XDP socket statistics
#[derive(Debug, Default, Clone)]
pub struct AfXdpStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub tx_ring_full: u64,
    pub rx_ring_empty: u64,
    pub invalid_descriptors: u64,
}

/// AF_XDP socket manager
pub struct AfXdpManager {
    sockets: Arc<RwLock<HashMap<String, AfXdpSocket>>>,
    /// UMEM configuration
    umem_config: UmemConfig,
    /// Ring configuration
    ring_config: RingConfig,
}

/// UMEM (User Memory) configuration
#[derive(Debug, Clone)]
pub struct UmemConfig {
    /// Number of frames in UMEM
    pub frame_count: u32,
    /// Size of each frame
    pub frame_size: u32,
    /// Headroom for each frame
    pub frame_headroom: u32,
    /// Use huge pages
    pub use_huge_pages: bool,
}

impl Default for UmemConfig {
    fn default() -> Self {
        Self {
            frame_count: 4096,
            frame_size: 4096,
            frame_headroom: 256,
            use_huge_pages: false,
        }
    }
}

/// Ring buffer configuration
#[derive(Debug, Clone)]
pub struct RingConfig {
    /// TX ring size (must be power of 2)
    pub tx_size: u32,
    /// RX ring size (must be power of 2)
    pub rx_size: u32,
    /// Fill ring size
    pub fill_size: u32,
    /// Completion ring size
    pub comp_size: u32,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            tx_size: 2048,
            rx_size: 2048,
            fill_size: 2048,
            comp_size: 2048,
        }
    }
}

impl AfXdpManager {
    /// Create new AF_XDP manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            sockets: Arc::new(RwLock::new(HashMap::new())),
            umem_config: UmemConfig::default(),
            ring_config: RingConfig::default(),
        })
    }

    /// Create AF_XDP socket for interface and queue
    ///
    /// AF_XDP zero-copy requires the `xsk-rs` crate for real kernel socket creation.
    /// Without it, this creates a tracking socket that uses standard socket I/O as fallback.
    pub fn create_socket(&mut self, interface: &str, queue_id: u32) -> Result<AfXdpSocket> {
        let socket_key = format!("{}:{}", interface, queue_id);

        if self.sockets.read().contains_key(&socket_key) {
            return Err(anyhow!("Socket already exists for {}:{}", interface, queue_id));
        }

        // AF_XDP requires xsk-rs crate and kernel 4.18+ with CAP_NET_ADMIN.
        // Real zero-copy socket creation will be enabled when xsk-rs is integrated.
        let kernel_backed = false;

        if !kernel_backed {
            tracing::info!(
                "AF_XDP socket for {}:{} using standard I/O fallback (xsk-rs not integrated)",
                interface, queue_id
            );
        }

        let af_xdp_socket = AfXdpSocket {
            interface: interface.to_string(),
            queue_id,
            stats: Arc::new(RwLock::new(AfXdpStats::default())),
            kernel_backed,
        };

        self.sockets.write().insert(socket_key, af_xdp_socket.clone());

        Ok(af_xdp_socket)
    }

    /// Close socket
    pub fn close_socket(&mut self, interface: &str, queue_id: u32) -> Result<()> {
        let socket_key = format!("{}:{}", interface, queue_id);

        if self.sockets.write().remove(&socket_key).is_some() {
            tracing::info!("Closed AF_XDP socket for {}:{}", interface, queue_id);
            Ok(())
        } else {
            Err(anyhow!("Socket not found for {}:{}", interface, queue_id))
        }
    }

    /// Close all sockets
    pub fn close_all(&mut self) -> Result<()> {
        self.sockets.write().clear();
        tracing::info!("Closed all AF_XDP sockets");
        Ok(())
    }

    /// Get socket statistics
    pub fn get_stats(&self, interface: &str, queue_id: u32) -> Option<AfXdpStats> {
        let socket_key = format!("{}:{}", interface, queue_id);
        self.sockets.read().get(&socket_key).map(|s| s.stats.read().clone())
    }
}

impl AfXdpSocket {
    /// Whether this socket has real kernel AF_XDP zero-copy backing
    pub fn is_kernel_backed(&self) -> bool {
        self.kernel_backed
    }

    /// Send packet via AF_XDP zero-copy (or standard I/O fallback)
    ///
    /// When kernel-backed, uses UMEM zero-copy for maximum throughput.
    /// When in fallback mode, tracks statistics but requires the caller
    /// to use standard socket I/O for actual transmission.
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        if !self.kernel_backed {
            // Fallback mode: track stats, caller handles actual I/O
            let mut stats = self.stats.write();
            stats.packets_sent += 1;
            stats.bytes_sent += data.len() as u64;
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard socket I/O for {}:{}",
                self.interface, self.queue_id
            ));
        }

        // Real AF_XDP zero-copy send would happen here via xsk-rs
        let mut stats = self.stats.write();
        stats.packets_sent += 1;
        stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    /// Receive packet via AF_XDP zero-copy (or standard I/O fallback)
    pub async fn receive(&self) -> Result<Bytes> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard socket I/O for {}:{}",
                self.interface, self.queue_id
            ));
        }

        // Real AF_XDP zero-copy receive would happen here via xsk-rs
        self.stats.write().packets_received += 1;
        Ok(Bytes::new())
    }

    /// Send multiple packets in batch for efficiency
    pub async fn send_batch(&self, packets: &[&[u8]]) -> Result<usize> {
        if !self.kernel_backed {
            let count = packets.len();
            let mut stats = self.stats.write();
            stats.packets_sent += count as u64;
            for packet in packets {
                stats.bytes_sent += packet.len() as u64;
            }
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard socket I/O for batch send"
            ));
        }

        let count = packets.len();
        let mut stats = self.stats.write();
        stats.packets_sent += count as u64;
        for packet in packets {
            stats.bytes_sent += packet.len() as u64;
        }
        Ok(count)
    }

    /// Receive multiple packets in batch for efficiency
    pub async fn receive_batch(&self, _max_packets: usize) -> Result<Vec<Bytes>> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard socket I/O for batch receive"
            ));
        }

        Ok(Vec::new())
    }

    /// Get socket statistics
    pub fn get_stats(&self) -> AfXdpStats {
        self.stats.read().clone()
    }
}

impl Clone for AfXdpSocket {
    fn clone(&self) -> Self {
        Self {
            interface: self.interface.clone(),
            queue_id: self.queue_id,
            stats: self.stats.clone(),
            kernel_backed: self.kernel_backed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_umem_config_default() {
        let config = UmemConfig::default();
        assert_eq!(config.frame_count, 4096);
        assert_eq!(config.frame_size, 4096);
        assert_eq!(config.frame_headroom, 256);
        assert!(!config.use_huge_pages);
    }

    #[test]
    fn test_ring_config_default() {
        let config = RingConfig::default();
        assert_eq!(config.tx_size, 2048);
        assert_eq!(config.rx_size, 2048);
        assert_eq!(config.fill_size, 2048);
        assert_eq!(config.comp_size, 2048);
    }

    #[test]
    fn test_af_xdp_manager_creation() {
        let manager = AfXdpManager::new();
        assert!(manager.is_ok());
    }
}
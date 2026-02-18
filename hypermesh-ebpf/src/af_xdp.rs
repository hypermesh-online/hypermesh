// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP (Address Family XDP) Zero-Copy Socket Management
//!
//! Provides zero-copy packet I/O bypassing the kernel network stack
//! for maximum performance on the STOQ fast path. This is execution
//! path 1: AF_XDP -> STOQ (XDP_REDIRECT).
//!
//! Real zero-copy requires the `xsk-rs` crate and kernel 4.18+.
//! Without kernel backing, sockets track statistics and fall back
//! to standard I/O.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;

// -----------------------------------------------------------------------
// AF_XDP Socket
// -----------------------------------------------------------------------

/// AF_XDP socket for zero-copy packet I/O.
///
/// When kernel-backed, provides true zero-copy via UMEM shared memory.
/// When in fallback mode, tracks statistics and signals the caller to
/// use standard socket I/O for actual transmission.
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

// -----------------------------------------------------------------------
// UMEM and Ring configuration
// -----------------------------------------------------------------------

/// UMEM (User Memory) configuration for AF_XDP sockets
#[derive(Debug, Clone)]
pub struct UmemConfig {
    /// Number of frames in UMEM
    pub frame_count: u32,
    /// Size of each frame in bytes
    pub frame_size: u32,
    /// Headroom reserved in each frame
    pub frame_headroom: u32,
    /// Use huge pages for UMEM allocation
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

/// Ring buffer configuration for AF_XDP sockets
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

// -----------------------------------------------------------------------
// AF_XDP Manager
// -----------------------------------------------------------------------

/// Manages AF_XDP sockets for zero-copy packet I/O across interfaces.
pub struct AfXdpManager {
    sockets: Arc<RwLock<HashMap<String, AfXdpSocket>>>,
    /// UMEM configuration for new sockets
    pub umem_config: UmemConfig,
    /// Ring buffer configuration for new sockets
    pub ring_config: RingConfig,
}

impl AfXdpManager {
    /// Create a new AF_XDP manager with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            sockets: Arc::new(RwLock::new(HashMap::new())),
            umem_config: UmemConfig::default(),
            ring_config: RingConfig::default(),
        })
    }

    /// Create a new AF_XDP manager with custom configuration
    pub fn with_config(umem_config: UmemConfig, ring_config: RingConfig) -> Result<Self> {
        Ok(Self {
            sockets: Arc::new(RwLock::new(HashMap::new())),
            umem_config,
            ring_config,
        })
    }

    /// Create an AF_XDP socket for a given interface and queue.
    ///
    /// Real zero-copy requires xsk-rs integration and kernel 4.18+.
    /// Without it, creates a tracking socket that falls back to standard I/O.
    pub fn create_socket(
        &mut self,
        interface: &str,
        queue_id: u32,
    ) -> Result<AfXdpSocket> {
        let socket_key = format!("{}:{}", interface, queue_id);

        if self.sockets.read().contains_key(&socket_key) {
            return Err(anyhow!(
                "Socket already exists for {}:{}",
                interface,
                queue_id
            ));
        }

        // Real AF_XDP requires xsk-rs crate and CAP_NET_ADMIN.
        // When xsk-rs is integrated, this will create true zero-copy sockets.
        let kernel_backed = false;

        if !kernel_backed {
            tracing::info!(
                "AF_XDP socket for {}:{} using standard I/O fallback",
                interface,
                queue_id
            );
        }

        let socket = AfXdpSocket {
            interface: interface.to_string(),
            queue_id,
            stats: Arc::new(RwLock::new(AfXdpStats::default())),
            kernel_backed,
        };

        self.sockets
            .write()
            .insert(socket_key, socket.clone());

        Ok(socket)
    }

    /// Close an AF_XDP socket
    pub fn close_socket(
        &mut self,
        interface: &str,
        queue_id: u32,
    ) -> Result<()> {
        let socket_key = format!("{}:{}", interface, queue_id);

        if self.sockets.write().remove(&socket_key).is_some() {
            tracing::info!("Closed AF_XDP socket for {}:{}", interface, queue_id);
            Ok(())
        } else {
            Err(anyhow!(
                "Socket not found for {}:{}",
                interface,
                queue_id
            ))
        }
    }

    /// Close all AF_XDP sockets
    pub fn close_all(&mut self) -> Result<()> {
        self.sockets.write().clear();
        tracing::info!("Closed all AF_XDP sockets");
        Ok(())
    }

    /// Get statistics for a specific socket
    pub fn get_stats(
        &self,
        interface: &str,
        queue_id: u32,
    ) -> Option<AfXdpStats> {
        let socket_key = format!("{}:{}", interface, queue_id);
        self.sockets
            .read()
            .get(&socket_key)
            .map(|s| s.stats.read().clone())
    }

    /// Get number of active sockets
    pub fn socket_count(&self) -> usize {
        self.sockets.read().len()
    }
}

// -----------------------------------------------------------------------
// AF_XDP Socket operations
// -----------------------------------------------------------------------

impl AfXdpSocket {
    /// Whether this socket has real kernel AF_XDP zero-copy backing
    pub fn is_kernel_backed(&self) -> bool {
        self.kernel_backed
    }

    /// Get the interface this socket is bound to
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Get the queue ID
    pub fn queue_id(&self) -> u32 {
        self.queue_id
    }

    /// Send packet via AF_XDP zero-copy (or signal standard I/O fallback).
    ///
    /// When kernel-backed, uses UMEM zero-copy for maximum throughput.
    /// When in fallback mode, tracks statistics and returns an error
    /// indicating the caller should use standard socket I/O.
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let mut stats = self.stats.write();

        if !self.kernel_backed {
            stats.packets_sent += 1;
            stats.bytes_sent += data.len() as u64;
            return Err(anyhow!(
                "AF_XDP not kernel-backed on {}:{}: use standard I/O",
                self.interface,
                self.queue_id
            ));
        }

        // Real AF_XDP zero-copy send via xsk-rs would happen here
        stats.packets_sent += 1;
        stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    /// Receive packet via AF_XDP zero-copy (or signal standard I/O fallback).
    pub async fn receive(&self) -> Result<Bytes> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed on {}:{}: use standard I/O",
                self.interface,
                self.queue_id
            ));
        }

        // Real AF_XDP zero-copy receive via xsk-rs would happen here
        self.stats.write().packets_received += 1;
        Ok(Bytes::new())
    }

    /// Send multiple packets in batch for efficiency
    pub async fn send_batch(&self, packets: &[&[u8]]) -> Result<usize> {
        let count = packets.len();
        let mut stats = self.stats.write();

        stats.packets_sent += count as u64;
        for packet in packets {
            stats.bytes_sent += packet.len() as u64;
        }

        if !self.kernel_backed {
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard I/O for batch send"
            ));
        }

        Ok(count)
    }

    /// Receive multiple packets in batch
    pub async fn receive_batch(
        &self,
        _max_packets: usize,
    ) -> Result<Vec<Bytes>> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard I/O for batch receive"
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

    #[test]
    fn test_af_xdp_socket_creation() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0);
        assert!(socket.is_ok());
        assert_eq!(manager.socket_count(), 1);

        // Duplicate should fail
        let dup = manager.create_socket("eth0", 0);
        assert!(dup.is_err());
    }

    #[test]
    fn test_af_xdp_socket_close() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _socket = manager.create_socket("eth0", 0).expect("test: create socket");

        assert!(manager.close_socket("eth0", 0).is_ok());
        assert_eq!(manager.socket_count(), 0);

        // Close non-existent should fail
        assert!(manager.close_socket("eth0", 0).is_err());
    }

    #[test]
    fn test_af_xdp_socket_not_kernel_backed() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        assert!(!socket.is_kernel_backed());
        assert_eq!(socket.interface(), "eth0");
        assert_eq!(socket.queue_id(), 0);
    }
}

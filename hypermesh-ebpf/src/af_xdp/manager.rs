// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP socket struct, configuration types, and manager implementation.
//!
//! Contains `AfXdpSocket`, `AfXdpStats`, `UmemConfig`, `RingConfig`,
//! and `AfXdpManager`. Kernel helper functions are in `helpers.rs`.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[cfg(feature = "kernel-attach")]
use super::kernel_types::*;

#[cfg(feature = "kernel-attach")]
use super::helpers::*;

// -----------------------------------------------------------------------
// AF_XDP Socket
// -----------------------------------------------------------------------

/// AF_XDP socket for zero-copy packet I/O.
///
/// When kernel-backed, provides true zero-copy via UMEM shared memory
/// with fill/completion/rx/tx ring buffers. When in fallback mode,
/// tracks statistics and signals the caller to use standard socket I/O.
pub struct AfXdpSocket {
    pub(crate) interface: String,
    pub(crate) queue_id: u32,
    pub(crate) stats: Arc<RwLock<AfXdpStats>>,
    /// Whether this socket has real kernel AF_XDP backing
    pub(crate) kernel_backed: bool,
    /// Kernel-backed AF_XDP state (fd, UMEM, rings, allocator)
    #[cfg(feature = "kernel-attach")]
    pub(crate) kernel_state: Option<Arc<KernelState>>,
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
    /// With `kernel-attach` feature enabled, creates a fully functional
    /// AF_XDP socket with UMEM shared memory, ring buffers, and zero-copy
    /// I/O capability. Without the feature or when the kernel probe fails,
    /// creates a tracking socket that falls back to standard I/O.
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

        // With kernel-attach: attempt full UMEM setup.
        // Without kernel-attach: always fallback.
        #[cfg(feature = "kernel-attach")]
        let (kernel_backed, kernel_state) = self.create_kernel_socket(interface, queue_id);

        #[cfg(not(feature = "kernel-attach"))]
        let kernel_backed = false;

        if kernel_backed {
            tracing::info!(
                "AF_XDP socket for {}:{} has kernel zero-copy backing",
                interface,
                queue_id
            );
        } else {
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
            #[cfg(feature = "kernel-attach")]
            kernel_state,
        };

        self.sockets
            .write()
            .insert(socket_key, socket.clone());

        Ok(socket)
    }

    /// Attempt to create a full kernel-backed AF_XDP socket with UMEM, rings,
    /// and binding. Returns (true, Some(state)) on success, (false, None)
    /// on any failure (graceful degradation).
    #[cfg(feature = "kernel-attach")]
    fn create_kernel_socket(
        &self,
        interface: &str,
        queue_id: u32,
    ) -> (bool, Option<Arc<KernelState>>) {
        match self.setup_kernel_state(interface, queue_id) {
            Ok(state) => (true, Some(Arc::new(state))),
            Err(e) => {
                tracing::debug!(
                    "AF_XDP kernel setup failed for {}:{}: {} (falling back to standard I/O)",
                    interface,
                    queue_id,
                    e
                );
                (false, None)
            }
        }
    }

    /// Full kernel AF_XDP socket setup: UMEM allocation, socket creation,
    /// UMEM registration, ring setup, mmap, bind, fill ring population.
    #[cfg(feature = "kernel-attach")]
    fn setup_kernel_state(
        &self,
        interface: &str,
        queue_id: u32,
    ) -> Result<KernelState> {
        use xdp_consts::*;

        let umem_cfg = &self.umem_config;
        let ring_cfg = &self.ring_config;

        // Validate configuration
        validate_config(umem_cfg, ring_cfg)?;

        let umem_len = umem_cfg.frame_count as usize * umem_cfg.frame_size as usize;

        // Step 1: Allocate UMEM region via mmap
        let mut mmap_flags = libc::MAP_ANONYMOUS | libc::MAP_PRIVATE;
        if umem_cfg.use_huge_pages {
            mmap_flags |= libc::MAP_HUGETLB;
        }
        let umem_area = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                umem_len,
                libc::PROT_READ | libc::PROT_WRITE,
                mmap_flags,
                -1,
                0,
            )
        };
        if umem_area == libc::MAP_FAILED {
            return Err(anyhow!(
                "Failed to mmap UMEM ({} bytes): {}",
                umem_len,
                std::io::Error::last_os_error()
            ));
        }
        let umem_area = umem_area as *mut u8;

        // Step 2: Create AF_XDP socket
        let fd = unsafe { libc::socket(AF_XDP, libc::SOCK_RAW, 0) };
        if fd < 0 {
            // Clean up UMEM
            unsafe { libc::munmap(umem_area as *mut libc::c_void, umem_len); }
            return Err(anyhow!(
                "Failed to create AF_XDP socket: {}",
                std::io::Error::last_os_error()
            ));
        }

        // From here, cleanup on error must close fd and munmap UMEM.
        // We use a helper closure pattern to avoid repetition.
        let result = (|| -> Result<KernelState> {
            // Step 3: Register UMEM with the socket
            let umem_reg = XdpUmemReg {
                addr: umem_area as u64,
                len: umem_len as u64,
                chunk_size: umem_cfg.frame_size,
                headroom: umem_cfg.frame_headroom,
                flags: 0,
            };
            let ret = unsafe {
                libc::setsockopt(
                    fd,
                    SOL_XDP,
                    XDP_UMEM_REG,
                    &umem_reg as *const XdpUmemReg as *const libc::c_void,
                    std::mem::size_of::<XdpUmemReg>() as libc::socklen_t,
                )
            };
            if ret < 0 {
                return Err(anyhow!(
                    "setsockopt(XDP_UMEM_REG) failed: {}",
                    std::io::Error::last_os_error()
                ));
            }

            // Step 4: Set ring sizes via setsockopt
            set_ring_size(fd, XDP_UMEM_FILL_RING, ring_cfg.fill_size)?;
            set_ring_size(fd, XDP_UMEM_COMPLETION_RING, ring_cfg.comp_size)?;
            set_ring_size(fd, XDP_RX_RING, ring_cfg.rx_size)?;
            set_ring_size(fd, XDP_TX_RING, ring_cfg.tx_size)?;

            // Step 5: Get mmap offsets from the kernel
            let offsets = get_mmap_offsets(fd)?;

            // Step 6: mmap all four ring buffers
            let fill_ring = mmap_ring(
                fd,
                XDP_UMEM_PGOFF_FILL_RING,
                &offsets.fr,
                ring_cfg.fill_size,
            )?;
            let comp_ring = mmap_ring(
                fd,
                XDP_UMEM_PGOFF_COMPLETION_RING,
                &offsets.cr,
                ring_cfg.comp_size,
            )?;
            let rx_ring = mmap_ring(
                fd,
                XDP_PGOFF_RX_RING,
                &offsets.rx,
                ring_cfg.rx_size,
            )?;
            let tx_ring = mmap_ring(
                fd,
                XDP_PGOFF_TX_RING,
                &offsets.tx,
                ring_cfg.tx_size,
            )?;

            // Step 7: Bind socket to interface + queue
            let ifindex = get_ifindex(interface)?;
            let addr = SockaddrXdp {
                sxdp_family: AF_XDP as u16,
                sxdp_flags: 0,
                sxdp_ifindex: ifindex,
                sxdp_queue_id: queue_id,
                sxdp_shared_umem_fd: 0,
            };
            let ret = unsafe {
                libc::bind(
                    fd,
                    &addr as *const SockaddrXdp as *const libc::sockaddr,
                    std::mem::size_of::<SockaddrXdp>() as libc::socklen_t,
                )
            };
            if ret < 0 {
                return Err(anyhow!(
                    "bind(AF_XDP, {}:{}) failed: {}",
                    interface,
                    queue_id,
                    std::io::Error::last_os_error()
                ));
            }

            // Step 8: Create frame allocator
            let allocator = FrameAllocator::new(umem_cfg.frame_count, umem_cfg.frame_size);

            // Step 9: Populate fill ring with initial frames so kernel can receive
            let initial_fill = std::cmp::min(
                ring_cfg.fill_size,
                umem_cfg.frame_count / 2, // Reserve half for TX
            );
            let mut fill_prod = fill_ring.load_producer();
            for _ in 0..initial_fill {
                if let Some(frame_addr) = allocator.allocate() {
                    unsafe {
                        *fill_ring.addr_at(fill_prod) = frame_addr;
                    }
                    fill_prod = fill_prod.wrapping_add(1);
                } else {
                    break;
                }
            }
            fill_ring.store_producer(fill_prod);

            tracing::info!(
                "AF_XDP kernel state ready: fd={}, umem={}MB, fill={}/{}, frames_free={}",
                fd,
                umem_len / (1024 * 1024),
                initial_fill,
                ring_cfg.fill_size,
                allocator.available()
            );

            Ok(KernelState {
                fd,
                umem_area,
                umem_len,
                frame_size: umem_cfg.frame_size,
                headroom: umem_cfg.frame_headroom,
                fill_ring,
                comp_ring,
                rx_ring,
                tx_ring,
                allocator,
            })
        })();

        match result {
            Ok(state) => Ok(state),
            Err(e) => {
                // Cleanup on failure
                unsafe {
                    libc::close(fd);
                    libc::munmap(umem_area as *mut libc::c_void, umem_len);
                }
                Err(e)
            }
        }
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

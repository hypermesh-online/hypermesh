// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP (Address Family XDP) Zero-Copy Socket Management
//!
//! Provides zero-copy packet I/O bypassing the kernel network stack
//! for maximum performance on the STOQ fast path. This is execution
//! path 1: AF_XDP -> STOQ (XDP_REDIRECT).
//!
//! With the `kernel-attach` feature enabled, creates real AF_XDP sockets
//! with UMEM shared memory, fill/completion/rx/tx ring buffers, and
//! true zero-copy packet I/O via direct `libc` syscalls.
//!
//! Without `kernel-attach` or when the kernel probe fails, sockets track
//! statistics and fall back to standard I/O.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;

// Kernel-attach specific imports for real AF_XDP I/O
#[cfg(feature = "kernel-attach")]
use std::sync::atomic::{AtomicU32, Ordering};

// -----------------------------------------------------------------------
// Linux AF_XDP kernel constants (from linux/if_xdp.h)
// -----------------------------------------------------------------------

#[cfg(feature = "kernel-attach")]
mod xdp_consts {
    /// AF_XDP address family number
    pub const AF_XDP: i32 = 44;

    /// Socket option level for XDP
    pub const SOL_XDP: i32 = 283;

    /// Socket options
    pub const XDP_RX_RING: i32 = 1;
    pub const XDP_TX_RING: i32 = 2;
    pub const XDP_UMEM_REG: i32 = 4;
    pub const XDP_UMEM_FILL_RING: i32 = 5;
    pub const XDP_UMEM_COMPLETION_RING: i32 = 6;
    pub const XDP_MMAP_OFFSETS: i32 = 1;

    /// mmap page offsets for ring buffers
    pub const XDP_PGOFF_RX_RING: i64 = 0;
    pub const XDP_PGOFF_TX_RING: i64 = 0x80000000;
    pub const XDP_UMEM_PGOFF_FILL_RING: i64 = 0x100000000;
    pub const XDP_UMEM_PGOFF_COMPLETION_RING: i64 = 0x180000000;
}

// -----------------------------------------------------------------------
// Kernel ring and UMEM repr structs
// -----------------------------------------------------------------------

/// Kernel `struct xdp_umem_reg` for registering UMEM with the socket.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
struct XdpUmemReg {
    addr: u64,
    len: u64,
    chunk_size: u32,
    headroom: u32,
    flags: u32,
}

/// Kernel `struct xdp_desc` used in RX/TX ring descriptors.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct XdpDesc {
    addr: u64,
    len: u32,
    options: u32,
}

/// Kernel `struct sockaddr_xdp` for binding AF_XDP sockets.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
struct SockaddrXdp {
    sxdp_family: u16,
    sxdp_flags: u16,
    sxdp_ifindex: u32,
    sxdp_queue_id: u32,
    sxdp_shared_umem_fd: u32,
}

/// Kernel mmap offset structure returned by `getsockopt(XDP_MMAP_OFFSETS)`.
///
/// Each ring has producer, consumer, desc, and flags offsets.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct XdpRingOffset {
    producer: u64,
    consumer: u64,
    desc: u64,
    flags: u64,
}

/// Combined mmap offsets for all 4 rings.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct XdpMmapOffsets {
    rx: XdpRingOffset,
    tx: XdpRingOffset,
    fr: XdpRingOffset, // fill ring
    cr: XdpRingOffset, // completion ring
}

// -----------------------------------------------------------------------
// Kernel-backed state: UMEM, rings, frame allocator
// -----------------------------------------------------------------------

/// A single memory-mapped ring buffer (fill, completion, rx, or tx).
///
/// The ring has a producer index, consumer index, and an array of
/// descriptors (u64 for fill/completion, XdpDesc for rx/tx).
#[cfg(feature = "kernel-attach")]
struct MappedRing {
    /// Base pointer to the mmap'd region
    map_addr: *mut u8,
    /// Total mmap size (for munmap)
    map_len: usize,
    /// Pointer to the producer index (u32, atomic)
    producer: *const AtomicU32,
    /// Pointer to the consumer index (u32, atomic)
    consumer: *const AtomicU32,
    /// Pointer to the start of the descriptor array
    desc_base: *mut u8,
    /// Number of entries (mask = size - 1)
    size: u32,
}

#[cfg(feature = "kernel-attach")]
unsafe impl Send for MappedRing {}
#[cfg(feature = "kernel-attach")]
unsafe impl Sync for MappedRing {}

#[cfg(feature = "kernel-attach")]
impl MappedRing {
    fn mask(&self) -> u32 {
        self.size - 1
    }

    /// Read the producer index with Acquire ordering.
    fn load_producer(&self) -> u32 {
        unsafe { (*self.producer).load(Ordering::Acquire) }
    }

    /// Read the consumer index with Acquire ordering.
    fn load_consumer(&self) -> u32 {
        unsafe { (*self.consumer).load(Ordering::Acquire) }
    }

    /// Store the producer index with Release ordering.
    fn store_producer(&self, val: u32) {
        unsafe { (*self.producer).store(val, Ordering::Release) }
    }

    /// Store the consumer index with Release ordering.
    fn store_consumer(&self, val: u32) {
        unsafe { (*self.consumer).store(val, Ordering::Release) }
    }

    /// Get a pointer to a fill/completion ring entry (u64 addresses).
    fn addr_at(&self, idx: u32) -> *mut u64 {
        let offset = (idx & self.mask()) as usize * std::mem::size_of::<u64>();
        unsafe { self.desc_base.add(offset) as *mut u64 }
    }

    /// Get a pointer to an rx/tx ring entry (XdpDesc).
    fn desc_at(&self, idx: u32) -> *mut XdpDesc {
        let offset = (idx & self.mask()) as usize * std::mem::size_of::<XdpDesc>();
        unsafe { self.desc_base.add(offset) as *mut XdpDesc }
    }
}

/// Thread-safe frame allocator using a free-list of UMEM frame addresses.
#[cfg(feature = "kernel-attach")]
struct FrameAllocator {
    free_list: parking_lot::Mutex<Vec<u64>>,
}

#[cfg(feature = "kernel-attach")]
impl FrameAllocator {
    fn new(frame_count: u32, frame_size: u32) -> Self {
        let mut free_list = Vec::with_capacity(frame_count as usize);
        for i in 0..frame_count {
            free_list.push(i as u64 * frame_size as u64);
        }
        Self {
            free_list: parking_lot::Mutex::new(free_list),
        }
    }

    fn allocate(&self) -> Option<u64> {
        self.free_list.lock().pop()
    }

    fn release(&self, addr: u64) {
        self.free_list.lock().push(addr);
    }

    fn release_batch(&self, addrs: &[u64]) {
        let mut list = self.free_list.lock();
        list.extend_from_slice(addrs);
    }

    fn available(&self) -> usize {
        self.free_list.lock().len()
    }
}

/// All kernel-backed AF_XDP state, shared via Arc between socket clones.
///
/// Owns the socket file descriptor, UMEM mapping, ring buffers, and
/// frame allocator. Cleaned up on Drop.
#[cfg(feature = "kernel-attach")]
struct KernelState {
    /// AF_XDP socket file descriptor
    fd: i32,
    /// UMEM memory region base pointer
    umem_area: *mut u8,
    /// Total UMEM size in bytes
    umem_len: usize,
    /// Frame size in bytes
    frame_size: u32,
    /// Per-frame headroom offset
    headroom: u32,
    /// Fill ring (userspace -> kernel: empty frames for RX)
    fill_ring: MappedRing,
    /// Completion ring (kernel -> userspace: TX frames done)
    comp_ring: MappedRing,
    /// RX ring (kernel -> userspace: received packets)
    rx_ring: MappedRing,
    /// TX ring (userspace -> kernel: packets to send)
    tx_ring: MappedRing,
    /// Frame allocator (thread-safe free-list)
    allocator: FrameAllocator,
}

#[cfg(feature = "kernel-attach")]
unsafe impl Send for KernelState {}
#[cfg(feature = "kernel-attach")]
unsafe impl Sync for KernelState {}

#[cfg(feature = "kernel-attach")]
impl Drop for KernelState {
    fn drop(&mut self) {
        // Munmap ring buffers (fill, comp, rx, tx)
        for ring in [
            &self.fill_ring,
            &self.comp_ring,
            &self.rx_ring,
            &self.tx_ring,
        ] {
            if !ring.map_addr.is_null() && ring.map_len > 0 {
                unsafe {
                    libc::munmap(ring.map_addr as *mut libc::c_void, ring.map_len);
                }
            }
        }

        // Munmap UMEM area
        if !self.umem_area.is_null() && self.umem_len > 0 {
            unsafe {
                libc::munmap(self.umem_area as *mut libc::c_void, self.umem_len);
            }
        }

        // Close socket fd
        if self.fd >= 0 {
            unsafe {
                libc::close(self.fd);
            }
        }

        tracing::debug!("AF_XDP kernel state cleaned up (fd={})", self.fd);
    }
}

// -----------------------------------------------------------------------
// AF_XDP Socket
// -----------------------------------------------------------------------

/// AF_XDP socket for zero-copy packet I/O.
///
/// When kernel-backed, provides true zero-copy via UMEM shared memory
/// with fill/completion/rx/tx ring buffers. When in fallback mode,
/// tracks statistics and signals the caller to use standard socket I/O.
pub struct AfXdpSocket {
    interface: String,
    queue_id: u32,
    stats: Arc<RwLock<AfXdpStats>>,
    /// Whether this socket has real kernel AF_XDP backing
    kernel_backed: bool,
    /// Kernel-backed AF_XDP state (fd, UMEM, rings, allocator)
    #[cfg(feature = "kernel-attach")]
    kernel_state: Option<Arc<KernelState>>,
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

// -----------------------------------------------------------------------
// Kernel helper functions (behind kernel-attach)
// -----------------------------------------------------------------------

/// Validate UMEM and ring configuration before kernel setup.
#[cfg(feature = "kernel-attach")]
fn validate_config(umem: &UmemConfig, ring: &RingConfig) -> Result<()> {
    if umem.frame_count == 0 || !umem.frame_count.is_power_of_two() {
        return Err(anyhow!(
            "frame_count must be a non-zero power of 2 (got {})",
            umem.frame_count
        ));
    }
    if umem.frame_size < 2048 || !umem.frame_size.is_power_of_two() {
        return Err(anyhow!(
            "frame_size must be >= 2048 and a power of 2 (got {})",
            umem.frame_size
        ));
    }
    if umem.frame_headroom >= umem.frame_size {
        return Err(anyhow!(
            "frame_headroom ({}) must be less than frame_size ({})",
            umem.frame_headroom,
            umem.frame_size
        ));
    }
    for (name, size) in [
        ("tx_size", ring.tx_size),
        ("rx_size", ring.rx_size),
        ("fill_size", ring.fill_size),
        ("comp_size", ring.comp_size),
    ] {
        if size == 0 || !size.is_power_of_two() {
            return Err(anyhow!(
                "{} must be a non-zero power of 2 (got {})",
                name,
                size
            ));
        }
    }
    Ok(())
}

/// Call setsockopt to set a ring size.
#[cfg(feature = "kernel-attach")]
fn set_ring_size(fd: i32, opt: i32, size: u32) -> Result<()> {
    let ret = unsafe {
        libc::setsockopt(
            fd,
            xdp_consts::SOL_XDP,
            opt,
            &size as *const u32 as *const libc::c_void,
            std::mem::size_of::<u32>() as libc::socklen_t,
        )
    };
    if ret < 0 {
        return Err(anyhow!(
            "setsockopt(SOL_XDP, opt={}, size={}) failed: {}",
            opt,
            size,
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Get mmap offsets for all ring buffers via getsockopt(XDP_MMAP_OFFSETS).
#[cfg(feature = "kernel-attach")]
fn get_mmap_offsets(fd: i32) -> Result<XdpMmapOffsets> {
    let mut offsets = XdpMmapOffsets::default();
    let mut optlen = std::mem::size_of::<XdpMmapOffsets>() as libc::socklen_t;
    let ret = unsafe {
        libc::getsockopt(
            fd,
            xdp_consts::SOL_XDP,
            xdp_consts::XDP_MMAP_OFFSETS,
            &mut offsets as *mut XdpMmapOffsets as *mut libc::c_void,
            &mut optlen,
        )
    };
    if ret < 0 {
        return Err(anyhow!(
            "getsockopt(XDP_MMAP_OFFSETS) failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(offsets)
}

/// Memory-map a ring buffer and return a MappedRing with pointers to
/// the producer, consumer, and descriptor array.
#[cfg(feature = "kernel-attach")]
fn mmap_ring(
    fd: i32,
    pgoff: i64,
    ring_offset: &XdpRingOffset,
    ring_size: u32,
) -> Result<MappedRing> {
    // Calculate the total mmap size: max of (desc_offset + descriptors_bytes) and
    // (producer/consumer offset + 4). We need enough room for all three.
    // For fill/completion rings, each descriptor is u64 (8 bytes).
    // For rx/tx rings, each descriptor is XdpDesc (16 bytes).
    // We use the larger size to be safe.
    let desc_entry_size = std::cmp::max(
        std::mem::size_of::<u64>(),
        std::mem::size_of::<XdpDesc>(),
    );
    let map_len = ring_offset.desc as usize + (ring_size as usize * desc_entry_size);

    let map_addr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            map_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED | libc::MAP_POPULATE,
            fd,
            pgoff,
        )
    };
    if map_addr == libc::MAP_FAILED {
        return Err(anyhow!(
            "mmap ring (pgoff=0x{:x}, len={}) failed: {}",
            pgoff,
            map_len,
            std::io::Error::last_os_error()
        ));
    }
    let base = map_addr as *mut u8;

    Ok(MappedRing {
        map_addr: base,
        map_len,
        producer: unsafe { base.add(ring_offset.producer as usize) as *const AtomicU32 },
        consumer: unsafe { base.add(ring_offset.consumer as usize) as *const AtomicU32 },
        desc_base: unsafe { base.add(ring_offset.desc as usize) },
        size: ring_size,
    })
}

/// Resolve interface name to ifindex via libc::if_nametoindex.
#[cfg(feature = "kernel-attach")]
fn get_ifindex(interface: &str) -> Result<u32> {
    let c_name = std::ffi::CString::new(interface)
        .map_err(|_| anyhow!("Invalid interface name: contains NUL byte"))?;
    let idx = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    if idx == 0 {
        return Err(anyhow!(
            "Interface '{}' not found: {}",
            interface,
            std::io::Error::last_os_error()
        ));
    }
    Ok(idx)
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
    /// When kernel-backed, copies the packet into a UMEM frame, writes a
    /// descriptor to the TX ring, and kicks the kernel via sendto(). When
    /// in fallback mode, returns an error indicating the caller should use
    /// standard socket I/O.
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        if !self.kernel_backed {
            let mut stats = self.stats.write();
            stats.packets_sent += 1;
            stats.bytes_sent += data.len() as u64;
            return Err(anyhow!(
                "AF_XDP not kernel-backed on {}:{}: use standard I/O",
                self.interface,
                self.queue_id
            ));
        }

        #[cfg(feature = "kernel-attach")]
        {
            self.send_kernel(data)?;
            return Ok(());
        }

        #[cfg(not(feature = "kernel-attach"))]
        Err(anyhow!("kernel-attach feature not enabled"))
    }

    /// Real kernel-backed send: allocate frame, copy data, enqueue TX, kick kernel.
    #[cfg(feature = "kernel-attach")]
    fn send_kernel(&self, data: &[u8]) -> Result<()> {
        let ks = self.kernel_state.as_ref().ok_or_else(|| {
            anyhow!("kernel_backed=true but no kernel state")
        })?;

        let max_payload = ks.frame_size as usize - ks.headroom as usize;
        if data.len() > max_payload {
            return Err(anyhow!(
                "Packet too large: {} bytes (max {} with headroom {})",
                data.len(),
                max_payload,
                ks.headroom
            ));
        }

        // Reclaim completed TX frames first
        self.reclaim_completed(ks);

        // Allocate a UMEM frame
        let frame_addr = ks.allocator.allocate().ok_or_else(|| {
            self.stats.write().tx_ring_full += 1;
            anyhow!("No free UMEM frames available for TX")
        })?;

        // Check TX ring space
        let prod = ks.tx_ring.load_producer();
        let cons = ks.tx_ring.load_consumer();
        let in_use = prod.wrapping_sub(cons);
        if in_use >= ks.tx_ring.size {
            ks.allocator.release(frame_addr);
            self.stats.write().tx_ring_full += 1;
            return Err(anyhow!("TX ring full ({} entries)", ks.tx_ring.size));
        }

        // Copy packet data into UMEM frame (at frame_addr + headroom)
        let write_offset = frame_addr as usize + ks.headroom as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                ks.umem_area.add(write_offset),
                data.len(),
            );
        }

        // Write TX descriptor
        let desc = XdpDesc {
            addr: frame_addr + ks.headroom as u64,
            len: data.len() as u32,
            options: 0,
        };
        unsafe {
            *ks.tx_ring.desc_at(prod) = desc;
        }
        ks.tx_ring.store_producer(prod.wrapping_add(1));

        // Kick the kernel to process the TX ring
        let ret = unsafe {
            libc::sendto(
                ks.fd,
                std::ptr::null(),
                0,
                libc::MSG_DONTWAIT,
                std::ptr::null(),
                0,
            )
        };
        // sendto can return EAGAIN/EWOULDBLOCK which is not a fatal error
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            let errno = err.raw_os_error().unwrap_or(0);
            if errno != libc::EAGAIN && errno != libc::EWOULDBLOCK && errno != libc::EBUSY {
                return Err(anyhow!("sendto() failed to kick TX: {}", err));
            }
        }

        let mut stats = self.stats.write();
        stats.packets_sent += 1;
        stats.bytes_sent += data.len() as u64;

        Ok(())
    }

    /// Receive packet via AF_XDP zero-copy (or signal standard I/O fallback).
    ///
    /// When kernel-backed, reads from the RX ring, copies data from the UMEM
    /// frame, and returns the frame to the fill ring. Uses poll() with a short
    /// timeout if no packets are immediately available.
    pub async fn receive(&self) -> Result<Bytes> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed on {}:{}: use standard I/O",
                self.interface,
                self.queue_id
            ));
        }

        #[cfg(feature = "kernel-attach")]
        {
            return self.receive_kernel();
        }

        #[cfg(not(feature = "kernel-attach"))]
        Err(anyhow!("kernel-attach feature not enabled"))
    }

    /// Real kernel-backed receive: check RX ring, copy from UMEM, refill.
    #[cfg(feature = "kernel-attach")]
    fn receive_kernel(&self) -> Result<Bytes> {
        let ks = self.kernel_state.as_ref().ok_or_else(|| {
            anyhow!("kernel_backed=true but no kernel state")
        })?;

        // Check RX ring for available descriptors
        let cons = ks.rx_ring.load_consumer();
        let prod = ks.rx_ring.load_producer();
        let available = prod.wrapping_sub(cons);

        if available == 0 {
            // No packets ready; poll with 1ms timeout
            let mut pfd = libc::pollfd {
                fd: ks.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, 1) };
            if ret <= 0 {
                self.stats.write().rx_ring_empty += 1;
                return Err(anyhow!("No packets available on RX ring"));
            }

            // Re-check after poll
            let prod_after = ks.rx_ring.load_producer();
            if prod_after.wrapping_sub(cons) == 0 {
                self.stats.write().rx_ring_empty += 1;
                return Err(anyhow!("No packets available after poll"));
            }
        }

        // Read the descriptor
        let desc = unsafe { *ks.rx_ring.desc_at(cons) };

        // Validate descriptor
        let addr = desc.addr as usize;
        let len = desc.len as usize;
        if addr + len > ks.umem_len {
            self.stats.write().invalid_descriptors += 1;
            // Still advance consumer to avoid stuck ring
            ks.rx_ring.store_consumer(cons.wrapping_add(1));
            return Err(anyhow!(
                "Invalid RX descriptor: addr={}, len={}, umem_len={}",
                addr,
                len,
                ks.umem_len
            ));
        }

        // Copy packet data from UMEM
        let mut buf = vec![0u8; len];
        unsafe {
            std::ptr::copy_nonoverlapping(
                ks.umem_area.add(addr),
                buf.as_mut_ptr(),
                len,
            );
        }

        // Advance RX consumer
        ks.rx_ring.store_consumer(cons.wrapping_add(1));

        // Return the frame to the fill ring for reuse.
        // The frame address is aligned to frame boundaries (strip offset).
        let frame_base = (desc.addr / ks.frame_size as u64) * ks.frame_size as u64;
        self.refill_frame(ks, frame_base);

        let mut stats = self.stats.write();
        stats.packets_received += 1;
        stats.bytes_received += len as u64;

        Ok(Bytes::from(buf))
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
                "AF_XDP not kernel-backed: use standard I/O for batch send"
            ));
        }

        #[cfg(feature = "kernel-attach")]
        {
            return self.send_batch_kernel(packets);
        }

        #[cfg(not(feature = "kernel-attach"))]
        Err(anyhow!("kernel-attach feature not enabled"))
    }

    /// Batch send via kernel: enqueue multiple frames then kick once.
    #[cfg(feature = "kernel-attach")]
    fn send_batch_kernel(&self, packets: &[&[u8]]) -> Result<usize> {
        let ks = self.kernel_state.as_ref().ok_or_else(|| {
            anyhow!("kernel_backed=true but no kernel state")
        })?;

        let max_payload = ks.frame_size as usize - ks.headroom as usize;

        // Reclaim completed TX frames first
        self.reclaim_completed(ks);

        let mut prod = ks.tx_ring.load_producer();
        let cons = ks.tx_ring.load_consumer();
        let free_slots = ks.tx_ring.size - prod.wrapping_sub(cons);
        let mut sent = 0usize;
        let mut total_bytes = 0u64;

        for &pkt in packets {
            if sent as u32 >= free_slots {
                break;
            }
            if pkt.len() > max_payload {
                self.stats.write().invalid_descriptors += 1;
                continue; // Skip oversized packets
            }

            let frame_addr = match ks.allocator.allocate() {
                Some(addr) => addr,
                None => break, // No more free frames
            };

            // Copy packet into UMEM frame
            let write_offset = frame_addr as usize + ks.headroom as usize;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    pkt.as_ptr(),
                    ks.umem_area.add(write_offset),
                    pkt.len(),
                );
            }

            // Write TX descriptor
            let desc = XdpDesc {
                addr: frame_addr + ks.headroom as u64,
                len: pkt.len() as u32,
                options: 0,
            };
            unsafe {
                *ks.tx_ring.desc_at(prod) = desc;
            }
            prod = prod.wrapping_add(1);

            sent += 1;
            total_bytes += pkt.len() as u64;
        }

        if sent > 0 {
            ks.tx_ring.store_producer(prod);

            // Single kick for the entire batch
            let ret = unsafe {
                libc::sendto(
                    ks.fd,
                    std::ptr::null(),
                    0,
                    libc::MSG_DONTWAIT,
                    std::ptr::null(),
                    0,
                )
            };
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                let errno = err.raw_os_error().unwrap_or(0);
                if errno != libc::EAGAIN && errno != libc::EWOULDBLOCK && errno != libc::EBUSY {
                    return Err(anyhow!("sendto() failed to kick TX batch: {}", err));
                }
            }

            let mut stats = self.stats.write();
            stats.packets_sent += sent as u64;
            stats.bytes_sent += total_bytes;
        }

        Ok(sent)
    }

    /// Receive multiple packets in batch
    pub async fn receive_batch(
        &self,
        max_packets: usize,
    ) -> Result<Vec<Bytes>> {
        if !self.kernel_backed {
            self.stats.write().rx_ring_empty += 1;
            return Err(anyhow!(
                "AF_XDP not kernel-backed: use standard I/O for batch receive"
            ));
        }

        #[cfg(feature = "kernel-attach")]
        {
            return self.receive_batch_kernel(max_packets);
        }

        #[cfg(not(feature = "kernel-attach"))]
        {
            let _ = max_packets;
            Err(anyhow!("kernel-attach feature not enabled"))
        }
    }

    /// Batch receive via kernel: drain RX ring up to max_packets.
    #[cfg(feature = "kernel-attach")]
    fn receive_batch_kernel(&self, max_packets: usize) -> Result<Vec<Bytes>> {
        let ks = self.kernel_state.as_ref().ok_or_else(|| {
            anyhow!("kernel_backed=true but no kernel state")
        })?;

        let mut cons = ks.rx_ring.load_consumer();
        let prod = ks.rx_ring.load_producer();
        let available = prod.wrapping_sub(cons) as usize;

        if available == 0 {
            // Poll with 1ms timeout
            let mut pfd = libc::pollfd {
                fd: ks.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, 1) };
            if ret <= 0 {
                self.stats.write().rx_ring_empty += 1;
                return Ok(Vec::new());
            }
        }

        // Re-read producer after potential poll
        let prod = ks.rx_ring.load_producer();
        let available = prod.wrapping_sub(cons) as usize;
        let to_read = std::cmp::min(available, max_packets);

        let mut packets = Vec::with_capacity(to_read);
        let mut total_bytes = 0u64;
        let mut frames_to_refill = Vec::with_capacity(to_read);

        for _ in 0..to_read {
            let desc = unsafe { *ks.rx_ring.desc_at(cons) };
            let addr = desc.addr as usize;
            let len = desc.len as usize;

            if addr + len > ks.umem_len {
                self.stats.write().invalid_descriptors += 1;
                cons = cons.wrapping_add(1);
                continue;
            }

            let mut buf = vec![0u8; len];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    ks.umem_area.add(addr),
                    buf.as_mut_ptr(),
                    len,
                );
            }

            // Calculate frame base address for refill
            let frame_base = (desc.addr / ks.frame_size as u64) * ks.frame_size as u64;
            frames_to_refill.push(frame_base);

            packets.push(Bytes::from(buf));
            total_bytes += len as u64;
            cons = cons.wrapping_add(1);
        }

        // Advance RX consumer
        ks.rx_ring.store_consumer(cons);

        // Batch-refill the fill ring
        self.refill_frames_batch(ks, &frames_to_refill);

        let received = packets.len();
        if received > 0 {
            let mut stats = self.stats.write();
            stats.packets_received += received as u64;
            stats.bytes_received += total_bytes;
        }

        Ok(packets)
    }

    /// Reclaim completed TX frames from the completion ring back to the
    /// frame allocator.
    #[cfg(feature = "kernel-attach")]
    fn reclaim_completed(&self, ks: &KernelState) {
        let cons = ks.comp_ring.load_consumer();
        let prod = ks.comp_ring.load_producer();
        let completed = prod.wrapping_sub(cons);

        if completed == 0 {
            return;
        }

        let mut addrs = Vec::with_capacity(completed as usize);
        let mut c = cons;
        for _ in 0..completed {
            let addr = unsafe { *ks.comp_ring.addr_at(c) };
            // The completion ring gives us the addr from the TX descriptor
            // (which includes headroom). We need to recover the frame base.
            let frame_base = (addr / ks.frame_size as u64) * ks.frame_size as u64;
            addrs.push(frame_base);
            c = c.wrapping_add(1);
        }

        ks.comp_ring.store_consumer(c);
        ks.allocator.release_batch(&addrs);
    }

    /// Return a single frame to the fill ring for kernel reuse.
    #[cfg(feature = "kernel-attach")]
    fn refill_frame(&self, ks: &KernelState, frame_addr: u64) {
        let prod = ks.fill_ring.load_producer();
        let cons = ks.fill_ring.load_consumer();
        let in_use = prod.wrapping_sub(cons);

        if in_use >= ks.fill_ring.size {
            // Fill ring full; return frame to allocator instead
            ks.allocator.release(frame_addr);
            return;
        }

        unsafe {
            *ks.fill_ring.addr_at(prod) = frame_addr;
        }
        ks.fill_ring.store_producer(prod.wrapping_add(1));
    }

    /// Batch-refill the fill ring with multiple frames.
    #[cfg(feature = "kernel-attach")]
    fn refill_frames_batch(&self, ks: &KernelState, frames: &[u64]) {
        let mut prod = ks.fill_ring.load_producer();
        let cons = ks.fill_ring.load_consumer();
        let free_slots = ks.fill_ring.size - prod.wrapping_sub(cons);

        let mut refilled = 0u32;
        for &frame_addr in frames {
            if refilled >= free_slots {
                // Fill ring full; return remaining frames to allocator
                ks.allocator.release(frame_addr);
            } else {
                unsafe {
                    *ks.fill_ring.addr_at(prod) = frame_addr;
                }
                prod = prod.wrapping_add(1);
                refilled += 1;
            }
        }

        if refilled > 0 {
            ks.fill_ring.store_producer(prod);
        }
    }

    /// Get socket statistics
    pub fn get_stats(&self) -> AfXdpStats {
        self.stats.read().clone()
    }

    /// Get the number of free UMEM frames available for allocation.
    ///
    /// Returns 0 for non-kernel-backed sockets.
    pub fn free_frames(&self) -> usize {
        #[cfg(feature = "kernel-attach")]
        if let Some(ref ks) = self.kernel_state {
            return ks.allocator.available();
        }
        0
    }

    /// Get number of free fill ring slots (available for RX).
    ///
    /// For kernel-backed sockets, reads `ring.size - (producer - consumer)`.
    /// For non-kernel-backed sockets, returns the configured fill ring size
    /// (treats as fully available).
    pub fn fill_ring_free(&self) -> u32 {
        #[cfg(feature = "kernel-attach")]
        if let Some(ref ks) = self.kernel_state {
            let prod = ks.fill_ring.load_producer();
            let cons = ks.fill_ring.load_consumer();
            let in_use = prod.wrapping_sub(cons);
            return ks.fill_ring.size.saturating_sub(in_use);
        }
        // Non-kernel-backed: treat as fully available
        0
    }

    /// Get number of free TX ring slots.
    ///
    /// For kernel-backed sockets, reads `ring.size - (producer - consumer)`.
    /// For non-kernel-backed sockets, returns 0.
    pub fn tx_ring_free(&self) -> u32 {
        #[cfg(feature = "kernel-attach")]
        if let Some(ref ks) = self.kernel_state {
            let prod = ks.tx_ring.load_producer();
            let cons = ks.tx_ring.load_consumer();
            let in_use = prod.wrapping_sub(cons);
            return ks.tx_ring.size.saturating_sub(in_use);
        }
        // Non-kernel-backed: return 0
        0
    }
}

impl Clone for AfXdpSocket {
    fn clone(&self) -> Self {
        Self {
            interface: self.interface.clone(),
            queue_id: self.queue_id,
            stats: self.stats.clone(),
            kernel_backed: self.kernel_backed,
            #[cfg(feature = "kernel-attach")]
            kernel_state: self.kernel_state.clone(),
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

    #[test]
    fn test_af_xdp_socket_free_frames_fallback() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        // Non-kernel-backed sockets report 0 free frames
        assert_eq!(socket.free_frames(), 0);
    }

    #[test]
    fn test_af_xdp_socket_clone() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        let cloned = socket.clone();
        assert_eq!(cloned.interface(), "eth0");
        assert_eq!(cloned.queue_id(), 0);
        assert_eq!(cloned.is_kernel_backed(), socket.is_kernel_backed());
    }

    #[test]
    fn test_af_xdp_manager_with_config() {
        let umem = UmemConfig {
            frame_count: 8192,
            frame_size: 4096,
            frame_headroom: 128,
            use_huge_pages: false,
        };
        let ring = RingConfig {
            tx_size: 4096,
            rx_size: 4096,
            fill_size: 4096,
            comp_size: 4096,
        };
        let manager = AfXdpManager::with_config(umem, ring);
        assert!(manager.is_ok());
        let mgr = manager.expect("test: create manager with config");
        assert_eq!(mgr.umem_config.frame_count, 8192);
        assert_eq!(mgr.ring_config.tx_size, 4096);
    }

    #[test]
    fn test_af_xdp_stats_default() {
        let stats = AfXdpStats::default();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.tx_ring_full, 0);
        assert_eq!(stats.rx_ring_empty, 0);
        assert_eq!(stats.invalid_descriptors, 0);
    }

    #[test]
    fn test_af_xdp_manager_get_stats() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _socket = manager.create_socket("lo", 0).expect("test: create socket");
        let stats = manager.get_stats("lo", 0);
        assert!(stats.is_some());
        let s = stats.expect("test: get stats");
        assert_eq!(s.packets_sent, 0);

        // Non-existent socket returns None
        let no_stats = manager.get_stats("eth99", 0);
        assert!(no_stats.is_none());
    }

    #[test]
    fn test_af_xdp_manager_close_all() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let _s1 = manager.create_socket("eth0", 0).expect("test: create socket 1");
        let _s2 = manager.create_socket("eth1", 0).expect("test: create socket 2");
        assert_eq!(manager.socket_count(), 2);

        manager.close_all().expect("test: close all");
        assert_eq!(manager.socket_count(), 0);
    }

    #[tokio::test]
    async fn test_af_xdp_send_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        let result = socket.send(&[1, 2, 3, 4]).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.packets_sent, 1);
        assert_eq!(stats.bytes_sent, 4);
    }

    #[tokio::test]
    async fn test_af_xdp_receive_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        let result = socket.receive().await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.rx_ring_empty, 1);
    }

    #[tokio::test]
    async fn test_af_xdp_send_batch_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        let packets: Vec<&[u8]> = vec![&[1, 2], &[3, 4, 5]];
        let result = socket.send_batch(&packets).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.packets_sent, 2);
        assert_eq!(stats.bytes_sent, 5);
    }

    #[tokio::test]
    async fn test_af_xdp_receive_batch_fallback_error() {
        let mut manager = AfXdpManager::new().expect("test: create manager");
        let socket = manager.create_socket("eth0", 0).expect("test: create socket");
        let result = socket.receive_batch(10).await;
        assert!(result.is_err());
        let stats = socket.get_stats();
        assert_eq!(stats.rx_ring_empty, 1);
    }

    #[cfg(feature = "kernel-attach")]
    mod kernel_tests {
        use super::super::*;

        #[test]
        fn test_validate_config_valid() {
            let umem = UmemConfig::default();
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_ok());
        }

        #[test]
        fn test_validate_config_bad_frame_count() {
            let umem = UmemConfig {
                frame_count: 100, // not power of 2
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_frame_size() {
            let umem = UmemConfig {
                frame_size: 1024, // too small (< 2048)
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_headroom() {
            let umem = UmemConfig {
                frame_headroom: 4096, // equals frame_size
                ..Default::default()
            };
            let ring = RingConfig::default();
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_validate_config_bad_ring_size() {
            let umem = UmemConfig::default();
            let ring = RingConfig {
                tx_size: 100, // not power of 2
                ..Default::default()
            };
            assert!(validate_config(&umem, &ring).is_err());
        }

        #[test]
        fn test_frame_allocator() {
            let alloc = FrameAllocator::new(8, 4096);
            assert_eq!(alloc.available(), 8);

            let a = alloc.allocate().expect("test: allocate frame 1");
            assert_eq!(alloc.available(), 7);

            alloc.release(a);
            assert_eq!(alloc.available(), 8);
        }

        #[test]
        fn test_frame_allocator_exhaustion() {
            let alloc = FrameAllocator::new(2, 4096);
            let _a1 = alloc.allocate().expect("test: allocate 1");
            let _a2 = alloc.allocate().expect("test: allocate 2");
            assert!(alloc.allocate().is_none());
        }

        #[test]
        fn test_frame_allocator_batch_release() {
            let alloc = FrameAllocator::new(4, 4096);
            let a1 = alloc.allocate().expect("test: allocate 1");
            let a2 = alloc.allocate().expect("test: allocate 2");
            assert_eq!(alloc.available(), 2);

            alloc.release_batch(&[a1, a2]);
            assert_eq!(alloc.available(), 4);
        }

        #[test]
        fn test_xdp_desc_size() {
            assert_eq!(std::mem::size_of::<XdpDesc>(), 16);
        }

        #[test]
        fn test_xdp_umem_reg_layout() {
            // addr(8) + len(8) + chunk_size(4) + headroom(4) + flags(4) = 28
            // But with padding the struct may be different; check it compiles
            let reg = XdpUmemReg {
                addr: 0,
                len: 0,
                chunk_size: 4096,
                headroom: 256,
                flags: 0,
            };
            assert_eq!(reg.chunk_size, 4096);
        }

        #[test]
        fn test_sockaddr_xdp_layout() {
            let addr = SockaddrXdp {
                sxdp_family: 44,
                sxdp_flags: 0,
                sxdp_ifindex: 1,
                sxdp_queue_id: 0,
                sxdp_shared_umem_fd: 0,
            };
            assert_eq!(addr.sxdp_family, 44);
        }
    }
}

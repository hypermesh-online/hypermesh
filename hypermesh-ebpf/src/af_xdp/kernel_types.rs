// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP kernel-level types: constants, repr structs, ring buffers,
//! frame allocator, and kernel state management.
//!
//! All types in this module are gated behind `#[cfg(feature = "kernel-attach")]`
//! except where noted. They provide the low-level kernel interface for
//! true zero-copy AF_XDP I/O via direct `libc` syscalls.

// Kernel-attach specific imports for real AF_XDP I/O
#[cfg(feature = "kernel-attach")]
use std::sync::atomic::{AtomicU32, Ordering};

// -----------------------------------------------------------------------
// Linux AF_XDP kernel constants (from linux/if_xdp.h)
// -----------------------------------------------------------------------

#[cfg(feature = "kernel-attach")]
pub(crate) mod xdp_consts {
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
pub(crate) struct XdpUmemReg {
    pub addr: u64,
    pub len: u64,
    pub chunk_size: u32,
    pub headroom: u32,
    pub flags: u32,
}

/// Kernel `struct xdp_desc` used in RX/TX ring descriptors.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct XdpDesc {
    pub addr: u64,
    pub len: u32,
    pub options: u32,
}

/// Kernel `struct sockaddr_xdp` for binding AF_XDP sockets.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
pub(crate) struct SockaddrXdp {
    pub sxdp_family: u16,
    pub sxdp_flags: u16,
    pub sxdp_ifindex: u32,
    pub sxdp_queue_id: u32,
    pub sxdp_shared_umem_fd: u32,
}

/// Kernel mmap offset structure returned by `getsockopt(XDP_MMAP_OFFSETS)`.
///
/// Each ring has producer, consumer, desc, and flags offsets.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct XdpRingOffset {
    pub producer: u64,
    pub consumer: u64,
    pub desc: u64,
    pub flags: u64,
}

/// Combined mmap offsets for all 4 rings.
#[cfg(feature = "kernel-attach")]
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct XdpMmapOffsets {
    pub rx: XdpRingOffset,
    pub tx: XdpRingOffset,
    pub fr: XdpRingOffset, // fill ring
    pub cr: XdpRingOffset, // completion ring
}

// -----------------------------------------------------------------------
// Kernel-backed state: UMEM, rings, frame allocator
// -----------------------------------------------------------------------

/// A single memory-mapped ring buffer (fill, completion, rx, or tx).
///
/// The ring has a producer index, consumer index, and an array of
/// descriptors (u64 for fill/completion, XdpDesc for rx/tx).
#[cfg(feature = "kernel-attach")]
pub(crate) struct MappedRing {
    /// Base pointer to the mmap'd region
    pub map_addr: *mut u8,
    /// Total mmap size (for munmap)
    pub map_len: usize,
    /// Pointer to the producer index (u32, atomic)
    pub producer: *const AtomicU32,
    /// Pointer to the consumer index (u32, atomic)
    pub consumer: *const AtomicU32,
    /// Pointer to the start of the descriptor array
    pub desc_base: *mut u8,
    /// Number of entries (mask = size - 1)
    pub size: u32,
}

#[cfg(feature = "kernel-attach")]
unsafe impl Send for MappedRing {}
#[cfg(feature = "kernel-attach")]
unsafe impl Sync for MappedRing {}

#[cfg(feature = "kernel-attach")]
impl MappedRing {
    pub fn mask(&self) -> u32 {
        self.size - 1
    }

    /// Read the producer index with Acquire ordering.
    pub fn load_producer(&self) -> u32 {
        unsafe { (*self.producer).load(Ordering::Acquire) }
    }

    /// Read the consumer index with Acquire ordering.
    pub fn load_consumer(&self) -> u32 {
        unsafe { (*self.consumer).load(Ordering::Acquire) }
    }

    /// Store the producer index with Release ordering.
    pub fn store_producer(&self, val: u32) {
        unsafe { (*self.producer).store(val, Ordering::Release) }
    }

    /// Store the consumer index with Release ordering.
    pub fn store_consumer(&self, val: u32) {
        unsafe { (*self.consumer).store(val, Ordering::Release) }
    }

    /// Get a pointer to a fill/completion ring entry (u64 addresses).
    pub fn addr_at(&self, idx: u32) -> *mut u64 {
        let offset = (idx & self.mask()) as usize * std::mem::size_of::<u64>();
        unsafe { self.desc_base.add(offset) as *mut u64 }
    }

    /// Get a pointer to an rx/tx ring entry (XdpDesc).
    pub fn desc_at(&self, idx: u32) -> *mut XdpDesc {
        let offset = (idx & self.mask()) as usize * std::mem::size_of::<XdpDesc>();
        unsafe { self.desc_base.add(offset) as *mut XdpDesc }
    }
}

/// Thread-safe frame allocator using a free-list of UMEM frame addresses.
#[cfg(feature = "kernel-attach")]
pub(crate) struct FrameAllocator {
    free_list: parking_lot::Mutex<Vec<u64>>,
}

#[cfg(feature = "kernel-attach")]
impl FrameAllocator {
    pub fn new(frame_count: u32, frame_size: u32) -> Self {
        let mut free_list = Vec::with_capacity(frame_count as usize);
        for i in 0..frame_count {
            free_list.push(i as u64 * frame_size as u64);
        }
        Self {
            free_list: parking_lot::Mutex::new(free_list),
        }
    }

    pub fn allocate(&self) -> Option<u64> {
        self.free_list.lock().pop()
    }

    pub fn release(&self, addr: u64) {
        self.free_list.lock().push(addr);
    }

    pub fn release_batch(&self, addrs: &[u64]) {
        let mut list = self.free_list.lock();
        list.extend_from_slice(addrs);
    }

    pub fn available(&self) -> usize {
        self.free_list.lock().len()
    }
}

/// All kernel-backed AF_XDP state, shared via Arc between socket clones.
///
/// Owns the socket file descriptor, UMEM mapping, ring buffers, and
/// frame allocator. Cleaned up on Drop.
#[cfg(feature = "kernel-attach")]
pub(crate) struct KernelState {
    /// AF_XDP socket file descriptor
    pub fd: i32,
    /// UMEM memory region base pointer
    pub umem_area: *mut u8,
    /// Total UMEM size in bytes
    pub umem_len: usize,
    /// Frame size in bytes
    pub frame_size: u32,
    /// Per-frame headroom offset
    pub headroom: u32,
    /// Fill ring (userspace -> kernel: empty frames for RX)
    pub fill_ring: MappedRing,
    /// Completion ring (kernel -> userspace: TX frames done)
    pub comp_ring: MappedRing,
    /// RX ring (kernel -> userspace: received packets)
    pub rx_ring: MappedRing,
    /// TX ring (userspace -> kernel: packets to send)
    pub tx_ring: MappedRing,
    /// Frame allocator (thread-safe free-list)
    pub allocator: FrameAllocator,
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

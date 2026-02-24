// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Kernel helper functions for AF_XDP socket setup.
//!
//! These functions are gated behind `#[cfg(feature = "kernel-attach")]` and
//! handle UMEM/ring configuration validation, setsockopt calls, mmap ring
//! buffer setup, and interface name resolution.

#[cfg(feature = "kernel-attach")]
use anyhow::{Result, anyhow};

#[cfg(feature = "kernel-attach")]
use std::sync::atomic::AtomicU32;

#[cfg(feature = "kernel-attach")]
use super::kernel_types::*;

#[cfg(feature = "kernel-attach")]
use super::manager::{UmemConfig, RingConfig};

// -----------------------------------------------------------------------
// Kernel helper functions (behind kernel-attach)
// -----------------------------------------------------------------------

/// Validate UMEM and ring configuration before kernel setup.
#[cfg(feature = "kernel-attach")]
pub(crate) fn validate_config(umem: &UmemConfig, ring: &RingConfig) -> Result<()> {
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
pub(super) fn set_ring_size(fd: i32, opt: i32, size: u32) -> Result<()> {
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
pub(super) fn get_mmap_offsets(fd: i32) -> Result<XdpMmapOffsets> {
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
pub(super) fn mmap_ring(
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
pub(super) fn get_ifindex(interface: &str) -> Result<u32> {
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

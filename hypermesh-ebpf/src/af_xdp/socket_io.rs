// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP socket single-packet I/O: send, receive, and accessors.

use anyhow::{Result, anyhow};
use bytes::Bytes;

#[cfg(feature = "kernel-attach")]
use super::kernel_types::{KernelState, XdpDesc};

use super::manager::{AfXdpSocket, AfXdpStats};

// -----------------------------------------------------------------------
// AF_XDP Socket single-packet operations
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

    /// Reclaim completed TX frames from the completion ring back to the
    /// frame allocator.
    #[cfg(feature = "kernel-attach")]
    pub(super) fn reclaim_completed(&self, ks: &KernelState) {
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
    pub(super) fn refill_frame(&self, ks: &KernelState, frame_addr: u64) {
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
    pub(super) fn refill_frames_batch(&self, ks: &KernelState, frames: &[u64]) {
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

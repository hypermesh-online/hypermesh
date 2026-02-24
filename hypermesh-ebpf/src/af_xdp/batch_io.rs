// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AF_XDP batch packet I/O: send_batch, receive_batch, and batch kernel helpers.

use anyhow::{Result, anyhow};
use bytes::Bytes;

#[cfg(feature = "kernel-attach")]
use super::kernel_types::{KernelState, XdpDesc};

use super::manager::AfXdpSocket;

// -----------------------------------------------------------------------
// AF_XDP Socket batch operations
// -----------------------------------------------------------------------

impl AfXdpSocket {
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
}

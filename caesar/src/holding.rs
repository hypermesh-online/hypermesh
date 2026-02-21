// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Holding buffer (orbit buffer) for stalled packets.
//!
//! When a packet cannot proceed -- fee budget exceeded during a surge, no
//! route available, or network congestion -- it enters the holding buffer
//! for later retry.

use chrono::{DateTime, Utc};
use hypermesh_lib::economic::PacketId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from holding buffer operations.
#[derive(Debug, thiserror::Error)]
pub enum HoldingError {
    #[error("packet {0} not found in holding buffer")]
    NotFound(PacketId),

    #[error("max retries ({max}) exceeded for packet")]
    MaxRetriesExceeded { max: u32 },
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Why a packet was placed in the holding buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HoldReason {
    /// The next-hop fee exceeds the packet's fee budget.
    FeeBudgetExceeded,
    /// The network path is congested.
    NetworkCongestion,
    /// No viable route to the destination.
    NoRouteAvailable,
}

/// A packet currently in the holding buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeldPacket {
    pub packet_id: PacketId,
    pub held_at: DateTime<Utc>,
    pub reason: HoldReason,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_retry: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// HoldingBuffer
// ---------------------------------------------------------------------------

/// In-memory orbit buffer for packets awaiting retry.
#[derive(Debug)]
pub struct HoldingBuffer {
    held: HashMap<PacketId, HeldPacket>,
    max_retries: u32,
}

impl HoldingBuffer {
    /// Create a new holding buffer with the given maximum retry count.
    pub fn new(max_retries: u32) -> Self {
        Self {
            held: HashMap::new(),
            max_retries,
        }
    }

    /// Place a packet into the holding buffer.
    pub fn hold(&mut self, packet_id: PacketId, reason: HoldReason) {
        let entry = HeldPacket {
            packet_id,
            held_at: Utc::now(),
            reason,
            retry_count: 0,
            max_retries: self.max_retries,
            last_retry: None,
        };
        self.held.insert(packet_id, entry);
    }

    /// Remove a packet from the holding buffer (e.g., when it can proceed).
    pub fn release(&mut self, packet_id: &PacketId) -> Option<HeldPacket> {
        self.held.remove(packet_id)
    }

    /// Record a retry attempt for a held packet.
    ///
    /// Returns the updated entry on success. Errors if the packet is not
    /// found or if `max_retries` has been exceeded.
    pub fn retry(&mut self, packet_id: &PacketId) -> Result<&HeldPacket, HoldingError> {
        let entry = self
            .held
            .get_mut(packet_id)
            .ok_or_else(|| HoldingError::NotFound(*packet_id))?;

        if entry.retry_count >= entry.max_retries {
            return Err(HoldingError::MaxRetriesExceeded {
                max: entry.max_retries,
            });
        }

        entry.retry_count += 1;
        entry.last_retry = Some(Utc::now());
        Ok(entry)
    }

    /// List all currently held packets.
    pub fn list_held(&self) -> Vec<&HeldPacket> {
        self.held.values().collect()
    }

    /// Number of packets currently in the buffer.
    pub fn held_count(&self) -> usize {
        self.held.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_packet_id(n: u8) -> PacketId {
        let mut bytes = [0u8; 32];
        bytes[0] = n;
        PacketId::new(bytes)
    }

    #[test]
    fn hold_and_release() {
        let mut buf = HoldingBuffer::new(3);
        let pid = test_packet_id(1);

        buf.hold(pid, HoldReason::FeeBudgetExceeded);
        assert_eq!(buf.held_count(), 1);

        let released = buf.release(&pid);
        assert!(released.is_some(), "should return the held packet");
        assert_eq!(released.expect("test: released").packet_id, pid);
        assert_eq!(buf.held_count(), 0);
    }

    #[test]
    fn hold_and_retry() {
        let mut buf = HoldingBuffer::new(5);
        let pid = test_packet_id(2);

        buf.hold(pid, HoldReason::NetworkCongestion);
        let entry = buf.retry(&pid).expect("test: first retry");
        assert_eq!(entry.retry_count, 1);
        assert!(entry.last_retry.is_some());
    }

    #[test]
    fn retry_max_exceeded() {
        let mut buf = HoldingBuffer::new(2);
        let pid = test_packet_id(3);

        buf.hold(pid, HoldReason::NoRouteAvailable);
        buf.retry(&pid).expect("test: retry 1");
        buf.retry(&pid).expect("test: retry 2");

        let err = buf.retry(&pid);
        assert!(
            matches!(err, Err(HoldingError::MaxRetriesExceeded { max: 2 })),
            "expected MaxRetriesExceeded, got {err:?}"
        );
    }

    #[test]
    fn list_held_packets() {
        let mut buf = HoldingBuffer::new(3);
        buf.hold(test_packet_id(10), HoldReason::FeeBudgetExceeded);
        buf.hold(test_packet_id(20), HoldReason::NetworkCongestion);
        buf.hold(test_packet_id(30), HoldReason::NoRouteAvailable);

        let held = buf.list_held();
        assert_eq!(held.len(), 3);
    }

    #[test]
    fn release_not_found() {
        let mut buf = HoldingBuffer::new(3);
        let result = buf.release(&test_packet_id(99));
        assert!(result.is_none(), "releasing non-existent should return None");
    }

    #[test]
    fn held_count() {
        let mut buf = HoldingBuffer::new(3);
        assert_eq!(buf.held_count(), 0);

        buf.hold(test_packet_id(1), HoldReason::FeeBudgetExceeded);
        assert_eq!(buf.held_count(), 1);

        buf.hold(test_packet_id(2), HoldReason::NetworkCongestion);
        assert_eq!(buf.held_count(), 2);

        buf.release(&test_packet_id(1));
        assert_eq!(buf.held_count(), 1);
    }
}

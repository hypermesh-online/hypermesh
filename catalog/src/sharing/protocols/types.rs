// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::{AssetMetadata, AssetPackage, AssetRegistration};

/// Share permission levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SharePermission {
    /// Publicly accessible to all
    Public,
    /// Private - owner only
    Private,
    /// Shared with specific nodes
    Restricted { allowed_nodes: Vec<String> },
    /// Shared with friends/trusted peers
    Friends,
    /// Anonymous sharing
    Anonymous,
    /// Verified nodes only (with Proof of State proofs)
    Verified,
}

impl Default for SharePermission {
    fn default() -> Self {
        Self::Public
    }
}

/// Bandwidth allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthAllocation {
    /// Maximum upload bandwidth (bytes/sec)
    pub max_upload: u64,
    /// Maximum download bandwidth (bytes/sec)
    pub max_download: u64,
    /// Reserved bandwidth for priority transfers
    pub reserved_priority: u64,
    /// Fair share per peer
    pub per_peer_limit: u64,
    /// Burst allowance
    pub burst_size: u64,
    /// Burst duration
    pub burst_duration: Duration,
}

impl Default for BandwidthAllocation {
    fn default() -> Self {
        Self {
            max_upload: 10 * 1024 * 1024,       // 10 MB/s
            max_download: 10 * 1024 * 1024,     // 10 MB/s
            reserved_priority: 2 * 1024 * 1024, // 2 MB/s
            per_peer_limit: 1024 * 1024,        // 1 MB/s
            burst_size: 5 * 1024 * 1024,        // 5 MB burst
            burst_duration: Duration::from_secs(5),
        }
    }
}

/// Transfer priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum TransferPriority {
    /// Critical system transfers
    Critical = 0,
    /// High priority user transfers
    High = 1,
    /// Normal priority
    Normal = 2,
    /// Low priority background transfers
    Low = 3,
}

/// Active transfer information
#[derive(Debug, Clone)]
pub(in crate::sharing) struct ActiveTransfer {
    /// Transfer ID
    pub _id: String,
    /// Peer ID
    pub peer_id: String,
    /// Asset being transferred
    pub asset_id: AssetRegistration,
    /// Transfer direction
    pub _direction: TransferDirection,
    /// Priority
    pub _priority: TransferPriority,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total size
    pub total_size: u64,
    /// Start time
    pub started_at: SystemTime,
    /// Current bandwidth (bytes/sec)
    pub current_bandwidth: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::sharing) enum TransferDirection {
    Upload,
    Download,
}

/// Incentive contribution tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContributionStats {
    /// Total bytes uploaded
    pub bytes_uploaded: u64,
    /// Total bytes downloaded
    pub bytes_downloaded: u64,
    /// Upload/download ratio
    pub ratio: f64,
    /// Contribution score
    pub score: f64,
    /// Earned credits
    pub credits: u64,
    /// Spent credits
    pub credits_spent: u64,
}

/// Protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ProtocolMessage {
    /// Request package
    RequestPackage {
        asset_id: String, // Package hash, not BlockMatrix AssetRegistration
        requester: String,
    },
    /// Package response
    PackageResponse {
        asset_id: String, // Package hash, not BlockMatrix AssetRegistration
        package: AssetPackage,
    },
    /// Package metadata
    PackageMetadata {
        asset_id: String, // Package hash, not BlockMatrix AssetRegistration
        metadata: AssetMetadata,
    },
    /// Availability notification
    AvailabilityNotification {
        asset_id: String, // Package hash, not BlockMatrix AssetRegistration
        available: bool,
    },
    /// Bandwidth negotiation
    BandwidthNegotiation {
        proposed_rate: u64,
        duration: Duration,
    },
    /// Transfer acknowledgment
    TransferAck {
        transfer_id: String,
        received_bytes: u64,
    },
    /// Error response
    Error { code: u32, message: String },
}

/// Peer connection state
#[derive(Debug, Clone)]
pub(in crate::sharing) struct PeerConnection {
    /// Peer ID
    pub _peer_id: String,
    /// Connection address
    pub _address: String,
    /// Connection established time
    pub _connected_at: SystemTime,
    /// Current bandwidth allocation
    pub allocated_bandwidth: u64,
    /// Permission level
    pub permission: SharePermission,
    /// Active transfers
    pub _active_transfers: Vec<String>,
    /// Connection quality score
    pub quality_score: f64,
}

/// Transfer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    /// Peer ID
    pub peer_id: String,
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Progress (0-1)
    pub progress: f64,
    /// Current speed (bytes/sec)
    pub speed: u64,
    /// Estimated time remaining
    pub estimated_time: Duration,
}

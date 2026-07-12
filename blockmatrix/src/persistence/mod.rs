// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix Persistence Layer Implementation
//!
//! Provides persistent storage for the Block-MATRIX distributed computing platform,
//! ensuring all state (matrix coordinates, every-node-blockchain data, geospatial
//! mappings, network topology) survives node restarts with zero data loss.

pub mod blockchain_storage;
pub mod manager;
pub mod matrix_state;
pub mod recovery;
pub mod snapshots;
pub mod topology_backup;

// Re-export main types
pub use blockchain_storage::{BlockQuery, BlockchainStorage, ChainMetadata};
pub use manager::{PersistenceConfig, PersistenceManager, StorageStats};
pub use matrix_state::{MatrixState, MatrixStateSerializer, SerializationFormat};
pub use recovery::{RecoveryManager, RecoveryReport, RecoveryStatus};
pub use snapshots::{SnapshotManager, SnapshotMetadata, SnapshotSchedule};
pub use topology_backup::{BackupMode, TopologyBackup};

// Explicit durable/ephemeral persistence boundary (defined below).
// Re-exported here so callers use `persistence::{classify, StateKind,
// StatePersistence}` without reaching into the module body.

use thiserror::Error;

/// Errors that can occur during persistence operations
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// I/O error during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Decompression error
    #[error("Decompression error: {0}")]
    Decompression(String),

    /// Checksum validation failed
    #[error("Checksum validation failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    /// Recovery failed
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Snapshot error
    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    /// Disk space error
    #[error("Insufficient disk space: needed {needed} bytes, available {available} bytes")]
    InsufficientDiskSpace { needed: u64, available: u64 },

    /// Lock error
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Block integrity violation detected — possible tampering
    ///
    /// SECURITY REVIEW REQUIRED: This error indicates the persisted block's
    /// canonical hash does not match the stored/expected hash. This could mean:
    /// 1. Data corruption (disk error)
    /// 2. Format incompatibility (software bug)
    /// 3. Intentional tampering (security breach)
    ///
    /// The node MUST NOT accept this block. Manual investigation required.
    /// See papers/HYPERMESH.md §6.2 and §7.2 for the security model.
    #[error("Block integrity violation at index {index}: stored hash {stored_hash}, computed hash {computed_hash} — possible tampering, SECURITY REVIEW REQUIRED")]
    IntegrityViolation {
        index: u64,
        stored_hash: String,
        computed_hash: String,
    },
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Whether a piece of node state is written to disk (durable) or rebuilt on boot
/// (ephemeral).
///
/// This classification makes the persistence boundary EXPLICIT and verifiable so
/// an upgrade / restart preserves exactly the durable set and discards the
/// ephemeral set without ambiguity. It documents intent; it does not itself move
/// bytes. See [`classify`] for the authoritative mapping.
///
/// ## Durable (survives restart, on disk)
/// - Blockchain (`persistence/blockchain_storage/`) and the genesis block that
///   carries the device fingerprint.
/// - Node identity DER (FALCON-1024 / Kyber-1024 key material).
/// - Matrix snapshots (`persistence/matrix_state.rs`) and the WAL.
///
/// ## Ephemeral (rebuilt on boot from chain + peer announcements)
/// - DNS registrations, `ShardLocationIndex`, `SwarmAnalytics`, propagation
///   weights.
/// - Live interface / carrier state (owned by the Substrate, re-enumerated each
///   boot) and the connection pool.
///
/// ## The address is durable-by-derivation
/// The node's `fd48:4d00::/32` address is NEITHER a stored durable value NOR a
/// rebuilt-from-peers ephemeral one: it is recomputed byte-identically every boot
/// by `base::derive_address(node_id)`. It is therefore classified
/// [`StatePersistence::DurableByDerivation`] — nothing writes it to disk as
/// authoritative state, and any peer can recompute and verify it (R15/R16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatePersistence {
    /// Written to disk; must survive a restart or upgrade unchanged.
    Durable,
    /// Rebuilt on boot from the chain and peer announcements; safe to discard.
    Ephemeral,
    /// Recomputed identically every boot from a pure function of identity; never
    /// stored as authoritative state, never leased (e.g. the derived address).
    DurableByDerivation,
}

/// Node-state kinds classified by [`StatePersistence`].
///
/// Enumerating the kinds (rather than free-form strings) lets a restart / upgrade
/// path assert, at compile time, that every known state category has an explicit
/// persistence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    /// The device/network blockchain and its blocks.
    Blockchain,
    /// The genesis block carrying the device fingerprint.
    GenesisBlock,
    /// FALCON/Kyber identity key material (DER).
    IdentityKeys,
    /// Matrix state snapshots.
    MatrixSnapshot,
    /// Write-ahead log.
    Wal,
    /// The node's derived `fd48:4d00::/32` address.
    DerivedAddress,
    /// DNS registrations (re-registered from chain on boot).
    DnsRegistrations,
    /// The shard location index.
    ShardLocationIndex,
    /// Swarm analytics / popularity metrics.
    SwarmAnalytics,
    /// Shard/asset propagation weights.
    PropagationWeights,
    /// Live interface and carrier state (Substrate-owned).
    LiveInterfaceState,
    /// The transport connection pool.
    ConnectionPool,
}

/// The authoritative durable/ephemeral classification for each [`StateKind`].
///
/// One `match` with no wildcard arm: adding a new [`StateKind`] forces a
/// deliberate persistence decision here (the compiler rejects a missing arm),
/// which is exactly the "make the boundary explicit and verifiable" goal.
pub fn classify(kind: StateKind) -> StatePersistence {
    match kind {
        StateKind::Blockchain
        | StateKind::GenesisBlock
        | StateKind::IdentityKeys
        | StateKind::MatrixSnapshot
        | StateKind::Wal => StatePersistence::Durable,

        StateKind::DerivedAddress => StatePersistence::DurableByDerivation,

        StateKind::DnsRegistrations
        | StateKind::ShardLocationIndex
        | StateKind::SwarmAnalytics
        | StateKind::PropagationWeights
        | StateKind::LiveInterfaceState
        | StateKind::ConnectionPool => StatePersistence::Ephemeral,
    }
}

#[cfg(test)]
mod persistence_classification_tests {
    use super::*;

    #[test]
    fn durable_state_is_classified_durable() {
        for kind in [
            StateKind::Blockchain,
            StateKind::GenesisBlock,
            StateKind::IdentityKeys,
            StateKind::MatrixSnapshot,
            StateKind::Wal,
        ] {
            assert_eq!(classify(kind), StatePersistence::Durable, "{kind:?}");
        }
    }

    #[test]
    fn ephemeral_state_is_classified_ephemeral() {
        for kind in [
            StateKind::DnsRegistrations,
            StateKind::ShardLocationIndex,
            StateKind::SwarmAnalytics,
            StateKind::PropagationWeights,
            StateKind::LiveInterfaceState,
            StateKind::ConnectionPool,
        ] {
            assert_eq!(classify(kind), StatePersistence::Ephemeral, "{kind:?}");
        }
    }

    #[test]
    fn derived_address_is_durable_by_derivation_not_stored() {
        // The address is recomputed every boot; it is never a stored durable
        // value nor a peer-rebuilt ephemeral one.
        assert_eq!(
            classify(StateKind::DerivedAddress),
            StatePersistence::DurableByDerivation
        );
        assert_ne!(
            classify(StateKind::DerivedAddress),
            StatePersistence::Durable
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PersistenceError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Checksum validation failed: expected abc123, got def456"
        );
    }
}

#[cfg(test)]
mod test_integration;

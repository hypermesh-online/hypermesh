// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain persistence with versioned format and integrity verification
//!
//! Provides efficient storage for per-node blockchains with append-only logs,
//! indexes, write-ahead logging for crash recovery, and tamper detection.
//!
//! Split into submodules:
//! - `format`: v1 serialization + canonical hash verification
//! - `wal`: write-ahead log entries, reader, writer
//! - `metadata`: `ChainMetadata`, `BlockQuery`
//! - `storage`: `BlockchainStorage` public API

mod format;
mod format_migrations;
mod metadata;
mod storage;
mod wal;

#[cfg(test)]
mod tests;

pub use format_migrations::{has_migration, migrate_v1_to_v2, MIGRATIONS};
pub use metadata::{BlockQuery, ChainMetadata};
pub use storage::BlockchainStorage;
pub use wal::{WalEntry, WalOperation};

/// Phase J.1 — exposure of the verified deserializer for integration
/// tests and tooling that needs to exercise the V1/V2 magic recognition
/// paths directly without round-tripping through [`BlockchainStorage`].
///
/// Hidden from rustdoc; not part of the stable public surface.
#[doc(hidden)]
pub fn test_deserialize(buffer: &[u8]) -> super::PersistenceResult<crate::blockchain::block::Block> {
    format::deserialize_block_verified(buffer)
}

/// Phase J.1 — V2 serializer for migration tooling and integration tests.
#[doc(hidden)]
pub fn test_serialize_v2(
    block: &crate::blockchain::block::Block,
) -> super::PersistenceResult<Vec<u8>> {
    format::serialize_block_v2(block)
}

/// Phase J.1 — V1 serializer for integration tests.
#[doc(hidden)]
pub fn test_serialize_v1(
    block: &crate::blockchain::block::Block,
) -> super::PersistenceResult<Vec<u8>> {
    format::serialize_block_v1(block)
}

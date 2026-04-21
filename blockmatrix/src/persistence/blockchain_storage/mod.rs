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
mod metadata;
mod storage;
mod wal;

#[cfg(test)]
mod tests;

pub use metadata::{BlockQuery, ChainMetadata};
pub use storage::BlockchainStorage;
pub use wal::{WalEntry, WalOperation};

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Durable write-through sink for accepted blocks (S3.0 / B1).
//!
//! Before S3.0 the only production `save_block` call sites were the two in the
//! node binary's bootstrap (genesis and the hardware-registration block).
//! `NodeBlockchain::add_block` / `insert_received_block` mutated memory ONLY,
//! so every runtime block — stored assets, shard registrations, transfer
//! receipts — was lost on restart, because restart rebuilds the chain from
//! disk.
//!
//! This trait is the seam that closes that hole without making the chain
//! depend on the concrete persistence stack. [`NodeBlockchain`] holds an
//! `Option<Arc<dyn BlockSink>>` (mirroring the existing optional
//! `NodeSigner`): when present, every block that passes validation is written
//! through BEFORE it becomes visible in memory; when absent (library and test
//! chains) behaviour is exactly as it was.
//!
//! [`NodeBlockchain`]: super::chain::NodeBlockchain

use super::block::Block;

/// Durable sink for blocks that have passed chain validation.
///
/// FAIL-CLOSED contract: `NodeBlockchain` calls this BEFORE inserting the
/// block into its in-memory maps and aborts the insert if it returns `Err`.
/// A chain must never hold a block it cannot recover after a restart — the
/// alternative (log-and-continue) silently produces an in-memory chain whose
/// height and head do not survive a reboot, which is the very defect this
/// trait exists to fix.
///
/// # RE-ENTRANCY: an implementation MUST NOT call back into `NodeBlockchain`
///
/// `persist_block` is awaited from inside `insert_block` **while the chain's
/// `blocks`, `hash_index`, `head` and `stats` write locks are held**. Any call
/// back into the chain from the sink — including read-only queries such as
/// `get_height`, `get_block`, `get_chain` — deadlocks the task permanently
/// (S3.0 QA proved this with a re-entrant probe sink: it hung until the test
/// harness timed it out). The only implementation today
/// (`PersistenceManager`) touches nothing but its own storage and is safe.
///
/// An implementation must therefore depend on nothing but the `Block` it is
/// handed and its own storage state. If a sink needs chain context, the caller
/// must pass it in — not fetch it from inside the callback.
///
/// Implementations should also keep the call as short as the durability
/// requirement allows, and must not block the async worker: the blocking file
/// I/O belongs on `tokio::task::spawn_blocking` (as
/// `BlockchainStorage` does since the S3.0 QA follow-up), because the same
/// held locks stall every chain reader for the call's duration.
#[async_trait::async_trait]
pub trait BlockSink: Send + Sync {
    /// Durably record `block` (write-ahead log + block storage).
    ///
    /// Must be idempotent-safe from the caller's perspective: the chain only
    /// ever calls it once per accepted block, after a duplicate-index check.
    ///
    /// MUST NOT call back into `NodeBlockchain` — see the trait docs.
    async fn persist_block(&self, block: &Block) -> Result<(), String>;
}

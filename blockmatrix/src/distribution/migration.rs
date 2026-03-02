// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard Migration (R14)
//!
//! Implements the copy-then-redirect pattern for moving shards between
//! nodes without blocking reads. The old location continues serving
//! until the new copy is verified and routing is updated.
//!
//! # Lifecycle
//!
//! ```text
//! Pending -> Copying -> Redirecting -> Complete
//!                  \-> Failed (rollback)
//! ```

use hypermesh_lib::{ContentHash, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::network::shard_transport::ShardTransport;

/// Status of a shard migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Plan created, not yet started.
    Pending,
    /// Shard data is being copied to the destination.
    Copying,
    /// Copy verified; routing is being updated.
    Redirecting,
    /// Migration finished successfully.
    Complete,
    /// Migration failed; old location still serves.
    Failed,
}

/// A plan describing one shard migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// The shard being migrated.
    pub shard_id: ContentHash,
    /// Node currently holding the shard.
    pub source: NodeId,
    /// Target node for the shard.
    pub destination: NodeId,
    /// Current status.
    pub status: MigrationStatus,
    /// Human-readable reason for migration (e.g. "rebalance", "node leaving").
    pub reason: String,
}

impl MigrationPlan {
    /// Create a new pending migration plan.
    pub fn new(shard_id: ContentHash, source: NodeId, destination: NodeId, reason: impl Into<String>) -> Self {
        Self {
            shard_id,
            source,
            destination,
            status: MigrationStatus::Pending,
            reason: reason.into(),
        }
    }
}

/// Outcome of executing a migration.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// The completed plan with final status.
    pub plan: MigrationPlan,
    /// Bytes transferred (0 if failed before copy).
    pub bytes_transferred: usize,
}

/// Executes shard migrations using an underlying transport.
///
/// The executor guarantees that reads are never blocked: the source
/// node continues serving until the redirect phase completes. On
/// failure the plan status is set to `Failed` and the source remains
/// authoritative.
pub struct MigrationExecutor<T: ShardTransport> {
    transport: Arc<T>,
}

impl<T: ShardTransport> MigrationExecutor<T> {
    /// Create a new migration executor.
    pub fn new(transport: Arc<T>) -> Self {
        Self { transport }
    }

    /// Execute a migration plan.
    ///
    /// Steps:
    /// 1. Fetch shard from source (Copying).
    /// 2. Send shard to destination (Copying).
    /// 3. Verify destination has the shard (Redirecting).
    /// 4. Mark complete or rollback on failure.
    pub async fn execute(&self, mut plan: MigrationPlan) -> MigrationResult {
        // Phase 1: Copy.
        plan.status = MigrationStatus::Copying;

        let data = match self.transport.fetch_shard(&plan.source, &plan.shard_id).await {
            Ok(d) => d,
            Err(_) => {
                plan.status = MigrationStatus::Failed;
                return MigrationResult {
                    plan,
                    bytes_transferred: 0,
                };
            }
        };

        let bytes_len = data.len();

        if let Err(_) = self
            .transport
            .send_shard(&plan.destination, &plan.shard_id, &data)
            .await
        {
            plan.status = MigrationStatus::Failed;
            return MigrationResult {
                plan,
                bytes_transferred: 0,
            };
        }

        // Phase 2: Redirect -- verify destination has the shard.
        plan.status = MigrationStatus::Redirecting;

        match self
            .transport
            .fetch_shard(&plan.destination, &plan.shard_id)
            .await
        {
            Ok(verified) if verified == data => {
                plan.status = MigrationStatus::Complete;
            }
            _ => {
                // Verification failed -- rollback.
                plan.status = MigrationStatus::Failed;
                return MigrationResult {
                    plan,
                    bytes_transferred: bytes_len,
                };
            }
        }

        MigrationResult {
            plan,
            bytes_transferred: bytes_len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::shard_transport::MockShardTransport;

    fn node(seed: u8) -> NodeId {
        NodeId::from_bytes([seed; 32])
    }

    fn shard_id(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_successful_migration() {
        let transport = Arc::new(MockShardTransport::new());
        let src = node(1);
        let dst = node(2);
        let sid = shard_id(10);
        let data = vec![0xAB; 1024];

        // Pre-populate source.
        transport.insert_shard(&src, &sid, data.clone()).await;

        let executor = MigrationExecutor::new(Arc::clone(&transport));
        let plan = MigrationPlan::new(sid, src, dst, "rebalance");

        let result = executor.execute(plan).await;
        assert_eq!(result.plan.status, MigrationStatus::Complete);
        assert_eq!(result.bytes_transferred, 1024);

        // Destination should now have the shard.
        let fetched = transport
            .fetch_shard(&dst, &sid)
            .await
            .expect("test: destination should have shard");
        assert_eq!(fetched, data);
    }

    #[tokio::test]
    async fn test_migration_failure_source_unreachable() {
        let transport = Arc::new(MockShardTransport::new());
        let src = node(1);
        let dst = node(2);
        let sid = shard_id(10);

        // Source is unreachable -- no shard populated.
        transport.set_unreachable(&src).await;

        let executor = MigrationExecutor::new(Arc::clone(&transport));
        let plan = MigrationPlan::new(sid, src, dst, "test-fail");

        let result = executor.execute(plan).await;
        assert_eq!(result.plan.status, MigrationStatus::Failed);
        assert_eq!(result.bytes_transferred, 0);
    }

    #[tokio::test]
    async fn test_migration_failure_destination_unreachable() {
        let transport = Arc::new(MockShardTransport::new());
        let src = node(1);
        let dst = node(2);
        let sid = shard_id(10);
        let data = vec![0xCD; 512];

        transport.insert_shard(&src, &sid, data.clone()).await;
        transport.set_unreachable(&dst).await;

        let executor = MigrationExecutor::new(Arc::clone(&transport));
        let plan = MigrationPlan::new(sid, src, dst, "test-fail-dst");

        let result = executor.execute(plan).await;
        assert_eq!(result.plan.status, MigrationStatus::Failed);
        // Copy was attempted but send to dst failed.
        assert_eq!(result.bytes_transferred, 0);
    }

    #[tokio::test]
    async fn test_migration_plan_initial_state() {
        let plan = MigrationPlan::new(
            shard_id(1),
            node(1),
            node(2),
            "unit test",
        );
        assert_eq!(plan.status, MigrationStatus::Pending);
        assert_eq!(plan.reason, "unit test");
    }
}

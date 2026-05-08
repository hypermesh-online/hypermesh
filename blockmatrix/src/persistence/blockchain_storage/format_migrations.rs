// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase J.1 — Block format migration registry.
//!
//! This module reserves the migration substrate so future block-format
//! changes have a single place to land their upgrade logic. **Phase
//! J.1 itself does NOT change the format.** V2 schema is byte-identical
//! to V1; the registry simply documents the upgrade path and the
//! invariant that block hashes are PERMANENT (never re-hashed during a
//! migration — see `papers/HYPERMESH.md` Section 6.2 and 7.2).
//!
//! ## Why a migration registry, not in-place mutation?
//!
//! Block-format changes have historically been a chain-integrity hazard
//! (the original deploy regression, commit `03bfae00`). Future changes
//! must be:
//!
//! 1. Forward-compatible at read time — old data still decodes.
//! 2. Deterministic — every node lands at the same canonical block
//!    after migration.
//! 3. Hash-preserving — the canonical block hash is never recomputed
//!    during a migration; new fields go into the V2+ envelope, NOT
//!    the canonical hash inputs.
//!
//! ## Adding a new format version
//!
//! 1. Bump `BLOCK_MAGIC_V<n>` in `format.rs` and extend
//!    `deserialize_block_verified` to recognise the new magic.
//! 2. Add `migrate_v<n-1>_to_v<n>(&Block) -> Result<Block>` here.
//! 3. Register it in `MIGRATIONS` so startup detection runs migrations
//!    in order.
//! 4. Add a forward-compat test in `format_migrations` that round-trips
//!    a block from old → new → old (when both directions are safe).

use crate::blockchain::block::Block;
use super::super::PersistenceResult;

/// Migrate a V1 block to V2.
///
/// **Phase J.1 placeholder:** V2 is currently identical to V1, so this
/// returns the input unchanged. When V2 introduces real schema changes,
/// this function will populate the new fields with safe defaults.
///
/// IMPORTANT: This must NEVER mutate inputs to `Block::calculate_hash`.
/// Block hashes are permanent. New fields belong in the envelope, not
/// the canonical hash payload.
pub fn migrate_v1_to_v2(block: &Block) -> PersistenceResult<Block> {
    // V2 schema is byte-identical to V1 in Phase J.1 — no transformation
    // needed. Future schema changes will populate new V2 fields here
    // while preserving the canonical hash inputs unchanged.
    Ok(block.clone())
}

/// Registry of supported migrations, ordered from oldest → newest.
///
/// `(from_version, to_version, migrate_fn)`. Startup migration logic
/// walks this list to run any required transformations.
pub const MIGRATIONS: &[(u8, u8, fn(&Block) -> PersistenceResult<Block>)] = &[
    (1, 2, migrate_v1_to_v2),
];

/// True when a migration exists for `(from, to)`. Used by startup
/// detection to decide whether to migrate or refuse-and-log.
pub fn has_migration(from: u8, to: u8) -> bool {
    MIGRATIONS.iter().any(|(f, t, _)| *f == from && *t == to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::block::Block;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn fixture_block() -> Block {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        Block::genesis(coord)
    }

    #[test]
    fn migration_v1_to_v2_is_identity_in_phase_j1() {
        let block = fixture_block();
        let migrated = migrate_v1_to_v2(&block).expect("test: migrate");
        assert_eq!(block.hash, migrated.hash);
        assert_eq!(block.index, migrated.index);
    }

    #[test]
    fn migration_preserves_canonical_hash() {
        let block = fixture_block();
        let migrated = migrate_v1_to_v2(&block).expect("test: migrate");
        assert_eq!(
            block.calculate_hash(),
            migrated.calculate_hash(),
            "format migrations MUST NOT change canonical hash inputs"
        );
    }

    #[test]
    fn registry_lists_v1_to_v2() {
        assert!(has_migration(1, 2));
        assert!(!has_migration(0, 1));
        assert!(!has_migration(2, 3));
    }
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block serialization format (v1) with versioning magic and canonical hash
//! verification for tamper detection.
//!
//! ## Storage format (v1)
//!
//! ```text
//! [4 bytes: magic "HMB\x01"]  -- HyperMesh Block format version 1
//! [4 bytes: payload_size as u32 LE]
//! [32 bytes: canonical_hash (raw BLAKE3 of block's canonical fields)]
//! [payload_size bytes: bincode-serialized Block]
//! ```
//!
//! SECURITY REVIEW REQUIRED: The canonical hash is verified on every read.
//! Block hashes are PERMANENT and format-independent. They are computed from
//! canonical fields only (index, prev_hash, entries[].asset_hash,
//! entries[].proof_hash). Blocks are NEVER re-hashed during format migration.
//! See papers/HYPERMESH.md Section 6.2 and Section 7.2.

use crate::blockchain::block::Block;

use super::super::{PersistenceError, PersistenceResult};

// SECURITY REVIEW REQUIRED: Format version header definition.
// The magic bytes identify the storage format version. Any change to the
// serialization format MUST use a new magic value (e.g., "HMB\x02").
// See papers/HYPERMESH.md Section 6.2 for the block integrity model.
/// Magic bytes for HyperMesh Block format version 1
pub(super) const BLOCK_MAGIC_V1: [u8; 4] = [b'H', b'M', b'B', 0x01];

/// Total header size: 4 (magic) + 4 (payload_size) + 32 (canonical_hash)
pub(super) const BLOCK_HEADER_SIZE: usize = 40;

/// Compute the raw 32-byte BLAKE3 canonical hash from a block's hex hash string.
///
/// The block's `calculate_hash()` returns a hex-encoded BLAKE3 hash. We parse
/// that back to raw bytes for compact storage in the header.
pub(super) fn canonical_hash_bytes(block: &Block) -> [u8; 32] {
    let hex_hash = block.calculate_hash();
    match blake3::Hash::from_hex(&hex_hash) {
        Ok(hash) => *hash.as_bytes(),
        Err(_) => {
            // Fallback: hash the hex string directly (should never happen with
            // valid BLAKE3 output, but we must not panic in production)
            *blake3::hash(hex_hash.as_bytes()).as_bytes()
        }
    }
}

/// Serialize a block with the v1 format header.
///
/// Returns the full byte sequence: magic + payload_size + canonical_hash + payload.
pub(super) fn serialize_block_v1(block: &Block) -> PersistenceResult<Vec<u8>> {
    let payload = bincode::serialize(block)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let payload_size = payload.len() as u32;
    let hash_bytes = canonical_hash_bytes(block);

    let mut buf = Vec::with_capacity(BLOCK_HEADER_SIZE + payload.len());
    buf.extend_from_slice(&BLOCK_MAGIC_V1);
    buf.extend_from_slice(&payload_size.to_le_bytes());
    buf.extend_from_slice(&hash_bytes);
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Deserialize a block from a buffer, handling both v1 (with header) and
/// legacy (raw bincode) formats.
///
/// SECURITY REVIEW REQUIRED: This function verifies the canonical hash on
/// every read. If the stored hash does not match the computed hash, the block
/// is rejected with `IntegrityViolation`. An attacker who modifies persisted
/// data on disk will be detected here. Blocks are NEVER re-hashed.
/// See papers/HYPERMESH.md Section 6.2 and Section 7.2.
pub(super) fn deserialize_block_verified(buffer: &[u8]) -> PersistenceResult<Block> {
    if buffer.len() >= BLOCK_HEADER_SIZE && buffer[..4] == BLOCK_MAGIC_V1 {
        // V1 format: parse header, deserialize payload, verify hash
        let payload_size =
            u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]) as usize;
        let stored_hash_bytes: [u8; 32] = buffer[8..40]
            .try_into()
            .map_err(|_| PersistenceError::Deserialization(
                "invalid canonical hash in header".to_string(),
            ))?;

        let payload_end = BLOCK_HEADER_SIZE + payload_size;
        if buffer.len() < payload_end {
            return Err(PersistenceError::Deserialization(format!(
                "truncated block: header says {} bytes but only {} available",
                payload_size,
                buffer.len() - BLOCK_HEADER_SIZE
            )));
        }

        let block: Block =
            bincode::deserialize(&buffer[BLOCK_HEADER_SIZE..payload_end])
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        // Verify canonical hash
        let computed = canonical_hash_bytes(&block);
        if computed != stored_hash_bytes {
            let stored_hex = blake3::Hash::from_bytes(stored_hash_bytes).to_hex();
            let computed_hex = blake3::Hash::from_bytes(computed).to_hex();
            return Err(PersistenceError::IntegrityViolation {
                index: block.index,
                stored_hash: stored_hex.to_string(),
                computed_hash: computed_hex.to_string(),
            });
        }

        Ok(block)
    } else {
        // SECURITY REVIEW REQUIRED: Legacy format detection.
        // Old data written without a header is raw bincode. We still verify
        // the block's stored hash matches its canonical hash to detect
        // tampering. An attacker who modifies legacy blocks will be caught
        // here because the hash field inside the Block struct was set at
        // creation time and is compared against a fresh calculate_hash().
        // See papers/HYPERMESH.md Section 7.2.
        let block: Block = bincode::deserialize(buffer)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        let computed = block.calculate_hash();
        if computed != block.hash {
            return Err(PersistenceError::IntegrityViolation {
                index: block.index,
                stored_hash: block.hash.clone(),
                computed_hash: computed,
            });
        }

        Ok(block)
    }
}

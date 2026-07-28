// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! D3 — the wire surface for PRESENTED ASSET CHAINS.
//!
//! D2 dissolved `foreign/` into `blockchain/accept/` and produced
//! [`accept_asset_chain`](crate::blockchain::NodeBlockchain::accept_asset_chain),
//! the unified "receive an asset chain" operation — but nothing carried a chain
//! from the peer that holds it to the node that would adopt it. Only tests
//! invoked it. This module is that missing hop: a peer presents an asset's
//! verified sub-chain, and the node verifies + stores it through the ONE accept
//! gate.
//!
//! # Shape
//!
//! Modelled exactly on the mirror-attestation wire
//! ([`attestation_wire`](super::attestation_wire)): a tag byte, a `u32`
//! little-endian length prefix, then the JSON payload.
//!
//! ```text
//! [ASSET_CHAIN_TAG (1)][len u32 LE (4)][PresentedAssetChain JSON (len)]
//! ```
//!
//! # Parsing attacker-controlled bytes
//!
//! Every field is read with checked arithmetic and checked slicing; there is no
//! indexing that can panic and no `unwrap`. The declared length is validated
//! against [`MAX_ASSET_CHAIN_WIRE_BYTES`] BEFORE anything is sliced, and against
//! the bytes actually present before the body slice is taken — the A2
//! remote-`SIGABRT` shape (a length prefix trusted into a slice expression)
//! cannot occur here. The decoder slices from the buffer it was handed and never
//! pre-allocates the declared length, so a lying prefix buys an attacker no
//! allocation.
//!
//! # This module performs NO verification
//!
//! A decoded [`PresentedAssetChain`] is an untrusted claim. Every
//! lineage/signer/bounds check lives inside `accept_asset_chain` — internal
//! prev-link lineage via `AssetLineage::verify`, every signer's FALCON envelope,
//! the `has_ever_seen_asset`/`AlreadyOnSpine` refusal, and the received-store
//! byte budget. This codec deliberately duplicates none of it (that divergence
//! class is what failed S3.3's gate).

use crate::blockchain::PresentedAssetChain;

/// The wire tag for a presented-asset-chain submission.
///
/// `0x55`, the next free byte after the S3.4 mirror-attestation tag `0x54`
/// (`attestation_wire::MIRROR_ATTEST_TAG`) — the two surfaces sit adjacent
/// because they are the two halves of the per-asset receive story. Defined HERE,
/// next to the codec, and merely re-exported by the protocol tag table
/// (`message_handlers::protocol::TAG_ASSET_CHAIN`), so a sender and a receiver
/// cannot disagree about the byte.
pub const ASSET_CHAIN_TAG: u8 = 0x55;

/// Largest accepted `ASSET_CHAIN_TAG` payload.
///
/// The accept path caps a chain at
/// [`MAX_RECEIVED_CHAIN_ENTRIES`](crate::blockchain::MAX_RECEIVED_CHAIN_ENTRIES)
/// (512) entries. A single entry carries its FALCON material TWICE (the
/// `state_proof` and the `proof_bytes` the envelope signs) plus a ~1793-byte
/// public key and a ~1280-byte signature; JSON-encoded as number arrays at
/// roughly four bytes each, a generous honest maximum is ~64 KiB per entry — the
/// same figure the whole mirror-attestation message is capped at, since an entry
/// carries comparable crypto material. 512 × 64 KiB = 32 MiB, so 32 MiB is a
/// legitimate maximum chain that still bounds a single message far below the
/// received store's 64 MiB budget
/// ([`MAX_RECEIVED_STORE_BYTES`](crate::blockchain::MAX_RECEIVED_STORE_BYTES)).
///
/// This is the FIRST line of defence; the accept path's own bounds (entry cap,
/// store byte budget, reject-not-evict) are the authoritative flood protection
/// now that the store is network-fed.
pub const MAX_ASSET_CHAIN_WIRE_BYTES: usize = 32 * 1024 * 1024;

/// Fixed header: tag byte + `u32` little-endian length.
const HEADER_LEN: usize = 5;

/// Why an `ASSET_CHAIN_TAG` payload could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetChainWireError {
    /// Fewer bytes than the fixed header requires.
    Truncated {
        /// Bytes received.
        got: usize,
        /// Bytes the header alone needs.
        need: usize,
    },
    /// The first byte is not the expected [`ASSET_CHAIN_TAG`].
    WrongTag {
        /// The tag byte actually present.
        got: u8,
    },
    /// The declared payload length exceeds [`MAX_ASSET_CHAIN_WIRE_BYTES`].
    TooLarge {
        /// Length the sender declared.
        declared: usize,
        /// The cap.
        limit: usize,
    },
    /// The declared payload length exceeds the bytes actually present.
    LengthMismatch {
        /// Length the sender declared.
        declared: usize,
        /// Payload bytes actually present.
        available: usize,
    },
    /// The payload is not a well-formed presented asset chain.
    Malformed {
        /// Deserialization diagnostic.
        detail: String,
    },
    /// The chain could not be serialized for sending.
    NotEncodable {
        /// Serialization diagnostic.
        detail: String,
    },
}

impl std::fmt::Display for AssetChainWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { got, need } => {
                write!(f, "asset-chain message truncated: {got} bytes, need {need}")
            }
            Self::WrongTag { got } => {
                write!(f, "asset-chain message has wrong tag 0x{got:02x}")
            }
            Self::TooLarge { declared, limit } => write!(
                f,
                "asset-chain payload declares {declared} bytes, limit is {limit}"
            ),
            Self::LengthMismatch { declared, available } => write!(
                f,
                "asset-chain payload declares {declared} bytes but {available} are present"
            ),
            Self::Malformed { detail } => {
                write!(f, "asset-chain payload is malformed: {detail}")
            }
            Self::NotEncodable { detail } => {
                write!(f, "asset-chain could not be encoded: {detail}")
            }
        }
    }
}

impl std::error::Error for AssetChainWireError {}

/// Build an `ASSET_CHAIN_TAG` payload for `chain`.
pub fn encode_presented_asset_chain(
    chain: &PresentedAssetChain,
) -> Result<Vec<u8>, AssetChainWireError> {
    let body = serde_json::to_vec(chain).map_err(|e| AssetChainWireError::NotEncodable {
        detail: e.to_string(),
    })?;
    if body.len() > MAX_ASSET_CHAIN_WIRE_BYTES {
        return Err(AssetChainWireError::TooLarge {
            declared: body.len(),
            limit: MAX_ASSET_CHAIN_WIRE_BYTES,
        });
    }

    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.push(ASSET_CHAIN_TAG);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode an `ASSET_CHAIN_TAG` payload.
///
/// Performs NO verification: the chain returned here is an untrusted claim.
/// [`accept_asset_chain`](crate::blockchain::NodeBlockchain::accept_asset_chain)
/// — the full lineage + signer + bounds gate — is the only accept path, and this
/// function deliberately does not duplicate any part of it.
pub fn decode_presented_asset_chain(
    data: &[u8],
) -> Result<PresentedAssetChain, AssetChainWireError> {
    let header = data.get(..HEADER_LEN).ok_or(AssetChainWireError::Truncated {
        got: data.len(),
        need: HEADER_LEN,
    })?;

    if header[0] != ASSET_CHAIN_TAG {
        return Err(AssetChainWireError::WrongTag { got: header[0] });
    }

    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&header[1..HEADER_LEN]);
    let declared = u32::from_le_bytes(length_bytes) as usize;

    if declared > MAX_ASSET_CHAIN_WIRE_BYTES {
        return Err(AssetChainWireError::TooLarge {
            declared,
            limit: MAX_ASSET_CHAIN_WIRE_BYTES,
        });
    }

    let body = data
        .get(HEADER_LEN..)
        .and_then(|rest| rest.get(..declared))
        .ok_or(AssetChainWireError::LengthMismatch {
            declared,
            available: data.len().saturating_sub(HEADER_LEN),
        })?;

    serde_json::from_slice(body).map_err(|e| AssetChainWireError::Malformed {
        detail: e.to_string(),
    })
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.4 — the wire surface for MIRROR ATTESTATIONS.
//!
//! S3.3 gave a mirror everything it needs to produce a
//! [`MirrorAttestation`](hypermesh_lib::attestation::MirrorAttestation) and gave
//! the owner everything it needs to record and seal one — but nothing carried an
//! attestation between the two. A mirror could attest only to itself. This
//! module is that missing hop.
//!
//! # Shape
//!
//! Modelled on `TAG_SHARD_ANNOUNCE` (`swarm_provider::build_shard_announce`):
//! a tag byte, a length prefix, then the payload. The payload is the JSON
//! serialization of one attestation, matching the block-announce path (which
//! also carries JSON behind a length prefix), so the same tooling reads both.
//!
//! ```text
//! [TAG_MIRROR_ATTEST (1)][len u32 LE (4)][attestation JSON (len)]
//! ```
//!
//! # Parsing attacker-controlled bytes
//!
//! Every field is read with checked arithmetic and checked slicing; there is no
//! indexing that can panic and no `unwrap`. The declared length is validated
//! against [`MAX_ATTESTATION_WIRE_BYTES`] BEFORE any allocation, and against the
//! bytes actually present before any slice is taken — the A2 remote-`SIGABRT`
//! shape (a length prefix trusted into a slice expression) cannot occur here.
//!
//! The size cap is also what keeps the canonical-bytes builder in `lib`
//! (`push_lp`, which length-prefixes with a `u32`) far away from its saturation
//! edge: an attestation that fits in 64 KiB cannot carry a 4 GiB string field.

use hypermesh_lib::attestation::MirrorAttestation;

/// The wire tag for a mirror-attestation submission.
///
/// Defined HERE, next to the codec that produces and consumes it, and merely
/// re-exported by the protocol tag table
/// (`message_handlers::protocol::TAG_MIRROR_ATTEST`) — one definition, so a
/// sender and a receiver cannot disagree about the byte.
pub const MIRROR_ATTEST_TAG: u8 = 0x54;

/// Largest accepted `TAG_MIRROR_ATTEST` payload.
///
/// A real attestation is dominated by its FALCON-1024 material — a ~1793-byte
/// public key and a ~1280-byte signature — which JSON encodes as number arrays
/// at roughly four bytes each, so ~16 KiB is a generous honest maximum. 64 KiB
/// leaves headroom without giving a sender a cheap allocation lever.
pub const MAX_ATTESTATION_WIRE_BYTES: usize = 64 * 1024;

/// Fixed header: tag byte + `u32` little-endian length.
const HEADER_LEN: usize = 5;

/// Why a `TAG_MIRROR_ATTEST` payload could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttestationWireError {
    /// Fewer bytes than the fixed header requires.
    Truncated {
        /// Bytes received.
        got: usize,
        /// Bytes the header alone needs.
        need: usize,
    },
    /// The first byte is not the expected `TAG_MIRROR_ATTEST`.
    WrongTag {
        /// The tag byte actually present.
        got: u8,
    },
    /// The declared payload length exceeds [`MAX_ATTESTATION_WIRE_BYTES`].
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
    /// The payload is not a well-formed attestation.
    Malformed {
        /// Deserialization diagnostic.
        detail: String,
    },
    /// The attestation could not be serialized for sending.
    NotEncodable {
        /// Serialization diagnostic.
        detail: String,
    },
}

impl std::fmt::Display for AttestationWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { got, need } => {
                write!(f, "attestation message truncated: {got} bytes, need {need}")
            }
            Self::WrongTag { got } => {
                write!(f, "attestation message has wrong tag 0x{got:02x}")
            }
            Self::TooLarge { declared, limit } => write!(
                f,
                "attestation payload declares {declared} bytes, limit is {limit}"
            ),
            Self::LengthMismatch { declared, available } => write!(
                f,
                "attestation payload declares {declared} bytes but {available} are present"
            ),
            Self::Malformed { detail } => {
                write!(f, "attestation payload is malformed: {detail}")
            }
            Self::NotEncodable { detail } => {
                write!(f, "attestation could not be encoded: {detail}")
            }
        }
    }
}

/// Build a `TAG_MIRROR_ATTEST` payload for `attestation`.
pub fn encode_mirror_attestation(
    attestation: &MirrorAttestation,
) -> Result<Vec<u8>, AttestationWireError> {
    let body = serde_json::to_vec(attestation).map_err(|e| AttestationWireError::NotEncodable {
        detail: e.to_string(),
    })?;
    if body.len() > MAX_ATTESTATION_WIRE_BYTES {
        return Err(AttestationWireError::TooLarge {
            declared: body.len(),
            limit: MAX_ATTESTATION_WIRE_BYTES,
        });
    }

    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.push(MIRROR_ATTEST_TAG);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode a `TAG_MIRROR_ATTEST` payload.
///
/// Performs NO verification: the attestation returned here is an untrusted
/// claim. `NodeBlockchain::record_mirror_attestation` — which delegates to
/// `verify_attestation`, itself the full audit gate — is the only accept gate,
/// and this function deliberately does not duplicate any part of it.
pub fn decode_mirror_attestation(data: &[u8]) -> Result<MirrorAttestation, AttestationWireError> {
    let header = data.get(..HEADER_LEN).ok_or(AttestationWireError::Truncated {
        got: data.len(),
        need: HEADER_LEN,
    })?;

    if header[0] != MIRROR_ATTEST_TAG {
        return Err(AttestationWireError::WrongTag { got: header[0] });
    }

    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&header[1..HEADER_LEN]);
    let declared = u32::from_le_bytes(length_bytes) as usize;

    if declared > MAX_ATTESTATION_WIRE_BYTES {
        return Err(AttestationWireError::TooLarge {
            declared,
            limit: MAX_ATTESTATION_WIRE_BYTES,
        });
    }

    let body = data
        .get(HEADER_LEN..)
        .and_then(|rest| rest.get(..declared))
        .ok_or(AttestationWireError::LengthMismatch {
            declared,
            available: data.len().saturating_sub(HEADER_LEN),
        })?;

    serde_json::from_slice(body).map_err(|e| AttestationWireError::Malformed {
        detail: e.to_string(),
    })
}

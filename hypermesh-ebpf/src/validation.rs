// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Signing-algorithm indicator constants.
//!
//! These identify the signature algorithm a peer used, and are the value
//! written into the kernel `pos_header_map` (`struct pos_validation.algorithm`)
//! when userspace mirrors a PoS-authenticated peer into the XDP allowlist via
//! [`crate::HyperMeshEbpf::set_peer_pos_validated`].
//!
//! The former userspace EXT_* structural pre-validators
//! (`ProofOfStateValidator` / `AssetHashValidator`) were removed with the F10
//! reframe: STOQ is encrypted QUIC, so the plaintext extension headers those
//! validators parsed were never present on the wire. Full cryptographic PoS
//! verification lives in TrustChain; the kernel gate admits by the
//! source-address allowlist those validators never touched.

/// FALCON-1024 signing algorithm indicator (HyperMesh default).
pub const ALG_FALCON_1024: u8 = 0x01;
/// Ed25519 signing algorithm indicator.
pub const ALG_ED25519: u8 = 0x02;
/// ECDSA signing algorithm indicator.
pub const ALG_ECDSA: u8 = 0x03;

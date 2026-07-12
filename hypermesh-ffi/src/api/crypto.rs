// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Direct (daemon-less) C ABI for the PoS-proof publish/verify seam.
//!
//! A `publish` of an asset payload is THREE distinct things that must never be
//! conflated:
//!
//! 1. **bytes / len** — the raw asset payload.
//! 2. **`blake3[32]`** — `BLAKE3(bytes)`, the CONTENT ADDRESS. This is what the
//!    mirror invariant (R4) binds to locally. It is NOT the signed preimage.
//! 3. **the FALCON signature** — a PoS PROOF signature in the [`WireSignedProof`]
//!    shape: FALCON-1024 over `BLAKE3(proof_bytes || nonce)`, where
//!    `proof_bytes` is the serialized four-proof [`StateProof`] and `nonce` is
//!    the proof's replay nonce. This is the SAME envelope the node uses on the
//!    wire (`TrustChainProofProvider::generate_proof` / `validate_proof` in
//!    `trustchain/src/proof_of_state/mod.rs`). It is NOT a signature over the
//!    raw asset bytes.
//!
//! [`hypermesh_publish`] emits BOTH #2 (the content address) and #3 (a full
//! [`WireSignedProof`]) so the two are produced together but stay separable:
//! the caller gets the 32-byte content hash AND a wire proof that interops
//! byte-for-byte with what the node already exchanges during handshakes.
//!
//! ## Why this construction is provably the wire proof, not a raw-bytes sign
//!
//! [`build_wire_signed_proof`] reproduces `TrustChainProofProvider::generate_proof`
//! step-for-step:
//!
//! 1. `StateProof::generate_from_network(node_id)` — build the four-proof
//!    StateProof (PoSpace/PoStake/PoWork/PoTime).
//! 2. `serde_json::to_vec(&proof)` → `proof_bytes`.
//! 3. random 32-byte `nonce`.
//! 4. `digest = BLAKE3(proof_bytes || nonce)`.
//! 5. `signature = identity.sign(digest)` — FALCON-1024 over the DIGEST, i.e.
//!    over `BLAKE3(proof_bytes || nonce)`, NEVER over the raw asset `bytes`.
//! 6. assemble [`WireSignedProof`] and `serde_json::to_vec` it.
//!
//! The raw asset `bytes` are used ONLY to compute the content address (#2).
//! They are never fed to `identity.sign`. This is the correction of the prior
//! (wrong) model that signed the raw payload with the identity key.
//!
//! ## PoS / secret-key boundary
//!
//! - Publishing REQUIRES the opaque `hypermesh_identity_t` handle; the FALCON
//!   secret key lives only inside that handle and is never copied across the C
//!   boundary.
//! - [`hypermesh_verify_proof`] is verify-only: it takes serialized wire bytes
//!   and runs the SAME check as `TrustChainProofProvider::validate_proof`. It
//!   holds no key and cannot fabricate a valid proof.

use crate::api::identity::{HmIdentity, HM_ERR_INTERNAL, HM_ERR_NULL};
use crate::error_state;
use crate::ffi_util::{catch_ffi, copy_into_out};
use hypermesh_lib::{compute_blake3_hash, NodeSigner};
use pqcrypto_falcon::falcon1024;
use std::os::raw::c_int;
use trustchain::proof_of_state::{StateProof, WireSignedProof};

/// Publish an asset payload: compute its content address AND produce a
/// FALCON-signed [`WireSignedProof`] for this identity.
///
/// This performs the THREE distinct operations of a publish without conflating
/// them (see module docs):
///
/// 1. `blake3_out = BLAKE3(bytes)` — the 32-byte content address (R4). Written
///    to `blake3_out` (must point to 32 writable bytes).
/// 2. Build a [`WireSignedProof`] — FALCON-1024 over `BLAKE3(proof_bytes ||
///    nonce)` where `proof_bytes` is the serialized four-proof StateProof — the
///    SAME wire envelope the node exchanges. Serialized (JSON) into `proof_out`
///    using the two-call length pattern.
///
/// The FALCON signature inside the proof is over the proof digest, NOT over the
/// raw `bytes`.
///
/// # Two-call pattern for `proof_out`
///
/// Pass `proof_out = NULL` (any `proof_cap`) to learn the required serialized
/// length via `*proof_len`, then call again with a buffer of that size. The
/// content address is still computed and written to `blake3_out` on the sizing
/// call, so a caller can obtain the hash and size in one pass.
///
/// IMPORTANT: every call regenerates a FRESH `WireSignedProof` (new nonce, new
/// timestamps, new system readings), so the serialized length can differ
/// slightly between the sizing call and the real call. A robust caller loops:
/// if the real call returns `HM_ERR_BUFFER_TOO_SMALL`, grow the buffer to the
/// newly reported `*proof_len` and retry (or simply oversize the buffer). The
/// content address in `blake3_out` is stable — only the proof envelope drifts.
///
/// # Returns
///
/// `HM_OK` on success (content hash written, proof serialized, `*proof_len` set),
/// `HM_ERR_BUFFER_TOO_SMALL` if `proof_out` is too small (`*proof_len` = required),
/// `HM_ERR_NULL` for a NULL required argument, or `HM_ERR_INTERNAL` on a crypto
/// or serialization failure (message in `hypermesh_last_error`).
///
/// # Safety
///
/// `identity` must be a valid handle; `bytes` must point to `len` readable
/// bytes (or be NULL iff `len == 0`); `blake3_out` must point to 32 writable
/// bytes; if `proof_out` is non-NULL it must have `proof_cap` writable bytes;
/// `proof_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_publish(
    identity: *const HmIdentity,
    bytes: *const u8,
    len: usize,
    blake3_out: *mut u8,
    proof_out: *mut u8,
    proof_cap: usize,
    proof_len: *mut usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        // --- #3 identity: secret key never crosses the boundary ---
        let id = match borrow_identity(identity) {
            Some(i) => i,
            None => return HM_ERR_NULL,
        };
        if blake3_out.is_null() {
            error_state::set_last_error("blake3_out is NULL".into());
            return HM_ERR_NULL;
        }
        if proof_len.is_null() {
            error_state::set_last_error("proof_len is NULL".into());
            return HM_ERR_NULL;
        }
        // #1 raw payload → #2 content address (BLAKE3(bytes)). This is the ONLY
        // use of the raw bytes; they are never signed.
        let payload = match slice_from_raw(bytes, len) {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        let content_hash = compute_blake3_hash(payload);
        std::ptr::copy_nonoverlapping(content_hash.as_bytes().as_ptr(), blake3_out, 32);

        // #3 PoS proof: identical construction to
        // TrustChainProofProvider::generate_proof (see build_wire_signed_proof).
        let wire_json = match build_wire_signed_proof(id) {
            Ok(w) => w,
            Err(e) => {
                error_state::set_last_error(format!("publish proof build: {e}"));
                return HM_ERR_INTERNAL;
            }
        };

        copy_into_out(&wire_json, proof_out, proof_cap, proof_len)
    })
}

/// Verify a [`WireSignedProof`] exactly as `TrustChainProofProvider::validate_proof`
/// does — FALCON verify over `BLAKE3(proof_bytes || nonce)` against the embedded
/// signer pubkey, then inner four-proof [`StateProof`] structural validation.
///
/// This is the contract-named verify seam. It delegates to the existing
/// [`crate::api::identity::hypermesh_signed_proof_verify`], which already
/// implements the exact `validate_proof` semantics — no duplicated logic.
///
/// Verify-only: holds no key, cannot mint. A forged or tampered proof returns
/// `HM_VERIFY_FAIL`.
///
/// # Returns
///
/// `HM_VERIFY_OK` (1) if authentic, `HM_VERIFY_FAIL` (0) if the signature or
/// inner proof is invalid, `HM_ERR_NULL` if `wire_proof` is NULL, or
/// `HM_ERR_INVALID` (negative) on malformed input.
///
/// # Safety
///
/// `wire_proof` must point to `wire_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_verify_proof(
    wire_proof: *const u8,
    wire_len: usize,
) -> c_int {
    // Single source of truth: the WireSignedProof verify already lives in the
    // identity module and mirrors validate_proof. Reuse it, don't duplicate.
    crate::api::identity::hypermesh_signed_proof_verify(wire_proof, wire_len)
}

/// Return the FALCON-1024 detached signature length in bytes (PQClean: 1280).
///
/// PQClean is authoritative for the concrete sizes:
/// - signature: 1280 bytes (this function)
/// - public key: 1793 bytes
/// - secret key: 2305 bytes
///
/// A consumer sizing a buffer for the FALCON signature inside a
/// [`WireSignedProof`] should use this rather than a hardcoded constant.
#[no_mangle]
pub extern "C" fn hypermesh_falcon_signature_max_len() -> usize {
    falcon1024::signature_bytes()
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

/// Build and JSON-serialize a [`WireSignedProof`] for `id`.
///
/// Reproduces `TrustChainProofProvider::generate_proof` step-for-step so the
/// emitted bytes are the SAME wire envelope the node exchanges:
///
/// 1. build the four-proof `StateProof` from network/system state,
/// 2. `serde_json` it → `proof_bytes`,
/// 3. random 32-byte `nonce`,
/// 4. `digest = BLAKE3(proof_bytes || nonce)`,
/// 5. `signature = id.sign(digest)` — FALCON over the DIGEST, not raw bytes,
/// 6. assemble `WireSignedProof` and `serde_json` it.
///
/// `StateProof::generate_from_network` is async; we drive it on a small
/// current-thread runtime local to this call (no daemon required).
fn build_wire_signed_proof(id: &HmIdentity) -> anyhow::Result<Vec<u8>> {
    let signer = id.identity();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("runtime: {e}"))?;

    // 1. Four-proof StateProof (WHO/WHEN/WHERE/WHAT).
    let proof = rt
        .block_on(StateProof::generate_from_network(signer.node_id()))
        .map_err(|e| anyhow::anyhow!("StateProof generation: {e}"))?;

    // 2. Serialize the inner StateProof as JSON (matches generate_proof).
    let proof_bytes = serde_json::to_vec(&proof)
        .map_err(|e| anyhow::anyhow!("serialize StateProof: {e}"))?;

    // 3. Random 32-byte nonce for replay prevention.
    let mut nonce = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

    // 4. digest = BLAKE3(proof_bytes || nonce).
    let mut hasher = blake3::Hasher::new();
    hasher.update(&proof_bytes);
    hasher.update(&nonce);
    let digest = hasher.finalize();

    // 5. FALCON-1024 sign the DIGEST with the identity secret key. The raw
    //    asset payload is NEVER signed — only this proof digest is.
    let signature = signer
        .sign(digest.as_bytes())
        .map_err(|e| anyhow::anyhow!("FALCON sign: {e}"))?;

    // 6. Assemble the wire envelope — identical shape to generate_proof.
    let wire = WireSignedProof {
        proof_bytes,
        signature,
        signer_pubkey: signer.public_key_bytes().to_vec(),
        nonce,
    };
    serde_json::to_vec(&wire).map_err(|e| anyhow::anyhow!("serialize WireSignedProof: {e}"))
}

unsafe fn borrow_identity<'a>(ptr: *const HmIdentity) -> Option<&'a HmIdentity> {
    if ptr.is_null() {
        error_state::set_last_error("identity handle is NULL".into());
        return None;
    }
    Some(&*ptr)
}

unsafe fn slice_from_raw<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        if len == 0 {
            return Some(&[]);
        }
        error_state::set_last_error("bytes is NULL".into());
        return None;
    }
    Some(std::slice::from_raw_parts(ptr, len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::identity::{
        hypermesh_identity_free, hypermesh_identity_generate, HM_ERR_BUFFER_TOO_SMALL, HM_OK,
        HM_VERIFY_FAIL, HM_VERIFY_OK,
    };

    #[test]
    fn falcon_sizes_are_pqclean_canonical() {
        assert_eq!(hypermesh_falcon_signature_max_len(), 1280, "FALCON-1024 sig");
        assert_eq!(falcon1024::public_key_bytes(), 1793, "FALCON-1024 pubkey");
        assert_eq!(falcon1024::secret_key_bytes(), 2305, "FALCON-1024 sk");
    }

    #[test]
    fn publish_null_identity_is_null() {
        let bytes = b"payload";
        let mut hash = [0u8; 32];
        let mut plen = 0usize;
        let rc = unsafe {
            hypermesh_publish(
                std::ptr::null(),
                bytes.as_ptr(),
                bytes.len(),
                hash.as_mut_ptr(),
                std::ptr::null_mut(),
                0,
                &mut plen,
            )
        };
        assert_eq!(rc, HM_ERR_NULL);
    }

    #[test]
    fn publish_emits_content_hash_and_verifiable_proof() {
        let h = hypermesh_identity_generate();
        let payload = b"the raw asset payload bytes";

        // Sizing call: content hash written, proof length reported.
        let mut hash = [0u8; 32];
        let mut plen = 0usize;
        let rc = unsafe {
            hypermesh_publish(
                h,
                payload.as_ptr(),
                payload.len(),
                hash.as_mut_ptr(),
                std::ptr::null_mut(),
                0,
                &mut plen,
            )
        };
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL, "sizing returns too-small");
        assert!(plen > 0, "proof length reported");

        // The content address is BLAKE3(bytes), independent of the proof.
        let expect = compute_blake3_hash(payload);
        assert_eq!(&hash, expect.as_bytes(), "blake3_out == BLAKE3(bytes)");

        // Real call. Each publish regenerates a FRESH proof (fresh nonce +
        // timestamps + system readings), so the serialized length can drift
        // between calls. Loop-grow the buffer until it fits — the robust
        // caller pattern documented on hypermesh_publish.
        let mut proof;
        let mut rc;
        loop {
            proof = vec![0u8; plen];
            rc = unsafe {
                hypermesh_publish(
                    h,
                    payload.as_ptr(),
                    payload.len(),
                    hash.as_mut_ptr(),
                    proof.as_mut_ptr(),
                    proof.len(),
                    &mut plen,
                )
            };
            if rc == HM_ERR_BUFFER_TOO_SMALL {
                // *plen now holds the (larger) required size; retry.
                continue;
            }
            break;
        }
        assert_eq!(rc, HM_OK);
        proof.truncate(plen);

        // The emitted proof is a WireSignedProof (interop shape) and verifies.
        let wire: WireSignedProof =
            serde_json::from_slice(&proof).expect("test: wire proof parses");
        // The FALCON sig is over BLAKE3(proof_bytes||nonce), NOT the raw bytes:
        // verifying it against the raw payload must FAIL, against the digest
        // must SUCCEED.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&wire.proof_bytes);
        hasher.update(&wire.nonce);
        let digest = hasher.finalize();
        assert!(
            trustchain::identity::FalconIdentity::verify_signature(
                &wire.signer_pubkey,
                digest.as_bytes(),
                &wire.signature,
            )
            .expect("test: verify digest"),
            "sig must verify over the proof digest"
        );
        assert!(
            !trustchain::identity::FalconIdentity::verify_signature(
                &wire.signer_pubkey,
                payload, // raw bytes — must NOT be the signed preimage
                &wire.signature,
            )
            .expect("test: verify raw"),
            "sig must NOT verify over the raw asset bytes (not the preimage)"
        );

        // verify_proof accepts the genuine proof.
        let rc = unsafe { hypermesh_verify_proof(proof.as_ptr(), proof.len()) };
        assert_eq!(rc, HM_VERIFY_OK, "genuine proof verifies");

        // Tamper the serialized proof → verify_proof rejects.
        let mut tampered = wire.clone();
        if let Some(b) = tampered.signature.get_mut(0) {
            *b ^= 0xFF;
        }
        let tampered_json = serde_json::to_vec(&tampered).expect("test: serialize");
        let rc = unsafe { hypermesh_verify_proof(tampered_json.as_ptr(), tampered_json.len()) };
        assert_eq!(rc, HM_VERIFY_FAIL, "tampered proof rejected");

        unsafe { hypermesh_identity_free(h) };
    }

    #[test]
    fn verify_proof_null_safe() {
        let rc = unsafe { hypermesh_verify_proof(std::ptr::null(), 0) };
        assert_eq!(rc, HM_ERR_NULL);
    }
}

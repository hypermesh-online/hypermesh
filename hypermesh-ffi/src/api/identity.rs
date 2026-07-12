// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Direct (daemon-less) C ABI for TrustChain identity and PoS-signed proofs.
//!
//! Unlike the `trustchain.*` IPC wrappers in [`super::trustchain`] (which
//! marshal JSON to a running daemon), these functions call the TrustChain
//! Rust crate directly, so an embedded / mobile / other-language consumer can:
//!
//! - Generate or load a post-quantum node identity (FALCON-1024 + Kyber-1024).
//! - Sign arbitrary data with the identity's FALCON-1024 secret key.
//! - Verify a FALCON-1024 detached signature (stateless).
//! - Verify a `WireSignedProof` envelope — the on-the-wire PoS-signed proof.
//!
//! ## PoS invariant preserved
//!
//! [`hypermesh_signed_proof_verify`] runs the EXACT same verification the
//! internal Rust path runs: it recomputes `BLAKE3(proof_bytes || nonce)`,
//! checks the FALCON-1024 detached signature against the embedded public key,
//! and then validates the inner four-proof `StateProof`. It returns `false`
//! for any tampered or forged proof. There is no path here that fabricates a
//! valid proof without the signing secret key (which only lives inside an
//! opaque `HmIdentity` handle and is never exposed over the ABI).

use crate::error_state;
use crate::ffi_util::{catch_ffi, copy_into_out, write_cstr_into};
use hypermesh_lib::NodeSigner;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::WireSignedProof;

/// FFI status codes returned by the direct identity/asset ABI.
///
/// `0` is success. Negative values are error classes; on any error the
/// thread-local message is also set (see `hypermesh_last_error`).
pub const HM_OK: c_int = 0;
/// A required pointer argument was NULL.
pub const HM_ERR_NULL: c_int = -1;
/// An argument was invalid (bad length, non-UTF-8, out of range, etc).
pub const HM_ERR_INVALID: c_int = -2;
/// The caller-provided output buffer was too small; required length is
/// written to the `*_len` out-parameter so the caller can retry.
pub const HM_ERR_BUFFER_TOO_SMALL: c_int = -3;
/// An underlying cryptographic or I/O operation failed.
pub const HM_ERR_INTERNAL: c_int = -4;
/// A verification check ran successfully but the result was "not authentic".
pub const HM_VERIFY_FAIL: c_int = 0;
/// A verification check ran successfully and the result was "authentic".
pub const HM_VERIFY_OK: c_int = 1;

/// Opaque handle wrapping a [`FalconIdentity`] (holds secret key material).
///
/// Exposed to C as `hypermesh_identity_t`. Never dereferenced by the caller.
/// Must be freed exactly once with [`hypermesh_identity_free`].
pub struct HmIdentity {
    inner: FalconIdentity,
}

/// Generate a fresh post-quantum identity (FALCON-1024 + Kyber-1024).
///
/// Returns an opaque handle the caller must free with
/// [`hypermesh_identity_free`], or `NULL` on error (see `hypermesh_last_error`).
#[no_mangle]
pub extern "C" fn hypermesh_identity_generate() -> *mut HmIdentity {
    catch_ffi(std::ptr::null_mut(), || {
        let inner = FalconIdentity::generate();
        Box::into_raw(Box::new(HmIdentity { inner }))
    })
}

/// Load an identity from `data_dir`, generating and persisting one if absent.
///
/// `data_dir` is a null-terminated UTF-8 path. Returns an opaque handle the
/// caller must free with [`hypermesh_identity_free`], or `NULL` on error.
///
/// # Safety
///
/// `data_dir` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_load(
    data_dir: *const c_char,
) -> *mut HmIdentity {
    catch_ffi(std::ptr::null_mut(), || {
        let dir = match ptr_to_str(data_dir, "data_dir") {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        match FalconIdentity::load_or_create(Path::new(dir)) {
            Ok(inner) => Box::into_raw(Box::new(HmIdentity { inner })),
            Err(e) => {
                error_state::set_last_error(format!("identity load failed: {e}"));
                std::ptr::null_mut()
            }
        }
    })
}

/// Free an identity handle. Passing `NULL` is a safe no-op.
///
/// # Safety
///
/// `identity` must be a pointer returned by `hypermesh_identity_generate` /
/// `hypermesh_identity_load`, or `NULL`. Must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_free(identity: *mut HmIdentity) {
    if !identity.is_null() {
        drop(Box::from_raw(identity));
    }
}

/// Write the node ID (64-char BLAKE3 hex of the FALCON public key) as a
/// null-terminated string into `out` (capacity `out_cap` bytes).
///
/// Returns `HM_OK` on success, `HM_ERR_BUFFER_TOO_SMALL` if `out_cap < 65`
/// (64 chars + NUL), or a negative error code.
///
/// # Safety
///
/// `identity` must be a valid handle; `out` must point to at least `out_cap`
/// writable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_node_id(
    identity: *const HmIdentity,
    out: *mut c_char,
    out_cap: usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let id = match borrow_identity(identity) {
            Some(i) => i,
            None => return HM_ERR_NULL,
        };
        write_cstr_into(id.inner.node_id.as_str(), out, out_cap)
    })
}

/// Copy the raw FALCON-1024 public key bytes into `out`.
///
/// Two-call pattern: pass `out = NULL` (any `out_cap`) to learn the required
/// length via `*out_len`, then call again with a buffer of that size.
///
/// Returns `HM_OK` on success (with `*out_len` = bytes written),
/// `HM_ERR_BUFFER_TOO_SMALL` if `out_cap` is too small (`*out_len` = required).
///
/// # Safety
///
/// `identity` must be valid; if non-NULL, `out` must have `out_cap` writable
/// bytes; `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_public_key(
    identity: *const HmIdentity,
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let id = match borrow_identity(identity) {
            Some(i) => i,
            None => return HM_ERR_NULL,
        };
        copy_into_out(id.inner.public_key_bytes(), out, out_cap, out_len)
    })
}

/// Copy the raw Kyber-1024 public key bytes into `out` (same two-call pattern
/// as [`hypermesh_identity_public_key`]). Peers use this to encrypt assets FOR
/// this node (KEM encapsulation).
///
/// # Safety
///
/// See [`hypermesh_identity_public_key`].
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_kyber_public_key(
    identity: *const HmIdentity,
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let id = match borrow_identity(identity) {
            Some(i) => i,
            None => return HM_ERR_NULL,
        };
        copy_into_out(&id.inner.kyber_public_key, out, out_cap, out_len)
    })
}

/// Sign `data` (`data_len` bytes) with the identity's FALCON-1024 secret key,
/// writing the detached signature into `out` (two-call pattern).
///
/// Returns `HM_OK` with `*out_len` = signature length on success.
///
/// # Safety
///
/// `identity` must be valid; `data` must have `data_len` readable bytes; if
/// `out` is non-NULL it must have `out_cap` writable bytes; `out_len` valid.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_identity_sign(
    identity: *const HmIdentity,
    data: *const u8,
    data_len: usize,
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let id = match borrow_identity(identity) {
            Some(i) => i,
            None => return HM_ERR_NULL,
        };
        let msg = match slice_from_raw(data, data_len, "data") {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        match id.inner.sign(msg) {
            Ok(sig) => copy_into_out(&sig, out, out_cap, out_len),
            Err(e) => {
                error_state::set_last_error(format!("signing failed: {e}"));
                HM_ERR_INTERNAL
            }
        }
    })
}

/// Verify a FALCON-1024 detached signature (stateless — no identity handle).
///
/// Returns `HM_VERIFY_OK` (1) if the signature is authentic for `data` under
/// `pubkey`, `HM_VERIFY_FAIL` (0) if not authentic, or a negative error code
/// on malformed input.
///
/// # Safety
///
/// `pubkey`/`data`/`signature` must point to their respective lengths of
/// readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_verify_signature(
    pubkey: *const u8,
    pubkey_len: usize,
    data: *const u8,
    data_len: usize,
    signature: *const u8,
    signature_len: usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let pk = match slice_from_raw(pubkey, pubkey_len, "pubkey") {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        let msg = match slice_from_raw(data, data_len, "data") {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        let sig = match slice_from_raw(signature, signature_len, "signature") {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        match FalconIdentity::verify_signature(pk, msg, sig) {
            Ok(true) => HM_VERIFY_OK,
            Ok(false) => HM_VERIFY_FAIL,
            Err(e) => {
                error_state::set_last_error(format!("verify_signature: {e}"));
                HM_ERR_INVALID
            }
        }
    })
}

/// Verify a `WireSignedProof` envelope — the on-the-wire PoS-signed proof.
///
/// `wire` is the JSON-serialized `WireSignedProof` (`wire_len` bytes). This
/// runs the SAME verification the internal Rust path runs:
/// 1. Recompute `BLAKE3(proof_bytes || nonce)`.
/// 2. Verify the FALCON-1024 detached signature against the embedded pubkey.
/// 3. Validate the inner four-proof `StateProof` (binary pass/fail).
///
/// Returns `HM_VERIFY_OK` (1) if authentic, `HM_VERIFY_FAIL` (0) if the
/// signature or inner proof is invalid, or a negative error on malformed input.
///
/// This is the PoS gate exposed over C: a forged or tampered proof returns 0,
/// and no ABI function can mint a valid proof without the signing secret key.
///
/// # Safety
///
/// `wire` must point to `wire_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_signed_proof_verify(
    wire: *const u8,
    wire_len: usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        if wire.is_null() {
            error_state::set_last_error("wire is NULL".into());
            return HM_ERR_NULL;
        }
        let bytes = match slice_from_raw(wire, wire_len, "wire") {
            Some(s) => s,
            None => return HM_ERR_NULL,
        };
        let wire_proof: WireSignedProof = match serde_json::from_slice(bytes) {
            Ok(w) => w,
            Err(e) => {
                error_state::set_last_error(format!("invalid WireSignedProof: {e}"));
                return HM_ERR_INVALID;
            }
        };
        match verify_wire_proof(&wire_proof) {
            Ok(true) => HM_VERIFY_OK,
            Ok(false) => HM_VERIFY_FAIL,
            Err(e) => {
                error_state::set_last_error(format!("proof verify: {e}"));
                HM_ERR_INVALID
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Full PoS verification of a wire proof: FALCON signature + inner StateProof.
///
/// Mirrors `TrustChainProofProvider::validate_proof` exactly.
fn verify_wire_proof(wire: &WireSignedProof) -> anyhow::Result<bool> {
    use trustchain::proof_of_state::StateProof;

    // 1. Recompute BLAKE3(proof_bytes || nonce)
    let mut hasher = blake3::Hasher::new();
    hasher.update(&wire.proof_bytes);
    hasher.update(&wire.nonce);
    let digest = hasher.finalize();

    // 2. Verify the FALCON-1024 detached signature against the embedded pubkey.
    let ok = FalconIdentity::verify_signature(
        &wire.signer_pubkey,
        digest.as_bytes(),
        &wire.signature,
    )?;
    if !ok {
        return Ok(false);
    }

    // 3. Validate the inner four-proof StateProof.
    let proof: StateProof = serde_json::from_slice(&wire.proof_bytes)
        .map_err(|e| anyhow::anyhow!("inner StateProof deserialize: {e}"))?;
    Ok(proof.validate())
}

unsafe fn borrow_identity<'a>(ptr: *const HmIdentity) -> Option<&'a HmIdentity> {
    if ptr.is_null() {
        error_state::set_last_error("identity handle is NULL".into());
        return None;
    }
    Some(&*ptr)
}

unsafe fn slice_from_raw<'a>(
    ptr: *const u8,
    len: usize,
    name: &str,
) -> Option<&'a [u8]> {
    if ptr.is_null() {
        if len == 0 {
            return Some(&[]);
        }
        error_state::set_last_error(format!("{name} is NULL"));
        return None;
    }
    Some(std::slice::from_raw_parts(ptr, len))
}

unsafe fn ptr_to_str<'a>(ptr: *const c_char, name: &str) -> Option<&'a str> {
    if ptr.is_null() {
        error_state::set_last_error(format!("{name} is NULL"));
        return None;
    }
    match std::ffi::CStr::from_ptr(ptr).to_str() {
        Ok(s) => Some(s),
        Err(e) => {
            error_state::set_last_error(format!("{name} not valid UTF-8: {e}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn generate_free_roundtrip() {
        let h = hypermesh_identity_generate();
        assert!(!h.is_null());
        unsafe { hypermesh_identity_free(h) };
    }

    #[test]
    fn node_id_written() {
        let h = hypermesh_identity_generate();
        let mut buf = [0i8; 128];
        let rc = unsafe {
            hypermesh_identity_node_id(h, buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, HM_OK);
        let s = unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_str()
            .expect("test: utf8");
        assert_eq!(s.len(), 64, "BLAKE3 hex node id is 64 chars");
        unsafe { hypermesh_identity_free(h) };
    }

    #[test]
    fn node_id_buffer_too_small() {
        let h = hypermesh_identity_generate();
        let mut buf = [0i8; 8];
        let rc = unsafe {
            hypermesh_identity_node_id(h, buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL);
        unsafe { hypermesh_identity_free(h) };
    }

    #[test]
    fn sign_then_verify_roundtrip() {
        let h = hypermesh_identity_generate();

        // Fetch public key (two-call).
        let mut pk_len = 0usize;
        let rc = unsafe {
            hypermesh_identity_public_key(h, std::ptr::null_mut(), 0, &mut pk_len)
        };
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL);
        let mut pk = vec![0u8; pk_len];
        let rc = unsafe {
            hypermesh_identity_public_key(h, pk.as_mut_ptr(), pk.len(), &mut pk_len)
        };
        assert_eq!(rc, HM_OK);

        // Sign.
        let msg = b"hypermesh ffi signing test";
        let mut sig_len = 0usize;
        let _ = unsafe {
            hypermesh_identity_sign(
                h, msg.as_ptr(), msg.len(), std::ptr::null_mut(), 0, &mut sig_len,
            )
        };
        let mut sig = vec![0u8; sig_len];
        let rc = unsafe {
            hypermesh_identity_sign(
                h, msg.as_ptr(), msg.len(), sig.as_mut_ptr(), sig.len(), &mut sig_len,
            )
        };
        assert_eq!(rc, HM_OK);
        sig.truncate(sig_len);

        // Verify: valid.
        let rc = unsafe {
            hypermesh_verify_signature(
                pk.as_ptr(), pk.len(), msg.as_ptr(), msg.len(),
                sig.as_ptr(), sig.len(),
            )
        };
        assert_eq!(rc, HM_VERIFY_OK);

        // Verify: tampered message must fail.
        let bad = b"tampered message";
        let rc = unsafe {
            hypermesh_verify_signature(
                pk.as_ptr(), pk.len(), bad.as_ptr(), bad.len(),
                sig.as_ptr(), sig.len(),
            )
        };
        assert_eq!(rc, HM_VERIFY_FAIL);

        unsafe { hypermesh_identity_free(h) };
    }

    #[test]
    fn null_handle_safe() {
        let mut buf = [0i8; 128];
        let rc = unsafe {
            hypermesh_identity_node_id(std::ptr::null(), buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, HM_ERR_NULL);
        unsafe { hypermesh_identity_free(std::ptr::null_mut()) };
    }

    #[test]
    fn signed_proof_verify_null_safe() {
        let rc = unsafe { hypermesh_signed_proof_verify(std::ptr::null(), 0) };
        assert_eq!(rc, HM_ERR_NULL);
    }

    #[test]
    fn signed_proof_verify_garbage_is_invalid() {
        let junk = b"not a wire signed proof";
        let rc = unsafe {
            hypermesh_signed_proof_verify(junk.as_ptr(), junk.len())
        };
        assert_eq!(rc, HM_ERR_INVALID);
    }

    #[test]
    fn signed_proof_verify_authentic_and_tampered() {
        // Build a genuine WireSignedProof the same way the Rust path does.
        use trustchain::proof_of_state::{StateProof, WireSignedProof};

        let id = FalconIdentity::generate();
        let proof = StateProof::new_for_testing();
        let proof_bytes = serde_json::to_vec(&proof).expect("test: serialize proof");

        let nonce = [7u8; 32];
        let mut hasher = blake3::Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();
        let signature = id.sign(digest.as_bytes()).expect("test: sign");

        let wire = WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: id.public_key.clone(),
            nonce,
        };
        let wire_json = serde_json::to_vec(&wire).expect("test: serialize wire");

        // Authentic proof verifies.
        let rc = unsafe {
            hypermesh_signed_proof_verify(wire_json.as_ptr(), wire_json.len())
        };
        assert_eq!(rc, HM_VERIFY_OK, "authentic proof must verify");

        // Tamper the signature — must fail (PoS gate).
        let mut tampered = wire.clone();
        if let Some(b) = tampered.signature.get_mut(0) {
            *b ^= 0xFF;
        }
        let tampered_json = serde_json::to_vec(&tampered).expect("test: serialize");
        let rc = unsafe {
            hypermesh_signed_proof_verify(tampered_json.as_ptr(), tampered_json.len())
        };
        assert_eq!(rc, HM_VERIFY_FAIL, "tampered proof must NOT verify");
    }
}

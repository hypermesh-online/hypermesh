// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Direct (daemon-less) C ABI for `hypermesh_lib::AssetAddress` and the
//! BLAKE3 content-hash mirror invariant.
//!
//! An `AssetAddress` is a 16-byte value:
//! `fd48:4d00` ULA prefix + matrix (x,y,z) i16 + 5-byte BLAKE3 fingerprint +
//! 4-bit shard index. These functions let external code construct, parse, and
//! inspect asset addresses, and verify content integrity, WITHOUT a running
//! daemon.
//!
//! ## Mirror invariant preserved
//!
//! [`hypermesh_verify_content_hash`] exposes the exact `BLAKE3(data) == hash`
//! check the internal pipeline uses (R4). Constructing an address embeds the
//! content hash into the address bytes, so an address is always bound to the
//! content it names. Nothing here stores or serves asset payloads: register /
//! fetch (which run the full validated, PoS-gated pipeline) remain behind the
//! daemon IPC functions `hypermesh_asset_store` / `hypermesh_asset_fetch`.

use crate::api::identity::{
    HM_ERR_INTERNAL, HM_ERR_INVALID, HM_ERR_NULL, HM_OK, HM_VERIFY_FAIL, HM_VERIFY_OK,
};
use crate::error_state;
use crate::ffi_util::{catch_ffi, write_cstr_into};
use hypermesh_lib::{compute_blake3_hash, validate_blake3_hash, AssetAddress, ContentHash};
use std::net::Ipv6Addr;
use std::os::raw::{c_char, c_int};
use std::str::FromStr;

/// Construct an `AssetAddress` from matrix coords, a 32-byte content hash, and
/// a shard index (0-15), writing the 16 address bytes into `out16`.
///
/// `x`/`y`/`z` must fit in i16 range `[-32768, 32767]`. Returns `HM_OK`,
/// `HM_ERR_INVALID` (coord overflow or shard > 15), or `HM_ERR_NULL`.
///
/// # Safety
///
/// `content_hash32` must point to 32 readable bytes; `out16` to 16 writable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_new(
    x: i64,
    y: i64,
    z: i64,
    content_hash32: *const u8,
    shard: u8,
    out16: *mut u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        if content_hash32.is_null() || out16.is_null() {
            error_state::set_last_error("content_hash32 or out16 is NULL".into());
            return HM_ERR_NULL;
        }
        let mut hash_bytes = [0u8; 32];
        std::ptr::copy_nonoverlapping(content_hash32, hash_bytes.as_mut_ptr(), 32);
        let hash = ContentHash::from_bytes(hash_bytes);

        match AssetAddress::with_shard(x, y, z, &hash, shard) {
            Ok(addr) => {
                std::ptr::copy_nonoverlapping(addr.as_bytes().as_ptr(), out16, 16);
                HM_OK
            }
            Err(e) => {
                error_state::set_last_error(format!("invalid asset address: {e}"));
                HM_ERR_INVALID
            }
        }
    })
}

/// Format the 16-byte address in `bytes16` as an IPv6 string, writing a
/// null-terminated string into `out` (capacity `out_cap`). A full IPv6 text
/// form fits in 46 bytes; pass at least 46.
///
/// # Safety
///
/// `bytes16` must point to 16 readable bytes; `out` to `out_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_to_ipv6(
    bytes16: *const u8,
    out: *mut c_char,
    out_cap: usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let addr = match addr_from_ptr(bytes16) {
            Some(a) => a,
            None => return HM_ERR_NULL,
        };
        let s = addr.to_ipv6().to_string();
        write_cstr_into(&s, out, out_cap)
    })
}

/// Parse an IPv6 string (`ipv6_str`) into the 16-byte address form (`out16`),
/// validating the HyperMesh `fd48:4d00` prefix.
///
/// Returns `HM_OK`, `HM_ERR_INVALID` (bad IPv6 or wrong prefix), or
/// `HM_ERR_NULL`.
///
/// # Safety
///
/// `ipv6_str` must be a valid null-terminated UTF-8 string; `out16` must point
/// to 16 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_from_ipv6(
    ipv6_str: *const c_char,
    out16: *mut u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        if ipv6_str.is_null() || out16.is_null() {
            error_state::set_last_error("ipv6_str or out16 is NULL".into());
            return HM_ERR_NULL;
        }
        let s = match std::ffi::CStr::from_ptr(ipv6_str).to_str() {
            Ok(s) => s,
            Err(e) => {
                error_state::set_last_error(format!("ipv6_str not valid UTF-8: {e}"));
                return HM_ERR_INVALID;
            }
        };
        let ip = match Ipv6Addr::from_str(s) {
            Ok(ip) => ip,
            Err(e) => {
                error_state::set_last_error(format!("invalid IPv6: {e}"));
                return HM_ERR_INVALID;
            }
        };
        match AssetAddress::from_ipv6(ip) {
            Ok(addr) => {
                std::ptr::copy_nonoverlapping(addr.as_bytes().as_ptr(), out16, 16);
                HM_OK
            }
            Err(e) => {
                error_state::set_last_error(format!("not a HyperMesh address: {e}"));
                HM_ERR_INVALID
            }
        }
    })
}

/// Extract the 6-byte asset fingerprint (bytes 10-15: 5 BLAKE3 hash bytes +
/// hash-nibble/shard byte) from `bytes16` into `out6`.
///
/// # Safety
///
/// `bytes16` must point to 16 readable bytes; `out6` to 6 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_fingerprint(
    bytes16: *const u8,
    out6: *mut u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let addr = match addr_from_ptr(bytes16) {
            Some(a) => a,
            None => return HM_ERR_NULL,
        };
        if out6.is_null() {
            error_state::set_last_error("out6 is NULL".into());
            return HM_ERR_NULL;
        }
        let fp = addr.asset_fingerprint();
        std::ptr::copy_nonoverlapping(fp.as_ptr(), out6, 6);
        HM_OK
    })
}

/// Extract the matrix coordinates (x,y,z) from `bytes16` into the three
/// out-params.
///
/// # Safety
///
/// `bytes16` must point to 16 readable bytes; `out_x`/`out_y`/`out_z` must be
/// valid pointers.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_coords(
    bytes16: *const u8,
    out_x: *mut i64,
    out_y: *mut i64,
    out_z: *mut i64,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let addr = match addr_from_ptr(bytes16) {
            Some(a) => a,
            None => return HM_ERR_NULL,
        };
        if out_x.is_null() || out_y.is_null() || out_z.is_null() {
            error_state::set_last_error("coord out-params must not be NULL".into());
            return HM_ERR_NULL;
        }
        let (x, y, z) = addr.matrix_coords();
        *out_x = x;
        *out_y = y;
        *out_z = z;
        HM_OK
    })
}

/// Return the shard index (0-15) of the address in `bytes16`, or a negative
/// error code (`HM_ERR_NULL`).
///
/// # Safety
///
/// `bytes16` must point to 16 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_shard_index(
    bytes16: *const u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let addr = match addr_from_ptr(bytes16) {
            Some(a) => a,
            None => return HM_ERR_NULL,
        };
        addr.shard_index() as c_int
    })
}

/// Return `HM_VERIFY_OK` (1) if the address in `bytes16` carries the HyperMesh
/// `fd48:4d00` prefix, `HM_VERIFY_FAIL` (0) if not, or `HM_ERR_NULL`.
///
/// # Safety
///
/// `bytes16` must point to 16 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_address_is_hypermesh(
    bytes16: *const u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        let addr = match addr_from_ptr(bytes16) {
            Some(a) => a,
            None => return HM_ERR_NULL,
        };
        if addr.is_hypermesh() { HM_VERIFY_OK } else { HM_VERIFY_FAIL }
    })
}

/// Compute the BLAKE3 content hash of `data` (`data_len` bytes) into `out32`.
///
/// This is the content-addressing primitive: the asset's identity is
/// `BLAKE3(payload)`. Use it to derive the `content_hash32` argument to
/// [`hypermesh_asset_address_new`].
///
/// # Safety
///
/// `data` must point to `data_len` readable bytes; `out32` to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_compute_content_hash(
    data: *const u8,
    data_len: usize,
    out32: *mut u8,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        if out32.is_null() {
            error_state::set_last_error("out32 is NULL".into());
            return HM_ERR_NULL;
        }
        if data.is_null() && data_len != 0 {
            error_state::set_last_error("data is NULL".into());
            return HM_ERR_NULL;
        }
        let slice = if data_len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(data, data_len)
        };
        let hash = compute_blake3_hash(slice);
        std::ptr::copy_nonoverlapping(hash.as_bytes().as_ptr(), out32, 32);
        HM_OK
    })
}

/// Verify the mirror invariant: `BLAKE3(data) == hash32` (R4).
///
/// Returns `HM_VERIFY_OK` (1) if the content matches the claimed hash,
/// `HM_VERIFY_FAIL` (0) if it does not, or `HM_ERR_NULL`.
///
/// This is the same integrity check the internal pipeline runs before trusting
/// any asset payload — exposed directly so C consumers cannot bypass it.
///
/// # Safety
///
/// `hash32` must point to 32 readable bytes; `data` to `data_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_verify_content_hash(
    hash32: *const u8,
    data: *const u8,
    data_len: usize,
) -> c_int {
    catch_ffi(HM_ERR_INTERNAL, || {
        if hash32.is_null() {
            error_state::set_last_error("hash32 is NULL".into());
            return HM_ERR_NULL;
        }
        if data.is_null() && data_len != 0 {
            error_state::set_last_error("data is NULL".into());
            return HM_ERR_NULL;
        }
        let mut hb = [0u8; 32];
        std::ptr::copy_nonoverlapping(hash32, hb.as_mut_ptr(), 32);
        let hash = ContentHash::from_bytes(hb);

        let slice = if data_len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(data, data_len)
        };
        if validate_blake3_hash(&hash, slice) {
            HM_VERIFY_OK
        } else {
            HM_VERIFY_FAIL
        }
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

unsafe fn addr_from_ptr(bytes16: *const u8) -> Option<AssetAddress> {
    if bytes16.is_null() {
        error_state::set_last_error("bytes16 is NULL".into());
        return None;
    }
    let mut buf = [0u8; 16];
    std::ptr::copy_nonoverlapping(bytes16, buf.as_mut_ptr(), 16);
    Some(AssetAddress::from_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn sample_hash() -> [u8; 32] {
        let mut h = [0u8; 32];
        for (i, b) in h.iter_mut().enumerate() {
            *b = i as u8;
        }
        h
    }

    #[test]
    fn new_then_parse_fields() {
        let hash = sample_hash();
        let mut addr = [0u8; 16];
        let rc = unsafe {
            hypermesh_asset_address_new(10, -20, 30, hash.as_ptr(), 3, addr.as_mut_ptr())
        };
        assert_eq!(rc, HM_OK);

        // is_hypermesh
        let rc = unsafe { hypermesh_asset_address_is_hypermesh(addr.as_ptr()) };
        assert_eq!(rc, HM_VERIFY_OK);

        // coords
        let (mut x, mut y, mut z) = (0i64, 0i64, 0i64);
        let rc = unsafe {
            hypermesh_asset_address_coords(addr.as_ptr(), &mut x, &mut y, &mut z)
        };
        assert_eq!(rc, HM_OK);
        assert_eq!((x, y, z), (10, -20, 30));

        // shard
        let s = unsafe { hypermesh_asset_address_shard_index(addr.as_ptr()) };
        assert_eq!(s, 3);

        // fingerprint = 5 hash bytes + (hash[5] high nibble | shard)
        let mut fp = [0u8; 6];
        let rc = unsafe {
            hypermesh_asset_address_fingerprint(addr.as_ptr(), fp.as_mut_ptr())
        };
        assert_eq!(rc, HM_OK);
        assert_eq!(&fp[..5], &hash[..5]);
        assert_eq!(fp[5], (hash[5] & 0xF0) | 3);
    }

    #[test]
    fn ipv6_roundtrip() {
        let hash = sample_hash();
        let mut addr = [0u8; 16];
        unsafe {
            hypermesh_asset_address_new(1, 2, 3, hash.as_ptr(), 0, addr.as_mut_ptr());
        }

        let mut buf = [0i8; 64];
        let rc = unsafe {
            hypermesh_asset_address_to_ipv6(addr.as_ptr(), buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, HM_OK);
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_str()
            .expect("test: utf8")
            .to_string();

        let cstr = CString::new(s).expect("test: cstring");
        let mut back = [0u8; 16];
        let rc = unsafe {
            hypermesh_asset_address_from_ipv6(cstr.as_ptr(), back.as_mut_ptr())
        };
        assert_eq!(rc, HM_OK);
        assert_eq!(addr, back);
    }

    #[test]
    fn from_ipv6_rejects_non_hypermesh() {
        let cstr = CString::new("2001:db8::1").expect("test: cstring");
        let mut back = [0u8; 16];
        let rc = unsafe {
            hypermesh_asset_address_from_ipv6(cstr.as_ptr(), back.as_mut_ptr())
        };
        assert_eq!(rc, HM_ERR_INVALID);
    }

    #[test]
    fn content_hash_compute_and_verify() {
        let data = b"the payload bytes";
        let mut hash = [0u8; 32];
        let rc = unsafe {
            hypermesh_compute_content_hash(data.as_ptr(), data.len(), hash.as_mut_ptr())
        };
        assert_eq!(rc, HM_OK);

        // Matches -> OK
        let rc = unsafe {
            hypermesh_verify_content_hash(hash.as_ptr(), data.as_ptr(), data.len())
        };
        assert_eq!(rc, HM_VERIFY_OK);

        // Tampered data -> FAIL (mirror invariant)
        let bad = b"the payload byteX";
        let rc = unsafe {
            hypermesh_verify_content_hash(hash.as_ptr(), bad.as_ptr(), bad.len())
        };
        assert_eq!(rc, HM_VERIFY_FAIL);
    }

    #[test]
    fn coord_overflow_rejected() {
        let hash = sample_hash();
        let mut addr = [0u8; 16];
        let rc = unsafe {
            hypermesh_asset_address_new(
                100_000, 0, 0, hash.as_ptr(), 0, addr.as_mut_ptr(),
            )
        };
        assert_eq!(rc, HM_ERR_INVALID);
    }

    #[test]
    fn null_pointers_safe() {
        let mut out = [0u8; 16];
        let rc = unsafe {
            hypermesh_asset_address_new(0, 0, 0, std::ptr::null(), 0, out.as_mut_ptr())
        };
        assert_eq!(rc, HM_ERR_NULL);

        let s = unsafe { hypermesh_asset_address_shard_index(std::ptr::null()) };
        assert_eq!(s, HM_ERR_NULL);

        let rc = unsafe {
            hypermesh_verify_content_hash(std::ptr::null(), std::ptr::null(), 0)
        };
        assert_eq!(rc, HM_ERR_NULL);
    }
}

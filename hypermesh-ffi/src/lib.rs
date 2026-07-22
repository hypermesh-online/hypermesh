// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! C FFI layer for the HyperMesh SDK.
//!
//! Provides an opaque handle (`hypermesh_client_t`) and a set of `extern "C"`
//! functions that language bindings (TypeScript/NAPI, Go/cgo, Python/ctypes,
//! C/C++/C#) can call directly.
//!
//! All returned strings are heap-allocated JSON and **must** be freed with
//! [`hypermesh_free_string`]. On error every function returns `NULL` and
//! stores a message retrievable via [`hypermesh_last_error`].

mod api;
mod error_state;
mod ffi_util;
mod handle;

pub use api::*;
pub use handle::{hypermesh_connect, hypermesh_disconnect};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Raw call
// ---------------------------------------------------------------------------

/// Send an arbitrary JSON-RPC call to the daemon.
///
/// `method` and `params_json` are null-terminated UTF-8 C strings.
/// Returns a JSON string the caller must free with `hypermesh_free_string`,
/// or `NULL` on error (inspect via `hypermesh_last_error`).
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `method` and `params_json` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_call(
    client: *mut handle::HypermeshClientHandle,
    method: *const c_char,
    params_json: *const c_char,
) -> *mut c_char {
    let h = match handle::borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };

    let method_str = match ptr_to_str(method, "method") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let params_str = match ptr_to_str(params_json, "params_json") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let params: serde_json::Value = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(e) => {
            error_state::set_last_error(format!("invalid params JSON: {e}"));
            return std::ptr::null_mut();
        }
    };

    match h.runtime.block_on(h.client.raw_call(method_str, params)) {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

/// Free a string previously returned by any `hypermesh_*` function.
///
/// # Safety
///
/// `s` must be a pointer previously returned by this library, or `NULL`.
/// Calling with any other pointer is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Return the last error message, or `NULL` if no error has occurred.
///
/// The returned pointer is valid until the next FFI call **on the same
/// thread**. The caller must NOT free this pointer.
///
/// # Safety
///
/// `_client` is accepted for API consistency but currently unused
/// (errors are thread-local). May be `NULL`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_last_error(
    _client: *const handle::HypermeshClientHandle,
) -> *const c_char {
    error_state::last_error_ptr()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a `*const c_char` to a `&str`, setting last_error on failure.
unsafe fn ptr_to_str<'a>(ptr: *const c_char, name: &str) -> Option<&'a str> {
    if ptr.is_null() {
        error_state::set_last_error(format!("{name} is NULL"));
        return None;
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => Some(s),
        Err(e) => {
            error_state::set_last_error(format!("{name} is not valid UTF-8: {e}"));
            None
        }
    }
}

/// Serialize a JSON value to a heap-allocated C string.
fn json_to_cstring(val: &serde_json::Value) -> *mut c_char {
    match serde_json::to_string(val) {
        Ok(s) => match CString::new(s) {
            Ok(cs) => cs.into_raw(),
            Err(e) => {
                error_state::set_last_error(format!("JSON contains interior NUL: {e}"));
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            error_state::set_last_error(format!("failed to serialize JSON: {e}"));
            std::ptr::null_mut()
        }
    }
}

/// Serialize a string to a heap-allocated C string.
fn str_to_cstring(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(e) => {
            error_state::set_last_error(format!("string contains interior NUL: {e}"));
            std::ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let val = serde_json::json!({"height": 42});
        let ptr = json_to_cstring(&val);
        assert!(!ptr.is_null());

        let cs = unsafe { std::ffi::CStr::from_ptr(ptr) };
        let parsed: serde_json::Value =
            serde_json::from_str(cs.to_str().expect("test: UTF-8"))
                .expect("test: parse JSON");
        assert_eq!(parsed["height"], 42);

        unsafe { hypermesh_free_string(ptr) };
    }

    #[test]
    fn str_roundtrip() {
        let ptr = str_to_cstring("hello");
        assert!(!ptr.is_null());

        let cs = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(cs.to_str().expect("test: UTF-8"), "hello");

        unsafe { hypermesh_free_string(ptr) };
    }

    #[test]
    fn free_null_is_safe() {
        unsafe { hypermesh_free_string(std::ptr::null_mut()) };
    }

    #[test]
    fn raw_call_null_handle() {
        error_state::clear_last_error();
        let method = CString::new("test").expect("test: CString");
        let params = CString::new("{}").expect("test: CString");
        let result = unsafe {
            hypermesh_call(std::ptr::null_mut(), method.as_ptr(), params.as_ptr())
        };
        assert!(result.is_null());
    }

    #[test]
    fn last_error_null_client_ok() {
        let ptr = unsafe { hypermesh_last_error(std::ptr::null()) };
        // May be null or not depending on prior state — just must not crash.
        let _ = ptr;
    }

    #[test]
    fn all_typed_functions_null_safe() {
        let null = std::ptr::null_mut();
        let c_str = CString::new("x").expect("test: CString");
        let p = c_str.as_ptr();

        unsafe {
            assert!(hypermesh_status(null).is_null());
            assert!(hypermesh_dns_resolve(null, p).is_null());
            assert!(hypermesh_dns_list(null).is_null());
            assert!(hypermesh_dns_register(null, p, p).is_null());
            assert!(hypermesh_peers(null).is_null());
            assert!(hypermesh_blockchain_height(null).is_null());
            assert!(hypermesh_blockchain_block(null, 0).is_null());
            assert!(hypermesh_topology_info(null).is_null());
            assert!(hypermesh_asset_list(null).is_null());
            assert!(hypermesh_asset_store(null, p).is_null());
            assert!(hypermesh_asset_fetch(null, p, p).is_null());
            assert!(hypermesh_domain_list(null).is_null());
            assert!(hypermesh_domain_register(null, p, p).is_null());
            assert!(hypermesh_dashboard_list(null).is_null());
            assert!(hypermesh_dashboard_deploy(null, p).is_null());
            assert!(hypermesh_config_show(null).is_null());
            assert!(hypermesh_config_get(null, p).is_null());
            // Caesar
            assert!(hypermesh_caesar_wallet(null).is_null());
            assert!(hypermesh_caesar_balance(null).is_null());
            assert!(hypermesh_caesar_transactions(null, 10).is_null());
            assert!(hypermesh_caesar_rewards(null).is_null());
            assert!(hypermesh_caesar_route_packet(null, p, 1.0).is_null());
            assert!(hypermesh_caesar_governor_params(null).is_null());
            // TrustChain
            assert!(hypermesh_trustchain_certificates(null).is_null());
            assert!(hypermesh_trustchain_issue(null, p, p).is_null());
            assert!(hypermesh_trustchain_validate(null, p).is_null());
            assert!(hypermesh_trustchain_revoke(null, p).is_null());
            assert!(hypermesh_trustchain_dns_zones(null).is_null());
            // NGauge
            assert!(hypermesh_ngauge_capacity(null).is_null());
            assert!(hypermesh_ngauge_traffic(null).is_null());
            assert!(hypermesh_ngauge_marketplace(null).is_null());
            assert!(hypermesh_ngauge_node_metrics(null).is_null());
            assert!(hypermesh_ngauge_leases(null).is_null());
            // Catalog
            assert!(hypermesh_catalog_browse(null, p, 0).is_null());
            assert!(hypermesh_catalog_browse(null, std::ptr::null(), 0).is_null());
            assert!(hypermesh_catalog_search(null, p).is_null());
            assert!(hypermesh_catalog_package_info(null, p).is_null());
            assert!(hypermesh_catalog_registry_stats(null).is_null());
        }
    }
}

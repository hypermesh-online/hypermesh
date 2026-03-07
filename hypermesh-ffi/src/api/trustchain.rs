// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! TrustChain CA FFI functions.
//!
//! All calls are forwarded to the blockmatrix daemon via JSON-RPC IPC
//! using `trustchain.*` method names. The daemon relays to TrustChain.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{json_to_cstring, ptr_to_str};
use std::os::raw::c_char;

/// List all TrustChain certificates. Returns JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_trustchain_certificates(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.raw_call("trustchain.certificates", serde_json::json!({})))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Issue a new certificate for the given subject and scope. Returns JSON.
///
/// `subject` is the entity name (e.g. node ID or domain).
/// `scope` is the certificate scope (e.g. "device", "network").
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `subject` and `scope` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_trustchain_issue(
    client: *mut HypermeshClientHandle,
    subject: *const c_char,
    scope: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let subject_str = match ptr_to_str(subject, "subject") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let scope_str = match ptr_to_str(scope, "scope") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"subject": subject_str, "scope": scope_str});
    match h
        .runtime
        .block_on(h.client.raw_call("trustchain.issue", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Validate a PEM-encoded certificate. Returns JSON validation result.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `cert_pem` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_trustchain_validate(
    client: *mut HypermeshClientHandle,
    cert_pem: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let pem_str = match ptr_to_str(cert_pem, "cert_pem") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"cert_pem": pem_str});
    match h
        .runtime
        .block_on(h.client.raw_call("trustchain.validate", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Revoke a certificate by its ID. Returns JSON result.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `cert_id` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_trustchain_revoke(
    client: *mut HypermeshClientHandle,
    cert_id: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let id_str = match ptr_to_str(cert_id, "cert_id") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"cert_id": id_str});
    match h
        .runtime
        .block_on(h.client.raw_call("trustchain.revoke", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// List TrustChain DNS zones. Returns JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_trustchain_dns_zones(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.raw_call("trustchain.dns_zones", serde_json::json!({})))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

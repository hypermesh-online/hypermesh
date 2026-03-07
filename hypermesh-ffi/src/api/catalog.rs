// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Catalog registry FFI functions.
//!
//! All calls are forwarded to the blockmatrix daemon via JSON-RPC IPC
//! using `catalog.*` method names. The daemon relays to the Catalog service.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{json_to_cstring, ptr_to_str};
use std::os::raw::c_char;

/// Browse catalog packages. Returns JSON with paginated results.
///
/// `query` is an optional search filter (may be NULL for unfiltered).
/// `page` is the zero-based page number.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `query` must be a valid null-terminated UTF-8 string or NULL.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_catalog_browse(
    client: *mut HypermeshClientHandle,
    query: *const c_char,
    page: u32,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let query_val = if query.is_null() {
        serde_json::Value::Null
    } else {
        match ptr_to_str(query, "query") {
            Some(s) => serde_json::Value::String(s.to_owned()),
            None => return std::ptr::null_mut(),
        }
    };
    let params = serde_json::json!({"query": query_val, "page": page});
    match h
        .runtime
        .block_on(h.client.raw_call("catalog.browse", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Search catalog packages by query string. Returns JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `query` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_catalog_search(
    client: *mut HypermeshClientHandle,
    query: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let query_str = match ptr_to_str(query, "query") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"query": query_str});
    match h
        .runtime
        .block_on(h.client.raw_call("catalog.search", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get detailed info about a specific catalog package. Returns JSON.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `name` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_catalog_package_info(
    client: *mut HypermeshClientHandle,
    name: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let name_str = match ptr_to_str(name, "name") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"name": name_str});
    match h
        .runtime
        .block_on(h.client.raw_call("catalog.package_info", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Fetch catalog registry statistics. Returns JSON.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_catalog_registry_stats(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.raw_call("catalog.registry_stats", serde_json::json!({})))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

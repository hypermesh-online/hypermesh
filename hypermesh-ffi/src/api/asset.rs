// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{ptr_to_str, str_to_cstring};
use super::to_json_ptr;
use std::os::raw::c_char;

/// List all stored assets as a JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_list(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h.runtime.block_on(h.client.asset().list()) {
        Ok(assets) => to_json_ptr(&assets),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Store a file as a HyperMesh asset. Returns the store result as JSON.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `file_path` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_store(
    client: *mut HypermeshClientHandle,
    file_path: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let path_str = match ptr_to_str(file_path, "file_path") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    match h.runtime.block_on(h.client.asset().store(path_str)) {
        Ok(result) => to_json_ptr(&result),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Fetch an asset by ID and write it to the output path.
/// Returns `"ok"` on success.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `asset_id` and `output_path` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_asset_fetch(
    client: *mut HypermeshClientHandle,
    asset_id: *const c_char,
    output_path: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let id_str = match ptr_to_str(asset_id, "asset_id") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let out_str = match ptr_to_str(output_path, "output_path") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.asset().fetch(id_str, out_str))
    {
        Ok(()) => str_to_cstring("\"ok\""),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

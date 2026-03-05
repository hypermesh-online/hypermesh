// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{json_to_cstring, ptr_to_str};
use std::os::raw::c_char;

/// List registered domains as a JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_domain_list(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.raw_call("domain.list", serde_json::json!({})))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Register a domain with the given name and privacy mode.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `name` and `privacy` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_domain_register(
    client: *mut HypermeshClientHandle,
    name: *const c_char,
    privacy: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let name_str = match ptr_to_str(name, "name") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let privacy_str = match ptr_to_str(privacy, "privacy") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"name": name_str, "privacy": privacy_str});
    match h
        .runtime
        .block_on(h.client.raw_call("domain.register", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

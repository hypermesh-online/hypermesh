// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{json_to_cstring, ptr_to_str};
use std::os::raw::c_char;

/// List deployed dashboards as a JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_dashboard_list(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.raw_call("dashboard.list", serde_json::json!({})))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Deploy a dashboard from the given path. Returns result as JSON.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `path` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_dashboard_deploy(
    client: *mut HypermeshClientHandle,
    path: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let path_str = match ptr_to_str(path, "path") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let params = serde_json::json!({"path": path_str});
    match h
        .runtime
        .block_on(h.client.raw_call("dashboard.deploy", params))
    {
        Ok(val) => json_to_cstring(&val),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

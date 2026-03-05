// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::{ptr_to_str, str_to_cstring};
use super::to_json_ptr;
use std::os::raw::c_char;

/// Resolve a DNS name. Returns the address as a C string.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `name` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_dns_resolve(
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
    match h.runtime.block_on(h.client.dns().resolve(name_str)) {
        Ok(addr) => str_to_cstring(&addr),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// List all DNS entries as a JSON array.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_dns_list(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h.runtime.block_on(h.client.dns().list()) {
        Ok(entries) => to_json_ptr(&entries),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Register a DNS name pointing to the given address.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
/// `name` and `addr` must be valid null-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_dns_register(
    client: *mut HypermeshClientHandle,
    name: *const c_char,
    addr: *const c_char,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let name_str = match ptr_to_str(name, "name") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let addr_str = match ptr_to_str(addr, "addr") {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    match h
        .runtime
        .block_on(h.client.dns().register(name_str, addr_str))
    {
        Ok(entry) => to_json_ptr(&entry),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

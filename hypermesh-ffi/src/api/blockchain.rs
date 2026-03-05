// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

use crate::error_state;
use crate::handle::{borrow_handle, HypermeshClientHandle};
use crate::json_to_cstring;
use super::to_json_ptr;
use std::os::raw::c_char;

/// Get the current blockchain height as a JSON number.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_blockchain_height(
    client: *mut HypermeshClientHandle,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h.runtime.block_on(h.client.blockchain().height()) {
        Ok(height) => {
            let val = serde_json::Value::Number(serde_json::Number::from(height));
            json_to_cstring(&val)
        }
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get a block by index. Returns block info as a JSON string.
///
/// # Safety
///
/// `client` must be a valid pointer returned by `hypermesh_connect`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_blockchain_block(
    client: *mut HypermeshClientHandle,
    index: u64,
) -> *mut c_char {
    let h = match borrow_handle(client) {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    match h.runtime.block_on(h.client.blockchain().block(index)) {
        Ok(block) => to_json_ptr(&block),
        Err(e) => {
            error_state::set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

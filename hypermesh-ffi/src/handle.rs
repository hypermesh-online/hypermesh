// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Opaque client handle and connection lifecycle.

use crate::error_state;
use hypermesh_sdk::{ConnectionMode, HyperMeshClient};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

/// Opaque handle exposed through the C API as `hypermesh_client_t`.
///
/// Contains the tokio runtime (owned) and the async SDK client.
pub struct HypermeshClientHandle {
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) client: HyperMeshClient,
}

/// Connect to a running HyperMesh daemon.
///
/// `socket_path` may be `NULL` to use the default 3-tier fallback
/// (`$HYPERMESH_SOCK` / `$XDG_RUNTIME_DIR/hypermesh/ctl.sock` /
/// `~/.hypermesh/ctl.sock`).
///
/// Returns an opaque pointer the caller must eventually pass to
/// `hypermesh_disconnect`, or `NULL` on error.
///
/// # Safety
///
/// `socket_path` must be a valid null-terminated UTF-8 string or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_connect(
    socket_path: *const c_char,
) -> *mut HypermeshClientHandle {
    let path_override: Option<PathBuf> = if socket_path.is_null() {
        None
    } else {
        match CStr::from_ptr(socket_path).to_str() {
            Ok(s) => Some(PathBuf::from(s)),
            Err(e) => {
                error_state::set_last_error(format!(
                    "socket_path is not valid UTF-8: {e}"
                ));
                return std::ptr::null_mut();
            }
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error_state::set_last_error(format!("failed to create tokio runtime: {e}"));
            return std::ptr::null_mut();
        }
    };

    let mode = ConnectionMode::Local {
        socket_path: path_override,
    };

    let client = match runtime.block_on(HyperMeshClient::connect(mode)) {
        Ok(c) => c,
        Err(e) => {
            error_state::set_last_error(format!("connection failed: {e}"));
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(HypermeshClientHandle { runtime, client }))
}

/// Disconnect and free a client handle.
///
/// After this call the pointer is invalid and must not be used.
///
/// # Safety
///
/// `client` must be a pointer previously returned by `hypermesh_connect`,
/// or `NULL` (which is a no-op).
#[no_mangle]
pub unsafe extern "C" fn hypermesh_disconnect(client: *mut HypermeshClientHandle) {
    if !client.is_null() {
        // Consume the Box, dropping both client and runtime.
        drop(Box::from_raw(client));
    }
}

/// Borrow the handle behind a raw pointer, setting last_error if NULL.
pub(crate) unsafe fn borrow_handle<'a>(
    ptr: *mut HypermeshClientHandle,
) -> Option<&'a HypermeshClientHandle> {
    if ptr.is_null() {
        error_state::set_last_error("client handle is NULL".into());
        return None;
    }
    Some(&*ptr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_null_socket_returns_null_with_error() {
        // Connecting to the default path will fail (no daemon running).
        let handle = unsafe { hypermesh_connect(std::ptr::null()) };
        assert!(handle.is_null());

        let err_ptr = unsafe { crate::hypermesh_last_error(std::ptr::null()) };
        assert!(!err_ptr.is_null());
    }

    #[test]
    fn disconnect_null_is_noop() {
        unsafe { hypermesh_disconnect(std::ptr::null_mut()) };
    }

    #[test]
    fn borrow_null_sets_error() {
        crate::error_state::clear_last_error();
        let result = unsafe { borrow_handle(std::ptr::null_mut()) };
        assert!(result.is_none());

        let err_ptr = crate::error_state::last_error_ptr();
        assert!(!err_ptr.is_null());
    }
}

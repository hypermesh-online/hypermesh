// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Thread-local error storage for the FFI layer.

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Store an error message for the current thread.
pub(crate) fn set_last_error(msg: String) {
    LAST_ERROR.with(|cell| {
        // Replace interior NUL bytes with '?' so CString never fails.
        let sanitized = msg.replace('\0', "?");
        *cell.borrow_mut() = CString::new(sanitized).ok();
    });
}

/// Return a pointer to the last error string for this thread, or NULL.
///
/// The pointer is valid until the next call to `set_last_error` on the
/// same thread.
pub(crate) fn last_error_ptr() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|cs| cs.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

/// Clear the last error for the current thread.
#[cfg(test)]
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_read_error() {
        clear_last_error();
        assert!(last_error_ptr().is_null());

        set_last_error("something broke".into());
        let ptr = last_error_ptr();
        assert!(!ptr.is_null());

        let msg = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .expect("test: valid UTF-8");
        assert_eq!(msg, "something broke");
    }

    #[test]
    fn interior_nul_sanitized() {
        set_last_error("bad\0byte".into());
        let ptr = last_error_ptr();
        assert!(!ptr.is_null());

        let msg = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .expect("test: valid UTF-8");
        assert_eq!(msg, "bad?byte");
    }

    #[test]
    fn overwrite_error() {
        set_last_error("first".into());
        set_last_error("second".into());
        let ptr = last_error_ptr();
        let msg = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .expect("test: valid UTF-8");
        assert_eq!(msg, "second");
    }
}

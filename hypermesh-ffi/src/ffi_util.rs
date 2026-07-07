// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Shared helpers for the direct (daemon-less) C ABI: panic isolation,
//! buffer-copy with the two-call length pattern, and C-string writing.

use crate::api::identity::{HM_ERR_BUFFER_TOO_SMALL, HM_ERR_INTERNAL, HM_ERR_NULL, HM_OK};
use crate::error_state;
use std::os::raw::{c_char, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Run `f` catching any panic at the FFI boundary, returning `on_panic` if it
/// unwinds. No Rust panic ever crosses into C.
pub(crate) fn catch_ffi<R, F: FnOnce() -> R>(on_panic: R, f: F) -> R {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            error_state::set_last_error("internal panic caught at FFI boundary".into());
            on_panic
        }
    }
}

/// Copy `src` into caller buffer `out` (capacity `out_cap`), using the
/// two-call length pattern.
///
/// Writes the required length to `*out_len` in all non-NULL-`out_len` cases.
/// - `out == NULL`: report required length, return `HM_ERR_BUFFER_TOO_SMALL`.
/// - `out_cap < src.len()`: report required length, return
///   `HM_ERR_BUFFER_TOO_SMALL` (nothing written).
/// - otherwise: copy `src.len()` bytes, set `*out_len = src.len()`, return
///   `HM_OK`.
///
/// # Safety-ish
///
/// The caller (an `extern "C"` fn) guarantees `out`/`out_len` validity for the
/// stated capacity. `out_len` NULL is treated as `HM_ERR_NULL`.
pub(crate) fn copy_into_out(
    src: &[u8],
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> c_int {
    if out_len.is_null() {
        error_state::set_last_error("out_len is NULL".into());
        return HM_ERR_NULL;
    }
    // Always report the required length.
    unsafe {
        *out_len = src.len();
    }
    if out.is_null() || out_cap < src.len() {
        return HM_ERR_BUFFER_TOO_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len());
    }
    HM_OK
}

/// Write `s` as a null-terminated C string into `out` (capacity `out_cap`).
///
/// Requires `out_cap >= s.len() + 1`. Returns `HM_OK`,
/// `HM_ERR_BUFFER_TOO_SMALL`, `HM_ERR_NULL`, or `HM_ERR_INTERNAL` (interior
/// NUL byte in `s`).
///
/// # Safety-ish
///
/// The caller guarantees `out` is valid for `out_cap` bytes.
pub(crate) fn write_cstr_into(s: &str, out: *mut c_char, out_cap: usize) -> c_int {
    if out.is_null() {
        error_state::set_last_error("out is NULL".into());
        return HM_ERR_NULL;
    }
    if s.as_bytes().contains(&0) {
        error_state::set_last_error("string contains interior NUL".into());
        return HM_ERR_INTERNAL;
    }
    let needed = s.len() + 1;
    if out_cap < needed {
        return HM_ERR_BUFFER_TOO_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(s.as_ptr(), out as *mut u8, s.len());
        *out.add(s.len()) = 0;
    }
    HM_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_reports_required_length_when_null() {
        let src = [1u8, 2, 3, 4];
        let mut len = 0usize;
        let rc = copy_into_out(&src, std::ptr::null_mut(), 0, &mut len);
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL);
        assert_eq!(len, 4);
    }

    #[test]
    fn copy_writes_when_fits() {
        let src = [9u8, 8, 7];
        let mut buf = [0u8; 3];
        let mut len = 0usize;
        let rc = copy_into_out(&src, buf.as_mut_ptr(), buf.len(), &mut len);
        assert_eq!(rc, HM_OK);
        assert_eq!(len, 3);
        assert_eq!(buf, [9, 8, 7]);
    }

    #[test]
    fn copy_too_small_writes_nothing() {
        let src = [1u8, 2, 3, 4];
        let mut buf = [0u8; 2];
        let mut len = 0usize;
        let rc = copy_into_out(&src, buf.as_mut_ptr(), buf.len(), &mut len);
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL);
        assert_eq!(len, 4);
        assert_eq!(buf, [0, 0]); // untouched
    }

    #[test]
    fn cstr_written_and_terminated() {
        let mut buf = [0i8; 8];
        let rc = write_cstr_into("abc", buf.as_mut_ptr(), buf.len());
        assert_eq!(rc, HM_OK);
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_str()
            .expect("test: utf8");
        assert_eq!(s, "abc");
    }

    #[test]
    fn cstr_too_small() {
        let mut buf = [0i8; 3];
        let rc = write_cstr_into("abc", buf.as_mut_ptr(), buf.len());
        assert_eq!(rc, HM_ERR_BUFFER_TOO_SMALL);
    }

    #[test]
    fn catch_ffi_isolates_panic() {
        let rc = catch_ffi(-99, || panic!("boom"));
        assert_eq!(rc, -99);
    }
}

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Typed C API wrappers organized by domain.

mod asset;
mod blockchain;
mod caesar;
mod catalog;
mod config;
mod dashboard;
mod dns;
mod domain;
mod engauge;
mod network;
mod node;
mod topology;
mod trustchain;

// Re-export all extern "C" functions so they appear in the shared library.
pub use asset::*;
pub use blockchain::*;
pub use caesar::*;
pub use catalog::*;
pub use config::*;
pub use dashboard::*;
pub use dns::*;
pub use domain::*;
pub use engauge::*;
pub use network::*;
pub use node::*;
pub use topology::*;
pub use trustchain::*;

use crate::error_state;
use std::os::raw::c_char;

/// Serialize a value to a heap-allocated JSON C string.
/// Sets last_error and returns NULL on failure.
pub(crate) fn to_json_ptr<T: serde::Serialize>(val: &T) -> *mut c_char {
    match serde_json::to_value(val) {
        Ok(v) => crate::json_to_cstring(&v),
        Err(e) => {
            error_state::set_last_error(format!("serialization: {e}"));
            std::ptr::null_mut()
        }
    }
}

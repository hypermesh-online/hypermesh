// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Typed C API wrappers organized by domain.
//!
//! Two families live here:
//! - IPC proxies (`asset`, `trustchain`, `caesar`, ...) marshal JSON to a
//!   running daemon.
//! - Direct crate ABIs (`identity`, `asset_address`) call the TrustChain /
//!   lib Rust paths in-process, so consumers can build cert/asset primitives
//!   without a daemon. These preserve the PoS and BLAKE3 mirror invariants.

mod asset;
pub mod asset_address;
mod blockchain;
mod caesar;
mod catalog;
mod config;
pub mod crypto;
mod dashboard;
mod dns;
mod domain;
mod ngauge;
pub mod identity;
mod network;
mod node;
mod topology;
mod trustchain;

// Re-export all extern "C" functions so they appear in the shared library.
pub use asset::*;
pub use asset_address::*;
pub use blockchain::*;
pub use caesar::*;
pub use catalog::*;
pub use config::*;
pub use crypto::*;
pub use dashboard::*;
pub use dns::*;
pub use domain::*;
pub use ngauge::*;
pub use identity::*;
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

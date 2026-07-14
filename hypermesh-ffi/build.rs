// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Build script: regenerate the C header (`include/hypermesh.h`) from the
//! crate's `extern "C"` surface via cbindgen.
//!
//! This is best-effort: if header generation fails for any reason (e.g. a
//! locked-down deploy build), we emit a `cargo:warning` and continue so that
//! `cargo build -p hypermesh-ffi` — and the musl static-pie deploy build —
//! never break because of header generation.

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => {
            println!("cargo:warning=CARGO_MANIFEST_DIR unset; skipping header gen");
            return;
        }
    };

    // Rebuild the header when the ABI source or config changes.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/api/identity.rs");
    println!("cargo:rerun-if-changed=src/api/asset_address.rs");
    println!("cargo:rerun-if-changed=src/api/crypto.rs");
    println!("cargo:rerun-if-changed=src/ffi_util.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let config = match cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=cbindgen config load failed: {e}; skipping header gen");
            return;
        }
    };

    let out_path = crate_dir.join("include").join("hypermesh.h");

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            if !bindings.write_to_file(&out_path) {
                // write_to_file returns false when the file was already
                // up-to-date; that is fine, not an error.
            }
        }
        Err(e) => {
            println!("cargo:warning=cbindgen header generation failed: {e}");
        }
    }
}

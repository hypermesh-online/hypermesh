// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Build script for Catalog extension
//!
//! This script configures the build process to create both a regular library
//! and a dynamic library (.so) that can be loaded by HyperMesh.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is always set by cargo"));
    let target_dir = out_dir
        .parent()
        .expect("OUT_DIR has parent")
        .parent()
        .expect("OUT_DIR grandparent exists")
        .parent()
        .expect("OUT_DIR great-grandparent exists")
        .to_path_buf();

    // cdylib link args removed — catalog is now rlib-only (no dynamic library target)

    // Create extension manifest file
    let manifest = r#"[metadata]
id = "catalog"
name = "HyperMesh Catalog Extension"
version = "1.0.0"
description = "Decentralized asset library and VM runtime for HyperMesh"
author = "HyperMesh Team"
license = "MIT"
homepage = "https://hypermesh.online/catalog"
category = "AssetLibrary"
hypermesh_version = "1.0.0"

[library]
name = "catalog"
lib_type = "native"
entry_point = "hypermesh_extension_create"

[security]
certificate = "SHA256:catalog_cert_fingerprint"
permissions = [
    "AssetManagement",
    "VMExecution",
    "NetworkAccess",
    "StateProofAccess",
    "TransportAccess",
    "FileSystemAccess"
]

[runtime]
min_version = "1.0.0"

[runtime.resources]
min_memory = 256
max_memory = 2048
cpu_cores = 2.0
"#;

    // Write manifest to output directory
    let manifest_path = target_dir.join("extension.toml");
    fs::write(&manifest_path, manifest).expect("Failed to write extension manifest");

    // Version script for symbol visibility removed — no cdylib target

    // Inform Cargo about rerun conditions
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_MANIFEST_DIR");

    // Set metadata for the build
    println!(
        "cargo:rustc-env=CATALOG_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );

    // Get build timestamp using standard library (chrono not available in build script)
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_secs()
    );
}

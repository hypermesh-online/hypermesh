//! Build script for Catalog extension
//!
//! This script configures the build process to create both a regular library
//! and a dynamic library (.so) that can be loaded by HyperMesh.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf();

    // Set up linking flags for creating a shared library
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libcatalog.so");
        println!("cargo:rustc-cdylib-link-arg=-Wl,--version-script={}/version.script", env::var("CARGO_MANIFEST_DIR").unwrap());
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libcatalog.dylib");
    }

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
    "ConsensusAccess",
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

    // Create version script for Linux to control symbol visibility
    if cfg!(target_os = "linux") {
        let version_script = r#"{
    global:
        hypermesh_extension_create;
        hypermesh_extension_destroy;
        hypermesh_extension_metadata;
    local:
        *;
};
"#;

        let script_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("version.script");
        fs::write(&script_path, version_script).expect("Failed to write version script");
    }

    // Inform Cargo about rerun conditions
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_MANIFEST_DIR");

    // Set metadata for the build
    println!("cargo:rustc-env=CATALOG_VERSION={}", env!("CARGO_PKG_VERSION"));

    // Get build timestamp using standard library (chrono not available in build script)
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}",
             std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs());
}
// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Build script for the hypermesh-ebpf crate.
//!
//! When the `ebpf-loader` feature is enabled and `clang` is available,
//! compiles all `.c` files in `programs/` to BPF object files in
//! `../target/bpf/`. Gracefully degrades when clang is missing.

fn main() {
    #[cfg(feature = "ebpf-loader")]
    compile_ebpf_programs();
}

#[cfg(feature = "ebpf-loader")]
fn compile_ebpf_programs() {
    use std::path::Path;
    use std::process::Command;

    let source_dir = Path::new("programs");
    let out_dir = Path::new("../target/bpf");

    if !source_dir.exists() {
        println!("cargo:warning=eBPF programs directory not found, skipping compilation");
        return;
    }

    // Check for clang availability
    if Command::new("clang").arg("--version").output().is_err() {
        println!("cargo:warning=clang not found, skipping eBPF compilation");
        return;
    }

    if let Err(e) = std::fs::create_dir_all(out_dir) {
        println!(
            "cargo:warning=Failed to create output dir {:?}: {}",
            out_dir, e
        );
        return;
    }

    let entries = match std::fs::read_dir(source_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("cargo:warning=Failed to read programs directory: {}", e);
            return;
        }
    };

    // Collect candidate include dirs that actually exist. Debian ships
    // arch headers under /usr/include/x86_64-linux-gnu; Arch/Fedora put
    // everything under /usr/include. Only pass dirs that exist so clang
    // does not warn about missing search paths.
    let candidate_includes = [
        "/usr/include",
        "/usr/include/bpf",
        "/usr/include/x86_64-linux-gnu",
    ];
    let include_args: Vec<String> = candidate_includes
        .iter()
        .filter(|d| Path::new(d).exists())
        .map(|d| format!("-I{d}"))
        .collect();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "c") {
            continue;
        }

        let stem = match path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => continue,
        };
        let output = out_dir.join(format!("{}.o", stem));

        println!("cargo:rerun-if-changed={}", path.display());

        // `-g` emits BTF, which aya 0.12 (aya-obj 0.2.1) REQUIRES to parse the
        // object — without it, load fails with "no BTF parsed for object".
        let status = Command::new("clang")
            .args(["-O2", "-g", "-target", "bpf", "-c"])
            .arg(&path)
            .arg("-o")
            .arg(&output)
            .args(&include_args)
            .arg("-D__TARGET_ARCH_x86")
            .status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "cargo:warning=Compiled eBPF: {} -> {}",
                    path.display(),
                    output.display()
                );
            }
            Ok(_) => {
                println!(
                    "cargo:warning=Failed to compile eBPF program: {}",
                    path.display()
                );
            }
            Err(e) => {
                println!(
                    "cargo:warning=clang invocation failed for {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }
}

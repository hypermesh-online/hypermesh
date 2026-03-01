// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Program Compiler and Loader
//!
//! Handles compilation from C source and loading of eBPF programs
//! into the kernel. All C program sources live in the `programs/`
//! directory of this crate.
//!
//! The `ebpf-loader` feature gates the compile-from-C functionality
//! which requires `clang` on the system.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// eBPF program source configuration
pub struct EbpfSources {
    /// Directory containing C source files
    pub source_dir: PathBuf,
    /// Output directory for compiled .o files
    pub output_dir: PathBuf,
}

impl Default for EbpfSources {
    fn default() -> Self {
        Self {
            source_dir: PathBuf::from("hypermesh-ebpf/programs"),
            output_dir: PathBuf::from("target/bpf"),
        }
    }
}

/// eBPF program type for identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramType {
    /// XDP program (packet filtering at NIC)
    Xdp,
    /// TC (Traffic Control) program
    Tc,
    /// Kprobe (kernel function tracing)
    Kprobe,
    /// Tracepoint (static kernel trace point)
    Tracepoint,
    /// Cgroup (resource control)
    Cgroup,
    /// Socket filter
    SocketFilter,
}

impl ProgramType {
    /// Human-readable name for this program type
    pub fn name(&self) -> &'static str {
        match self {
            Self::Xdp => "XDP",
            Self::Tc => "TC",
            Self::Kprobe => "Kprobe",
            Self::Tracepoint => "Tracepoint",
            Self::Cgroup => "Cgroup",
            Self::SocketFilter => "SocketFilter",
        }
    }
}

/// eBPF program loader and compiler.
///
/// Compiles C source files from `programs/` into BPF bytecode objects
/// and loads them into the kernel via aya.
pub struct EbpfLoader {
    sources: EbpfSources,
    /// Whether programs have been loaded into the kernel
    programs_loaded: bool,
    /// Loaded BPF handle (only with kernel-attach feature)
    #[cfg(feature = "kernel-attach")]
    bpf: Option<aya::Bpf>,
}

impl EbpfLoader {
    /// Create a new loader with default source paths
    pub fn new() -> Self {
        Self::with_sources(EbpfSources::default())
    }

    /// Create a loader with custom source paths
    pub fn with_sources(sources: EbpfSources) -> Self {
        Self {
            sources,
            programs_loaded: false,
            #[cfg(feature = "kernel-attach")]
            bpf: None,
        }
    }

    /// Compile all eBPF programs from source.
    ///
    /// Requires `clang` to be installed on the system. Compiles each
    /// `.c` file in the source directory into a `.o` BPF object file.
    #[cfg(feature = "ebpf-loader")]
    pub fn compile(&mut self) -> Result<()> {
        std::fs::create_dir_all(&self.sources.output_dir)?;

        if !Self::check_clang() {
            return Err(anyhow!(
                "clang not found. Install with: apt install clang llvm"
            ));
        }

        // Compile all C files in source directory
        let source_dir = &self.sources.source_dir;
        if !source_dir.exists() {
            tracing::warn!("eBPF source directory {:?} not found", source_dir);
            return Ok(());
        }

        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "c") {
                let stem = path
                    .file_stem()
                    .ok_or_else(|| anyhow!("No file stem for {:?}", path))?;
                let output = self
                    .sources
                    .output_dir
                    .join(format!("{}.o", stem.to_string_lossy()));
                self.compile_single(&path, &output)?;
            }
        }

        tracing::info!("eBPF programs compiled successfully");
        Ok(())
    }

    #[cfg(not(feature = "ebpf-loader"))]
    pub fn compile(&mut self) -> Result<()> {
        Err(anyhow!(
            "ebpf-loader feature not enabled. Enable it to compile from C source."
        ))
    }

    /// Compile a single C file to BPF bytecode
    #[cfg(feature = "ebpf-loader")]
    fn compile_single(&self, source: &Path, output: &Path) -> Result<()> {
        if !source.exists() {
            tracing::warn!("eBPF source {:?} not found, skipping", source);
            return Ok(());
        }

        let source_str = source
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in source path: {:?}", source))?;
        let output_str = output
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in output path: {:?}", output))?;

        let status = Command::new("clang")
            .args([
                "-O2",
                "-target",
                "bpf",
                "-c",
                source_str,
                "-o",
                output_str,
                "-I/usr/include",
                "-I/usr/include/bpf",
                "-D__TARGET_ARCH_x86",
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to compile eBPF program {:?}", source));
        }

        tracing::info!("Compiled {:?} -> {:?}", source, output);
        Ok(())
    }

    /// Check if clang is available on the system
    pub fn check_clang() -> bool {
        Command::new("clang").arg("--version").output().is_ok()
    }

    /// Load compiled eBPF programs into the kernel.
    ///
    /// With `kernel-attach`: uses aya to load BPF object into kernel.
    /// Without: logs a message and returns Ok (graceful degradation).
    pub fn load(&mut self) -> Result<()> {
        let xdp_path = self.sources.output_dir.join("hypermesh_xdp.o");
        let legacy_path = self.sources.output_dir.join("stoq_xdp.o");

        let load_path = if xdp_path.exists() {
            xdp_path.clone()
        } else if legacy_path.exists() {
            tracing::info!("Using legacy stoq_xdp.o (rename to hypermesh_xdp.o recommended)");
            legacy_path
        } else {
            // Try to compile first
            let _ = self.compile();

            if xdp_path.exists() {
                xdp_path
            } else {
                tracing::warn!("No compiled eBPF bytecode found. Run compile_ebpf.sh to build.");
                return Ok(());
            }
        };

        #[cfg(feature = "kernel-attach")]
        {
            match aya::Bpf::load_file(&load_path) {
                Ok(bpf) => {
                    self.bpf = Some(bpf);
                    self.programs_loaded = true;
                    tracing::info!("eBPF program loaded from {:?}", load_path);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load eBPF from {:?}: {}. Userspace fallback.",
                        load_path,
                        e
                    );
                }
            }
        }

        #[cfg(not(feature = "kernel-attach"))]
        {
            let _ = load_path; // suppress unused warning
            tracing::info!("kernel-attach feature not enabled, skipping kernel load");
        }

        Ok(())
    }

    /// Check if programs are loaded into the kernel
    pub fn are_programs_loaded(&self) -> bool {
        self.programs_loaded
    }

    /// Get the output path for the main XDP object
    pub fn output_path(&self) -> PathBuf {
        self.sources.output_dir.join("hypermesh_xdp.o")
    }

    /// Take ownership of the loaded BPF handle
    #[cfg(feature = "kernel-attach")]
    pub fn take_bpf(&mut self) -> Option<aya::Bpf> {
        self.bpf.take()
    }

    /// Verify an eBPF program before loading (uses bpftool if available)
    pub fn verify(&self, program_path: &Path) -> Result<()> {
        if !program_path.exists() {
            return Err(anyhow!("Program file not found: {program_path:?}"));
        }

        let path_str = program_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in path: {program_path:?}"))?;

        if let Ok(output) = Command::new("bpftool")
            .args(["prog", "load", path_str, "/sys/fs/bpf/test_verify"])
            .output()
        {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Program verification failed: {stderr}"));
            }

            // Clean up test program
            let _ = Command::new("rm").arg("/sys/fs/bpf/test_verify").status();
        }

        Ok(())
    }

    /// Get source directory path
    pub fn source_dir(&self) -> &Path {
        &self.sources.source_dir
    }
}

impl Default for EbpfLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = EbpfLoader::new();
        assert!(loader
            .sources
            .output_dir
            .to_str()
            .expect("test: output dir to str")
            .contains("bpf"));
        assert!(!loader.are_programs_loaded());
    }

    #[test]
    fn test_program_types() {
        assert_eq!(ProgramType::Xdp.name(), "XDP");
        assert_eq!(ProgramType::Tc.name(), "TC");
        assert_eq!(ProgramType::Kprobe.name(), "Kprobe");
        assert_eq!(ProgramType::Tracepoint.name(), "Tracepoint");
        assert_eq!(ProgramType::Cgroup.name(), "Cgroup");
        assert_eq!(ProgramType::SocketFilter.name(), "SocketFilter");
    }

    #[test]
    fn test_custom_sources() {
        let sources = EbpfSources {
            source_dir: PathBuf::from("/tmp/ebpf_src"),
            output_dir: PathBuf::from("/tmp/ebpf_out"),
        };
        let loader = EbpfLoader::with_sources(sources);
        assert_eq!(loader.source_dir(), Path::new("/tmp/ebpf_src"));
    }

    #[test]
    fn test_verify_missing_file() {
        let loader = EbpfLoader::new();
        let result = loader.verify(Path::new("/nonexistent/program.o"));
        assert!(result.is_err());
    }
}

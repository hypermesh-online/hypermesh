// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Rust code analysis for asset requirements and error translation.
//!
//! Analyzes Rust source code to determine:
//! - CPU requirements (parallelism, async)
//! - GPU requirements (compute libraries)
//! - Memory requirements (data structures)
//! - Storage requirements (file I/O)
//!
//! Also translates Rust compiler errors into user-friendly messages.

use anyhow::Result;

use super::super::super::{
    AssetRequirements, CpuRequirements, GpuRequirements, MemoryRequirements,
    StorageRequirements, MemoryAccessPattern, TranslatedError, ErrorCategory,
};

/// Analyze Rust code for asset requirements.
pub fn analyze_rust_asset_requirements(code: &str) -> Result<AssetRequirements> {
    let cpu_requirements = analyze_cpu_needs(code);
    let gpu_requirements = analyze_gpu_needs(code);
    let memory_requirements = Some(analyze_memory_needs(code));
    let storage_requirements = analyze_storage_needs(code);

    Ok(AssetRequirements {
        cpu_requirements,
        gpu_requirements,
        memory_requirements,
        storage_requirements,
        network_requirements: None,
    })
}

fn analyze_cpu_needs(code: &str) -> Option<CpuRequirements> {
    if code.contains("rayon") || code.contains("std::thread")
        || code.contains("tokio") || code.contains("async")
    {
        Some(CpuRequirements {
            min_cores: 2,
            preferred_cores: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(4),
            architecture: Some("x86_64".to_string()),
            required_features: vec!["multithread".to_string()],
        })
    } else {
        None
    }
}

fn analyze_gpu_needs(code: &str) -> Option<GpuRequirements> {
    if code.contains("novarc") || code.contains("wgpu")
        || code.contains("vulkano") || code.contains("opencl")
    {
        Some(GpuRequirements {
            min_memory_bytes: 1024 * 1024 * 1024,
            compute_capability: Some("5.0".to_string()),
            gpu_types: vec!["nvidia".to_string(), "amd".to_string()],
        })
    } else {
        None
    }
}

fn analyze_memory_needs(code: &str) -> MemoryRequirements {
    let memory_multiplier = if code.contains("Vec") || code.contains("HashMap") {
        if code.contains("Box") || code.contains("Rc") || code.contains("Arc") {
            3
        } else {
            2
        }
    } else {
        1
    };

    let base_memory: u64 = 128 * 1024 * 1024; // 128MB base
    let estimated_memory = base_memory * memory_multiplier;

    MemoryRequirements {
        min_ram_bytes: estimated_memory / 2,
        preferred_ram_bytes: estimated_memory,
        access_patterns: vec![MemoryAccessPattern::Sequential],
    }
}

fn analyze_storage_needs(code: &str) -> Option<StorageRequirements> {
    if code.contains("std::fs") || code.contains("serde")
        || code.contains("bincode") || code.contains("persist")
    {
        Some(StorageRequirements {
            min_storage_bytes: 100 * 1024 * 1024,
            storage_types: vec!["ssd".to_string()],
            io_patterns: vec!["sequential".to_string()],
        })
    } else {
        None
    }
}

/// Translate Rust compiler errors into user-friendly messages.
pub fn translate_rust_error(error: &str) -> Result<TranslatedError> {
    let error_category = categorize_error(error);
    let translated_error = translate_message(error, &error_category);
    let suggested_fixes = suggest_fixes(&error_category);

    Ok(TranslatedError {
        original_error: error.to_string(),
        translated_error,
        error_category: error_category.clone(),
        suggested_fixes,
        consensus_issues: if matches!(error_category, ErrorCategory::ConsensusError) {
            vec!["Consensus validation failed in Rust code".to_string()]
        } else {
            vec![]
        },
    })
}

fn categorize_error(error: &str) -> ErrorCategory {
    if error.contains("error[E") {
        if error.contains("E0425") || error.contains("E0412") {
            ErrorCategory::SyntaxError
        } else if error.contains("E0277") || error.contains("E0308") {
            ErrorCategory::RuntimeError
        } else {
            ErrorCategory::SyntaxError
        }
    } else if error.contains("consensus") {
        ErrorCategory::ConsensusError
    } else if error.contains("memory") || error.contains("allocation") {
        ErrorCategory::ResourceError
    } else {
        ErrorCategory::RuntimeError
    }
}

fn translate_message(error: &str, category: &ErrorCategory) -> String {
    match category {
        ErrorCategory::SyntaxError => {
            if error.contains("E0425") {
                "Variable or function not found - check spelling and imports".to_string()
            } else if error.contains("E0412") {
                "Type not found - check type names and use statements".to_string()
            } else {
                "Rust compilation error - check syntax and types".to_string()
            }
        }
        ErrorCategory::RuntimeError => {
            if error.contains("E0277") {
                "Trait not implemented - implement required traits".to_string()
            } else {
                "Type mismatch - check function signatures and variable types".to_string()
            }
        }
        ErrorCategory::ConsensusError => {
            "Consensus validation failed - ensure proper proofs".to_string()
        }
        ErrorCategory::ResourceError => {
            "Memory allocation failed - reduce memory usage".to_string()
        }
        _ => error.to_string(),
    }
}

fn suggest_fixes(category: &ErrorCategory) -> Vec<String> {
    match category {
        ErrorCategory::SyntaxError => vec![
            "Check variable and function names".to_string(),
            "Verify all imports with 'use' statements".to_string(),
            "Check Rust edition compatibility".to_string(),
        ],
        ErrorCategory::RuntimeError => vec![
            "Check trait implementations".to_string(),
            "Verify type annotations".to_string(),
            "Check function signatures".to_string(),
        ],
        ErrorCategory::ConsensusError => vec![
            "Verify consensus proofs meet minimum requirements".to_string(),
            "Check asset allocations".to_string(),
            "Ensure proper attribute usage".to_string(),
        ],
        _ => vec!["Check Rust documentation for details".to_string()],
    }
}

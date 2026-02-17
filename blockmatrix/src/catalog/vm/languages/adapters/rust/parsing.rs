// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Rust consensus construct parsing and code analysis.
//!
//! Parses Rust source code for consensus-related constructs:
//! - `#[consensus_required]` attributes
//! - `consensus_validate!` macro invocations
//! - Asset struct instantiations (CpuAsset, GpuAsset, etc.)
//! - `remote_execute` P2P calls

use anyhow::Result;

use super::super::super::{
    ConsensusConstruct, ConsensusConstructType, SourceLocation,
    ProofRequirement, AssetDependency, AssetAccessPattern,
};

/// Parse Rust-specific consensus constructs from source code.
pub fn parse_rust_consensus_constructs(code: &str) -> Result<Vec<ConsensusConstruct>> {
    let mut constructs = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if line.trim().starts_with("#[consensus_required") {
            constructs.push(parse_consensus_attribute(line, line_num as u32)?);
        }

        if line.contains("consensus_validate!") {
            constructs.push(parse_consensus_macro(line, line_num as u32)?);
        }

        if line.contains("CpuAsset::new") || line.contains("GpuAsset::new")
            || line.contains("MemoryAsset::new") || line.contains("StorageAsset::new")
        {
            constructs.push(parse_asset_operation(line, line_num as u32)?);
        }

        if line.contains("remote_execute") {
            constructs.push(parse_p2p_execution(line, line_num as u32)?);
        }
    }

    Ok(constructs)
}

/// Parse `#[consensus_required]` attribute with parameters.
fn parse_consensus_attribute(line: &str, line_num: u32) -> Result<ConsensusConstruct> {
    let mut required_proofs = Vec::new();

    if let Some(params_start) = line.find('(') {
        if let Some(params_end) = line.find(')') {
            let params = &line[params_start + 1..params_end];
            for param in params.split(',') {
                let parts: Vec<&str> = param.split('=').collect();
                if parts.len() == 2 {
                    let proof_type = parts[0].trim();
                    let value: u64 = parts[1].trim().parse().unwrap_or(0);

                    let mut minimum_values = std::collections::HashMap::new();
                    minimum_values.insert(proof_type.to_string(), value);

                    required_proofs.push(ProofRequirement {
                        proof_type: proof_type.to_string(),
                        minimum_values,
                        constraints: vec!["rust_validated".to_string()],
                    });
                }
            }
        }
    }

    Ok(ConsensusConstruct {
        construct_type: ConsensusConstructType::ConsensusFunction,
        source_location: SourceLocation {
            line: line_num + 1,
            column: 1,
            length: line.len() as u32,
            text: line.to_string(),
        },
        required_proofs,
        asset_dependencies: vec![],
    })
}

/// Parse `consensus_validate!` macro invocation.
fn parse_consensus_macro(line: &str, line_num: u32) -> Result<ConsensusConstruct> {
    Ok(ConsensusConstruct {
        construct_type: ConsensusConstructType::ConsensusFunction,
        source_location: SourceLocation {
            line: line_num + 1,
            column: 1,
            length: line.len() as u32,
            text: line.to_string(),
        },
        required_proofs: vec![ProofRequirement {
            proof_type: "all".to_string(),
            minimum_values: std::collections::HashMap::new(),
            constraints: vec!["macro_validated".to_string()],
        }],
        asset_dependencies: vec![],
    })
}

/// Parse asset operation (CpuAsset::new, GpuAsset::new, etc.).
fn parse_asset_operation(line: &str, line_num: u32) -> Result<ConsensusConstruct> {
    let asset_type = if line.contains("CpuAsset") {
        "cpu"
    } else if line.contains("GpuAsset") {
        "gpu"
    } else if line.contains("MemoryAsset") {
        "memory"
    } else if line.contains("StorageAsset") {
        "storage"
    } else {
        "generic"
    };

    let asset_dependencies = vec![AssetDependency {
        asset_type: asset_type.to_string(),
        minimum_amount: 1024,
        access_pattern: AssetAccessPattern::Exclusive,
    }];

    Ok(ConsensusConstruct {
        construct_type: ConsensusConstructType::AssetOperation,
        source_location: SourceLocation {
            line: line_num + 1,
            column: 1,
            length: line.len() as u32,
            text: line.to_string(),
        },
        required_proofs: vec![ProofRequirement {
            proof_type: "space".to_string(),
            minimum_values: {
                let mut map = std::collections::HashMap::new();
                map.insert("space".to_string(), 1024);
                map
            },
            constraints: vec!["asset_validated".to_string()],
        }],
        asset_dependencies,
    })
}

/// Parse `remote_execute` P2P call.
fn parse_p2p_execution(line: &str, line_num: u32) -> Result<ConsensusConstruct> {
    Ok(ConsensusConstruct {
        construct_type: ConsensusConstructType::P2PExecution,
        source_location: SourceLocation {
            line: line_num + 1,
            column: 1,
            length: line.len() as u32,
            text: line.to_string(),
        },
        required_proofs: vec![ProofRequirement {
            proof_type: "stake".to_string(),
            minimum_values: {
                let mut map = std::collections::HashMap::new();
                map.insert("stake".to_string(), 1000);
                map
            },
            constraints: vec!["p2p_validated".to_string()],
        }],
        asset_dependencies: vec![AssetDependency {
            asset_type: "network".to_string(),
            minimum_amount: 1024,
            access_pattern: AssetAccessPattern::Shared,
        }],
    })
}

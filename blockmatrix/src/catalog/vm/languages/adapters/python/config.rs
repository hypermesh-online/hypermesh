// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Python adapter configuration types

use serde::{Serialize, Deserialize};
use super::super::LanguageSpecificConfig;

/// Python-specific adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonAdapterConfig {
    /// Python executable path
    pub python_executable: Option<String>,
    /// Python virtual environment path
    pub venv_path: Option<String>,
    /// Required Python packages
    pub required_packages: Vec<String>,
    /// Python PYTHONPATH additions
    pub python_path: Vec<String>,
    /// Enable numpy integration
    pub enable_numpy: bool,
    /// Enable scipy integration
    pub enable_scipy: bool,
    /// Enable pandas integration
    pub enable_pandas: bool,
    /// Enable machine learning packages
    pub enable_ml_packages: bool,
    /// Consensus-specific Python modules
    pub consensus_modules: Vec<String>,
}

impl PythonAdapterConfig {
    pub fn from_language_config(config: Option<&LanguageSpecificConfig>) -> Self {
        if let Some(lang_config) = config {
            Self {
                python_executable: lang_config.runtime_path.clone(),
                venv_path: None,
                required_packages: vec![
                    "numpy".to_string(),
                    "scipy".to_string(),
                    "pandas".to_string(),
                ],
                python_path: vec![],
                enable_numpy: true,
                enable_scipy: true,
                enable_pandas: true,
                enable_ml_packages: false, // Conservative default
                consensus_modules: vec![
                    "hypermesh_consensus".to_string(),
                    "asset_management".to_string(),
                ],
            }
        } else {
            Self::default()
        }
    }
}

impl Default for PythonAdapterConfig {
    fn default() -> Self {
        Self {
            python_executable: None, // Will use system default
            venv_path: None,
            required_packages: vec![
                "numpy".to_string(),
                "scipy".to_string(),
                "pandas".to_string(),
                "requests".to_string(),
            ],
            python_path: vec![],
            enable_numpy: true,
            enable_scipy: true,
            enable_pandas: true,
            enable_ml_packages: false,
            consensus_modules: vec![
                "hypermesh_consensus".to_string(),
                "asset_management".to_string(),
                "p2p_execution".to_string(),
                "blockchain_storage".to_string(),
            ],
        }
    }
}

//! Validation Traits
//!
//! Trait definitions for extensible validation system.

use anyhow::Result;
use async_trait::async_trait;
use crate::assets::AssetPackage;
use super::results::{SecurityValidationResult, SyntaxValidationResult};

/// Type-specific validator trait
#[async_trait]
pub trait TypeValidator: Send + Sync {
    /// Get validator name
    fn name(&self) -> &str;

    /// Get supported asset types
    fn supported_types(&self) -> Vec<String>;

    /// Validate asset syntax
    async fn validate_syntax(&self, asset: &AssetPackage) -> Result<SyntaxValidationResult>;

    /// Check if validator supports asset type
    fn supports(&self, asset_type: &str) -> bool {
        self.supported_types().iter().any(|t| t == asset_type)
    }
}

/// Security scanner trait
#[async_trait]
pub trait SecurityScanner: Send + Sync {
    /// Get scanner name
    fn name(&self) -> &str;

    /// Get scanner capabilities
    fn capabilities(&self) -> Vec<String>;

    /// Perform security scan
    async fn scan(&self, asset: &AssetPackage) -> Result<SecurityValidationResult>;
}
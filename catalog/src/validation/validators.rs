//! Type-Specific Validators
//!
//! REMOVED: Language-specific validators moved to remote nodes.
//! Catalog is a package manager, not a syntax validator.
//! Runtime requirements stored in metadata, validation happens on remote execution nodes.

use anyhow::Result;
use async_trait::async_trait;

// Use local Catalog AssetPackage
use crate::assets::AssetPackage;
use super::traits::TypeValidator;
use super::results::SyntaxValidationResult;

// All language-specific validators removed
// Syntax validation happens on remote HyperMesh nodes during execution
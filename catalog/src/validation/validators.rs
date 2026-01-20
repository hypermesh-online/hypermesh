//! Type-Specific Validators
//!
//! Implementations of validators for specific asset types.
//! Migrated to use Asset Registry architecture with BlockMatrix Assets.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

// Use local Catalog AssetPackage
use crate::assets::AssetPackage;
use super::traits::TypeValidator;
use super::results::{
    SyntaxValidationResult, SyntaxError, StyleViolation,
    BestPracticeViolation, LintingIssue, CodeLocation
};
use super::config::LintSeverity;

/// Lua language validator
pub struct LuaValidator;

impl LuaValidator {
    /// Create new Lua validator
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypeValidator for LuaValidator {
    fn name(&self) -> &str {
        "LuaValidator"
    }

    fn supported_types(&self) -> Vec<String> {
        vec![
            "lua-module".to_string(),
            "lua-script".to_string(),
            "lua-library".to_string(),
        ]
    }

    async fn validate_syntax(&self, asset: &AssetPackage) -> Result<SyntaxValidationResult> {
        let mut errors = Vec::new();
        let mut style_violations = Vec::new();
        let mut best_practices = Vec::new();
        let mut linting_issues = Vec::new();

        // Check for Lua syntax errors from BlockMatrix Asset metadata
        // Use content from AssetPackage
        if let Some(code) = Some(&asset.content.main_content) {
            let code_str = code.as_str();

            // Check for balanced do-end blocks
            let do_count = code_str.matches("do").count();
            let end_count = code_str.matches("end").count();

            if do_count != end_count {
                errors.push(SyntaxError {
                    message: format!("Mismatched do-end blocks: {} do, {} end", do_count, end_count),
                    location: CodeLocation {
                        file: asset.package_hash.clone(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    error_code: Some("LUA001".to_string()),
                    fix_suggestion: Some("Check do-end block matching".to_string()),
                });
            }

            // Check for proper function definitions
            if code_str.contains("function") && !code_str.contains("end") {
                errors.push(SyntaxError {
                    message: "Function definition missing 'end'".to_string(),
                    location: CodeLocation {
                        file: asset.package_hash.clone(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    error_code: Some("LUA002".to_string()),
                    fix_suggestion: Some("Add 'end' keyword to close function".to_string()),
                });
            }

            // Style checks
            if code_str.contains("\t") {
                style_violations.push(StyleViolation {
                    rule: "no-tabs".to_string(),
                    description: "Use spaces instead of tabs".to_string(),
                    location: CodeLocation {
                        file: asset.package_hash.clone(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    auto_fixable: true,
                });
            }

            // Best practices
            if code_str.contains("_G[") {
                best_practices.push(BestPracticeViolation {
                    practice: "avoid-global-access".to_string(),
                    description: "Avoid direct global table access".to_string(),
                    location: CodeLocation {
                        file: asset.package_hash.clone(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    recommendation: "Use local variables where possible".to_string(),
                    impact: "performance".to_string(),
                });
            }

            // Linting
            let lines: Vec<&str> = code_str.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.len() > 120 {
                    linting_issues.push(LintingIssue {
                        rule_id: "line-too-long".to_string(),
                        message: format!("Line {} exceeds 120 characters", i + 1),
                        location: CodeLocation {
                            file: asset.package_hash.clone(),
                            line: Some((i + 1) as u32),
                            column: Some(120),
                            snippet: Some(line.to_string()),
                        },
                        severity: LintSeverity::Warning,
                        auto_fixable: false,
                    });
                }
            }
        }

        let total_issues = errors.len() + style_violations.len()
            + best_practices.len() + linting_issues.len();

        Ok(SyntaxValidationResult {
            valid: errors.is_empty(),
            errors,
            style_violations,
            best_practices,
            linting_issues,
            total_issues,
        })
    }
}
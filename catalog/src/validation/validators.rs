//! Type-Specific Validators
//!
//! Implementations of validators for specific asset types.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::assets::Asset;
use super::traits::TypeValidator;
use super::results::{
    SyntaxValidationResult, SyntaxError, StyleViolation,
    BestPracticeViolation, LintingIssue, CodeLocation
};
use super::config::LintSeverity;

/// Julia language validator
pub struct JuliaValidator;

impl JuliaValidator {
    /// Create new Julia validator
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypeValidator for JuliaValidator {
    fn name(&self) -> &str {
        "JuliaValidator"
    }

    fn supported_types(&self) -> Vec<String> {
        vec![
            "julia-package".to_string(),
            "julia-module".to_string(),
            "julia-script".to_string(),
        ]
    }

    async fn validate_syntax(&self, asset: &Asset) -> Result<SyntaxValidationResult> {
        let mut errors = Vec::new();
        let mut style_violations = Vec::new();
        let mut best_practices = Vec::new();
        let mut linting_issues = Vec::new();

        // Check for Julia syntax errors
        if let Some(code) = &asset.metadata.get("code") {
            let code_str = code.as_str().unwrap_or("");

            // Check for balanced parentheses
            let mut paren_count = 0;
            for (i, ch) in code_str.chars().enumerate() {
                match ch {
                    '(' => paren_count += 1,
                    ')' => {
                        paren_count -= 1;
                        if paren_count < 0 {
                            errors.push(SyntaxError {
                                message: "Unmatched closing parenthesis".to_string(),
                                location: CodeLocation {
                                    file: asset.id.to_string(),
                                    line: Some((i / 80) as u32 + 1),
                                    column: Some((i % 80) as u32 + 1),
                                    snippet: None,
                                },
                                error_code: Some("JL001".to_string()),
                                fix_suggestion: Some("Check parenthesis matching".to_string()),
                            });
                        }
                    }
                    _ => {}
                }
            }

            if paren_count > 0 {
                errors.push(SyntaxError {
                    message: "Unclosed parenthesis".to_string(),
                    location: CodeLocation {
                        file: asset.id.to_string(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    error_code: Some("JL002".to_string()),
                    fix_suggestion: Some("Add closing parenthesis".to_string()),
                });
            }

            // Check for proper function definitions
            if code_str.contains("function") && !code_str.contains("end") {
                errors.push(SyntaxError {
                    message: "Function definition missing 'end'".to_string(),
                    location: CodeLocation {
                        file: asset.id.to_string(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    error_code: Some("JL003".to_string()),
                    fix_suggestion: Some("Add 'end' keyword to close function".to_string()),
                });
            }

            // Style checks
            if code_str.contains("  ") {
                style_violations.push(StyleViolation {
                    rule: "no-double-spaces".to_string(),
                    description: "Avoid multiple consecutive spaces".to_string(),
                    location: CodeLocation {
                        file: asset.id.to_string(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    auto_fixable: true,
                });
            }

            // Best practices
            if code_str.contains("eval(") {
                best_practices.push(BestPracticeViolation {
                    practice: "avoid-eval".to_string(),
                    description: "Avoid using eval() for security reasons".to_string(),
                    location: CodeLocation {
                        file: asset.id.to_string(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    recommendation: "Use alternative approaches without eval".to_string(),
                    impact: "security".to_string(),
                });
            }

            // Linting
            if code_str.len() > 10000 {
                linting_issues.push(LintingIssue {
                    rule_id: "file-too-large".to_string(),
                    message: "File exceeds recommended size".to_string(),
                    location: CodeLocation {
                        file: asset.id.to_string(),
                        line: None,
                        column: None,
                        snippet: None,
                    },
                    severity: LintSeverity::Warning,
                    auto_fixable: false,
                });
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

    async fn validate_syntax(&self, asset: &Asset) -> Result<SyntaxValidationResult> {
        let mut errors = Vec::new();
        let mut style_violations = Vec::new();
        let mut best_practices = Vec::new();
        let mut linting_issues = Vec::new();

        // Check for Lua syntax errors
        if let Some(code) = &asset.metadata.get("code") {
            let code_str = code.as_str().unwrap_or("");

            // Check for balanced do-end blocks
            let do_count = code_str.matches("do").count();
            let end_count = code_str.matches("end").count();

            if do_count != end_count {
                errors.push(SyntaxError {
                    message: format!("Mismatched do-end blocks: {} do, {} end", do_count, end_count),
                    location: CodeLocation {
                        file: asset.id.to_string(),
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
                        file: asset.id.to_string(),
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
                        file: asset.id.to_string(),
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
                        file: asset.id.to_string(),
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
                            file: asset.id.to_string(),
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
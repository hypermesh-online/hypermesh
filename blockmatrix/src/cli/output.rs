// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI output types and formatting
//!
//! Provides structured output representations (`Text`, `Table`, `Json`) and
//! error types for CLI command execution. All formatting is done via `Display`
//! implementations so callers can simply `println!("{}", output)`.

use serde::Serialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Structured output from a CLI command.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CliOutput {
    /// Free-form text output.
    Text(String),
    /// Tabular output with headers and rows.
    Table(CliTable),
    /// Pre-serialized JSON string.
    Json(String),
}

impl fmt::Display for CliOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliOutput::Text(text) => write!(f, "{text}"),
            CliOutput::Table(table) => write!(f, "{table}"),
            CliOutput::Json(json) => write!(f, "{json}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

/// A simple table with column headers and rows of string values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CliTable {
    /// Column header names.
    pub headers: Vec<String>,
    /// Row data (each inner `Vec` has the same length as `headers`).
    pub rows: Vec<Vec<String>>,
}

impl CliTable {
    /// Create a new table with the given headers and no rows.
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    /// Append a row. Returns `Err` if the row length does not match headers.
    pub fn add_row(&mut self, row: Vec<String>) -> Result<(), CliError> {
        if row.len() != self.headers.len() {
            return Err(CliError::InvalidArgument(format!(
                "Row has {} columns but table has {} headers",
                row.len(),
                self.headers.len(),
            )));
        }
        self.rows.push(row);
        Ok(())
    }

    /// Number of data rows (excludes the header).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Compute the maximum display width for each column.
    fn column_widths(&self) -> Vec<usize> {
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() && cell.len() > widths[i] {
                    widths[i] = cell.len();
                }
            }
        }
        widths
    }
}

impl fmt::Display for CliTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.headers.is_empty() {
            return Ok(());
        }

        let widths = self.column_widths();

        // Header row
        for (i, header) in self.headers.iter().enumerate() {
            if i > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{:<width$}", header, width = widths[i])?;
        }
        writeln!(f)?;

        // Separator
        for (i, w) in widths.iter().enumerate() {
            if i > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{}", "-".repeat(*w))?;
        }
        writeln!(f)?;

        // Data rows
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    write!(f, "  ")?;
                }
                write!(f, "{:<width$}", cell, width = widths[i])?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during CLI command execution.
#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
pub enum CliError {
    /// An argument was missing or malformed.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// The requested entity was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Command execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// A scope string could not be parsed.
    #[error("Invalid scope: {0}")]
    InvalidScope(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_output_display() {
        let out = CliOutput::Text("hello world".into());
        assert_eq!(format!("{out}"), "hello world");
    }

    #[test]
    fn test_json_output_display() {
        let out = CliOutput::Json(r#"{"key":"value"}"#.into());
        assert_eq!(format!("{out}"), r#"{"key":"value"}"#);
    }

    #[test]
    fn test_table_formatting() {
        let mut table = CliTable::new(vec!["Name".into(), "Value".into()]);
        table
            .add_row(vec!["alpha".into(), "1".into()])
            .expect("test: add row");
        table
            .add_row(vec!["beta".into(), "2".into()])
            .expect("test: add row");

        let rendered = format!("{table}");
        assert!(rendered.contains("Name"));
        assert!(rendered.contains("Value"));
        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("beta"));
        assert!(rendered.contains("----"));
    }

    #[test]
    fn test_table_column_widths() {
        let mut table = CliTable::new(vec!["X".into(), "LongHeader".into()]);
        table
            .add_row(vec!["short".into(), "a".into()])
            .expect("test: add row");

        let widths = table.column_widths();
        // "short" (5) > "X" (1), so first column = 5
        assert_eq!(widths[0], 5);
        // "LongHeader" (10) > "a" (1), so second column = 10
        assert_eq!(widths[1], 10);
    }

    #[test]
    fn test_table_row_count() {
        let mut table = CliTable::new(vec!["A".into()]);
        assert_eq!(table.row_count(), 0);
        table.add_row(vec!["x".into()]).expect("test: add row");
        assert_eq!(table.row_count(), 1);
    }

    #[test]
    fn test_table_mismatched_row() {
        let mut table = CliTable::new(vec!["A".into(), "B".into()]);
        let err = table.add_row(vec!["only-one".into()]).unwrap_err();
        assert!(matches!(err, CliError::InvalidArgument(_)));
    }

    #[test]
    fn test_empty_table_display() {
        let table = CliTable::new(vec![]);
        let rendered = format!("{table}");
        assert!(rendered.is_empty());
    }

    #[test]
    fn test_cli_error_display() {
        let e = CliError::NotFound("node-42".into());
        assert_eq!(format!("{e}"), "Not found: node-42");

        let e2 = CliError::InvalidScope("foobar".into());
        assert_eq!(format!("{e2}"), "Invalid scope: foobar");
    }

    #[test]
    fn test_table_output_display() {
        let mut table = CliTable::new(vec!["Col".into()]);
        table.add_row(vec!["val".into()]).expect("test: add row");

        let out = CliOutput::Table(table);
        let rendered = format!("{out}");
        assert!(rendered.contains("Col"));
        assert!(rendered.contains("val"));
    }
}

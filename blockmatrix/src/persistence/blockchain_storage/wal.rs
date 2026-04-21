// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Write-ahead log entries, reader, and writer.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::blockchain::block::Block;

use super::super::{PersistenceError, PersistenceResult};

/// Write-ahead log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Operation type
    pub op_type: WalOperation,
    /// Block data
    pub block: Block,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WAL operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalOperation {
    /// Add new block
    AddBlock,
    /// Update block (shouldn't happen in blockchain but included for completeness)
    UpdateBlock,
}

/// Write-ahead log writer
pub(super) struct WalWriter {
    file: BufWriter<File>,
}

impl WalWriter {
    pub(super) fn new(path: PathBuf) -> PersistenceResult<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    pub(super) fn write_entry(&mut self, entry: WalEntry) -> PersistenceResult<()> {
        let serialized = bincode::serialize(&entry)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Write length prefix
        let len = serialized.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&serialized)?;
        self.file.flush()?;

        Ok(())
    }

    pub(super) fn flush(&mut self) -> PersistenceResult<()> {
        self.file.flush()?;
        Ok(())
    }
}

/// Write-ahead log reader
pub(super) struct WalReader;

impl WalReader {
    pub(super) fn read_all(path: &Path) -> PersistenceResult<Vec<WalEntry>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            if reader.read_exact(&mut len_bytes).is_err() {
                break; // End of file
            }

            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut buffer = vec![0u8; len];
            reader.read_exact(&mut buffer)?;

            let entry: WalEntry = bincode::deserialize(&buffer)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            entries.push(entry);
        }

        Ok(entries)
    }
}

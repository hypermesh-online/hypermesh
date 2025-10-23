//! Deduplication engine

use anyhow::Result;
use crate::chunking::{Chunk, ChunkId};
use std::collections::HashSet;

pub struct DedupEngine {
    known_chunks: HashSet<ChunkId>,
}

impl DedupEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            known_chunks: HashSet::new(),
        })
    }
    
    pub fn check_duplicates(&mut self, chunks: &[Chunk]) -> usize {
        let mut duplicates = 0;
        for chunk in chunks {
            if self.known_chunks.contains(&chunk.id) {
                duplicates += 1;
            } else {
                self.known_chunks.insert(chunk.id.clone());
            }
        }
        duplicates
    }
    
    pub fn find_duplicates(&self, chunks: &[Chunk]) -> Vec<ChunkId> {
        chunks.iter()
            .filter(|chunk| self.known_chunks.contains(&chunk.id))
            .map(|chunk| chunk.id.clone())
            .collect()
    }
}
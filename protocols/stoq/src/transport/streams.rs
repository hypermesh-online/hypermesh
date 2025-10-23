//! Stream management for STOQ transport

use anyhow::Result;

pub struct StreamManager {
    max_streams: u32,
}

impl StreamManager {
    pub fn new(max_streams: u32) -> Self {
        Self { max_streams }
    }
}
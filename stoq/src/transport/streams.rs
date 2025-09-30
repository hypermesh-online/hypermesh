//! Stream management for STOQ transport

// Stream management utilities

pub struct StreamManager {
    #[allow(dead_code)]
    max_streams: u32,
}

impl StreamManager {
    pub fn new(max_streams: u32) -> Self {
        Self { max_streams }
    }
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Stream management for STOQ transport

// Stream management utilities

pub struct StreamManager {
    _max_streams: u32,
}

impl StreamManager {
    pub fn new(max_streams: u32) -> Self {
        Self {
            _max_streams: max_streams,
        }
    }
}

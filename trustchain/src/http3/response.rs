// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorDetails>,
    pub request_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetails,
    pub request_id: String,
    pub timestamp: i64,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(code: String, message: String, request_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorDetails {
                code,
                message,
                details: None,
            }),
            request_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl ErrorResponse {
    pub fn new(code: String, message: String, request_id: String) -> Self {
        Self {
            success: false,
            error: ErrorDetails {
                code,
                message,
                details: None,
            },
            request_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.error.details = Some(details);
        self
    }

    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}
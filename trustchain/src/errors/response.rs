// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Error response types for API endpoints

use super::{domain::*, TrustChainError};
use serde::{Deserialize, Serialize};

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: &TrustChainError) -> Self {
        Self {
            error: error.to_string(),
            code: Self::error_code(error),
            details: Self::error_details(error),
            timestamp: chrono::Utc::now(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    fn error_code(error: &TrustChainError) -> String {
        match error {
            TrustChainError::CertificateAuthority(e) => match e {
                CAError::CertificateNotFound { .. } => "CA_CERT_NOT_FOUND".to_string(),
                CAError::CertificateRevoked { .. } => "CA_CERT_REVOKED".to_string(),
                CAError::CertificateExpired { .. } => "CA_CERT_EXPIRED".to_string(),
                _ => "CA_ERROR".to_string(),
            },
            TrustChainError::CertificateTransparency(e) => match e {
                CTError::LogNotFound { .. } => "CT_LOG_NOT_FOUND".to_string(),
                CTError::EntryNotFound { .. } => "CT_ENTRY_NOT_FOUND".to_string(),
                _ => "CT_ERROR".to_string(),
            },
            TrustChainError::DnsResolver(e) => match e {
                DnsError::RecordNotFound { .. } => "DNS_RECORD_NOT_FOUND".to_string(),
                DnsError::IPv6OnlyViolation => "DNS_IPV6_ONLY_VIOLATION".to_string(),
                _ => "DNS_ERROR".to_string(),
            },
            TrustChainError::ApiServer(e) => match e {
                ApiError::Authentication { .. } => "API_AUTH_FAILED".to_string(),
                ApiError::Authorization { .. } => "API_AUTHZ_FAILED".to_string(),
                ApiError::RateLimitExceeded { .. } => "API_RATE_LIMIT".to_string(),
                _ => "API_ERROR".to_string(),
            },
            TrustChainError::StateProofValidation(e) => match e {
                StateProofError::ByzantineFault { .. } => "STATE_PROOF_BYZANTINE_FAULT".to_string(),
                _ => "STATE_PROOF_ERROR".to_string(),
            },
            TrustChainError::SecurityError { .. } => "SECURITY_ERROR".to_string(),
            TrustChainError::SecurityValidationFailed { .. } => {
                "SECURITY_VALIDATION_FAILED".to_string()
            }
            TrustChainError::ByzantineFaultDetected { .. } => {
                "BYZANTINE_FAULT_DETECTED".to_string()
            }
            TrustChainError::Configuration(_) => "CONFIG_ERROR".to_string(),
            TrustChainError::Network(_) => "NETWORK_ERROR".to_string(),
            TrustChainError::Storage(_) => "STORAGE_ERROR".to_string(),
            TrustChainError::Cryptographic(_) => "CRYPTO_ERROR".to_string(),
            TrustChainError::Timeout { .. } => "TIMEOUT_ERROR".to_string(),
            TrustChainError::Internal { .. } => "INTERNAL_ERROR".to_string(),
            _ => "UNKNOWN_ERROR".to_string(),
        }
    }

    fn error_details(error: &TrustChainError) -> Option<serde_json::Value> {
        match error {
            TrustChainError::Timeout {
                operation,
                duration,
            } => Some(serde_json::json!({
                "operation": operation,
                "timeout_duration_secs": duration.as_secs()
            })),
            TrustChainError::StateProofValidation(StateProofError::ProofOfStakeFailed {
                stake,
                minimum,
            }) => Some(serde_json::json!({
                "current_stake": stake,
                "minimum_required": minimum
            })),
            TrustChainError::ApiServer(ApiError::RateLimitExceeded { limit }) => {
                Some(serde_json::json!({
                    "rate_limit": limit,
                    "unit": "requests_per_minute"
                }))
            }
            TrustChainError::SecurityValidationFailed { reason } => Some(serde_json::json!({
                "security_failure_reason": reason
            })),
            TrustChainError::ByzantineFaultDetected { node_id, reason } => {
                Some(serde_json::json!({
                    "byzantine_node_id": node_id,
                    "byzantine_reason": reason
                }))
            }
            _ => None,
        }
    }
}

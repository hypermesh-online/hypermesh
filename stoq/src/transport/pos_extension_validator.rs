// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! PoS Extension Validator for eBPF Pipeline
//!
//! Bridges the [`PosFastValidator`] into the eBPF extension validation pipeline.
//! Implements [`ExtensionValidator`] from `hypermesh-ebpf` to handle STOQ_TOKEN
//! extension frames at the transport layer.

use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, warn};

use super::ebpf::ExtensionValidator;
use crate::protocol::pos_fast_validator::{FastValidationResult, PosFastValidator};
use crate::protocol::pos_validator::PosToken;

/// STOQ_TOKEN extension type used in eBPF extension headers.
///
/// This is the lower 16 bits of the STOQ_TOKEN frame type (0xfe000001).
/// The eBPF layer uses u16 extension type identifiers.
const STOQ_POS_EXTENSION_TYPE: u16 = 0x0001;

/// PoS extension validator for the eBPF transport pipeline.
///
/// Deserializes extension data as a [`PosToken`] and runs the fast
/// structural pre-validation. Full crypto validation is deferred to the
/// application layer based on privacy tier.
pub struct StoqPosExtensionValidator {
    fast_validator: Arc<PosFastValidator>,
}

impl StoqPosExtensionValidator {
    /// Create a new extension validator backed by the given fast validator.
    pub fn new(fast_validator: Arc<PosFastValidator>) -> Self {
        Self { fast_validator }
    }

    /// Get a reference to the underlying fast validator.
    pub fn fast_validator(&self) -> &Arc<PosFastValidator> {
        &self.fast_validator
    }
}

#[async_trait::async_trait]
impl ExtensionValidator for StoqPosExtensionValidator {
    /// Validate a STOQ PoS extension header.
    ///
    /// Deserializes the extension data as a bincode-encoded [`PosToken`]
    /// and runs the fast structural pre-check. Tokens that pass structural
    /// checks are allowed through; full crypto is deferred to the protocol
    /// layer based on privacy tier.
    async fn validate(&self, extension_type: u16, extension_data: &[u8]) -> Result<()> {
        if extension_type != STOQ_POS_EXTENSION_TYPE {
            // Not our extension type; skip silently
            return Ok(());
        }

        // Deserialize the token from bincode
        let token: PosToken = match bincode::deserialize(extension_data) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to deserialize PoS token extension: {}", e);
                anyhow::bail!("Invalid PoS token extension data: {e}");
            }
        };

        // Run fast structural validation
        match self.fast_validator.fast_validate(&token) {
            FastValidationResult::CachedValid => {
                debug!("eBPF extension: PoS token cached valid");
                Ok(())
            }
            FastValidationResult::PassToFull => {
                debug!("eBPF extension: PoS token passed structural checks");
                Ok(())
            }
            FastValidationResult::Rejected(reason) => {
                warn!("eBPF extension: PoS token rejected: {}", reason);
                anyhow::bail!("PoS token rejected at line rate: {reason}");
            }
        }
    }

    /// Extension types handled by this validator.
    fn supported_extensions(&self) -> Vec<u16> {
        vec![STOQ_POS_EXTENSION_TYPE]
    }

    /// Validator name for logging.
    fn name(&self) -> &str {
        "stoq-pos-validator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::pos_fast_validator::FastValidationConfig;
    use crate::protocol::pos_validator::{
        PosTokenValidator, ProofOfSpace, ProofOfStake, ProofOfTime, ProofOfWork,
    };
    use std::time::{Duration, SystemTime};

    fn make_test_token() -> PosToken {
        PosToken {
            id: vec![1, 2, 3, 4],
            proof_of_space: ProofOfSpace {
                commitment_hash: vec![5, 6, 7, 8],
                matrix_position: (1, 2, 3),
                capacity: 1024 * 1024,
            },
            proof_of_stake: ProofOfStake {
                owner_pubkey: vec![9, 10, 11, 12],
                stake_amount: 1000,
                staked_until: SystemTime::now() + Duration::from_secs(3600),
            },
            proof_of_work: ProofOfWork {
                difficulty: 10,
                nonce: 12345,
                work_hash: vec![0, 0, 0x0F, 0xFF],
            },
            proof_of_time: ProofOfTime {
                timestamp: SystemTime::now(),
                sequence: 1,
                prev_hash: vec![17, 18, 19, 20],
            },
            signature: vec![21, 22, 23, 24],
            expires_at: SystemTime::now() + Duration::from_secs(300),
            issuer_pubkey: Some(vec![25, 26, 27, 28]),
        }
    }

    fn make_ext_validator() -> StoqPosExtensionValidator {
        let full = Arc::new(PosTokenValidator::new(Duration::from_secs(300)));
        let fast = Arc::new(PosFastValidator::new(FastValidationConfig::default(), full));
        StoqPosExtensionValidator::new(fast)
    }

    #[tokio::test]
    async fn test_extension_validator_valid_token() {
        let v = make_ext_validator();
        let token = make_test_token();
        let data = bincode::serialize(&token).expect("test: serialize token");

        let result = v.validate(STOQ_POS_EXTENSION_TYPE, &data).await;
        assert!(
            result.is_ok(),
            "Valid token should pass extension validation"
        );
    }

    #[tokio::test]
    async fn test_extension_validator_invalid_data() {
        let v = make_ext_validator();
        let result = v.validate(STOQ_POS_EXTENSION_TYPE, b"not a token").await;
        assert!(result.is_err(), "Invalid data should fail deserialization");
    }

    #[tokio::test]
    async fn test_extension_validator_wrong_type() {
        let v = make_ext_validator();
        // Wrong extension type should pass through silently
        let result = v.validate(0xFFFF, b"anything").await;
        assert!(result.is_ok(), "Wrong extension type should be ignored");
    }

    #[tokio::test]
    async fn test_extension_validator_name() {
        let v = make_ext_validator();
        assert_eq!(v.name(), "stoq-pos-validator");
    }

    #[tokio::test]
    async fn test_extension_validator_supported() {
        let v = make_ext_validator();
        let supported = v.supported_extensions();
        assert_eq!(supported, vec![STOQ_POS_EXTENSION_TYPE]);
    }

    #[tokio::test]
    async fn test_extension_validator_rejected_token() {
        let v = make_ext_validator();
        let mut token = make_test_token();
        token.proof_of_work.difficulty = 1; // Below minimum
        let data = bincode::serialize(&token).expect("test: serialize token");

        let result = v.validate(STOQ_POS_EXTENSION_TYPE, &data).await;
        assert!(result.is_err(), "Low-difficulty token should be rejected");
    }
}

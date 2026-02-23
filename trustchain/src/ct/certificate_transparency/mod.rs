// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Transparency Log Implementation
//!
//! High-performance certificate transparency logging with Merkle tree validation,
//! Byzantine fault tolerance, and <1s per certificate logging performance.

pub mod types;
pub mod operations;

// Re-export all public types for backward compatibility
pub use types::*;
pub use operations::CertificateTransparencyLog;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::{IssuedCertificate, CertificateMetadata, CertificateStatus};
    use crate::consensus::ConsensusProof;
    use std::time::{SystemTime, Duration};

    #[tokio::test]
    async fn test_ct_log_creation() {
        let ct_log = CertificateTransparencyLog::new().await.expect("test");
        assert_eq!(ct_log.get_metrics().await.current_tree_size.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_certificate_addition() {
        let ct_log = CertificateTransparencyLog::new().await.expect("test");

        let certificate = IssuedCertificate {
            certificate_der: vec![0x30, 0x82, 0x01, 0x00],
            certificate_pem: String::new(),
            chain_pem: String::new(),
            serial_number: "test-cert-001".to_string(),
            fingerprint: [0u8; 32],
            common_name: "test.example.com".to_string(),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(86400 * 365),
            issuer_ca_id: "test-ca".to_string(),
            consensus_proof: match ConsensusProof::generate_from_network("test-node").await {
                Ok(p) => p,
                Err(_) => {
                    use crate::consensus::proof::{StakeProof, TimeProof, SpaceProof, WorkProof};
                    ConsensusProof::new(
                        StakeProof::default(),
                        TimeProof::default(),
                        SpaceProof::default(),
                        WorkProof::default(),
                    )
                }
            },
            status: CertificateStatus::Valid,
            metadata: CertificateMetadata::default(),
        };

        let entry = ct_log.add_certificate(&certificate).await.expect("test");
        assert_eq!(entry.issuer_ca_id, "test-ca");
        assert_eq!(ct_log.get_metrics().await.current_tree_size.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_cryptographic_signing() {
        let ct_log = CertificateTransparencyLog::new().await.expect("test");

        let test_data = b"test signing data";
        let signature = ct_log._sign_data(test_data).await.expect("test");

        assert!(!signature.iter().all(|&b| b == 0));
        assert_eq!(signature.len(), 64);
    }

    #[tokio::test]
    async fn test_tree_head_signing() {
        let ct_log = CertificateTransparencyLog::new().await.expect("test");

        let tree_size = 100;
        let signature = ct_log._sign_tree_head(tree_size).await.expect("test");

        assert!(!signature.iter().all(|&b| b == 0));
        assert_eq!(signature.len(), 64);
    }
}

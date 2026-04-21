// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Kyber/FALCON encryption and key management
//!
//! Handles quantum-resistant encryption operations

/// Create Kyber encryption key for quantum-resistant security
pub async fn create_kyber_encryption_key() -> anyhow::Result<String> {
    let kyber = trustchain::crypto::KyberCrypto::new()?;
    let kp = kyber.generate_keypair().await?;
    Ok(hex::encode(&kp.public_key.key_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_encryption_key() {
        let key_id = create_kyber_encryption_key()
            .await
            .expect("test: kyber keygen");
        assert!(!key_id.is_empty());
        assert!(key_id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

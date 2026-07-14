// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset transfer protocol for moving ownership between matrix positions.
//!
//! Every transfer requires full Proof of State authentication (all 4 proofs:
//! PoSpace, PoStake, PoWork, PoTime) at both source and target nodes.
//! The transfer produces blockchain records on both chains and a
//! cryptographic receipt binding the entire operation.
//!
//! Proofs are opaque byte vectors validated by [`StateAuthenticator`]. This
//! module never inspects proof internals — it only cares whether the
//! authenticator says "authentic" or "not authentic".

use std::sync::Arc;
use std::time::SystemTime;

use crate::assets::core::asset_id::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use crate::blockchain::node_chain::NodeBlockchain;
use crate::proof_of_state::validation::StateAuthenticator;
use crate::proof_of_state::StateProof;
use crate::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::{AddressError, AssetAddress, ContentHash};

/// Opaque Proof of State bytes.
///
/// The transfer layer treats PoS proofs as opaque — it passes them to
/// [`StateAuthenticator::validate`] and acts on the boolean result.
/// The internal structure (Space/Stake/Work/Time) is the authenticator's
/// concern, not ours.
#[derive(Debug, Clone)]
pub struct StateProofBytes(pub Vec<u8>);

impl StateProofBytes {
    /// Wrap raw proof bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Whether the proof is empty (will always fail authentication).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Intent to transfer asset ownership between matrix positions.
#[derive(Debug, Clone)]
pub struct TransferIntent {
    /// BLAKE3 hash of (asset_addr + source + target + timestamp)
    pub transfer_id: ContentHash,
    /// IPv6 address of the asset being transferred
    pub asset_address: AssetAddress,
    /// Source node's matrix position
    pub source_coord: MatrixCoordinate,
    /// Target node's matrix position
    pub target_coord: MatrixCoordinate,
    /// All 4 PoS proofs from source node (opaque bytes)
    pub source_proof: StateProofBytes,
    /// Per-shard transfer metadata
    pub shard_map: Vec<ShardTransferEntry>,
    /// When this intent was created
    pub created_at: SystemTime,
}

/// Per-shard transfer info
#[derive(Debug, Clone)]
pub struct ShardTransferEntry {
    /// Derived shard sub-address
    pub shard_address: AssetAddress,
    /// BLAKE3 hash of shard data
    pub shard_hash: ContentHash,
    /// Matrix positions where this shard currently lives
    pub source_positions: Vec<MatrixCoordinate>,
}

/// Completed transfer receipt with cryptographic proof of the operation.
#[derive(Debug, Clone)]
pub struct TransferReceipt {
    /// Matches the intent's transfer_id
    pub transfer_id: ContentHash,
    /// Asset address at source (old position)
    pub old_address: AssetAddress,
    /// Asset address at target (new position)
    pub new_address: AssetAddress,
    /// Block index recording "transfer-out" on source chain
    pub source_block_index: u64,
    /// Block index recording "transfer-in" on target chain
    pub target_block_index: u64,
    /// All 4 PoS proofs from target node (opaque bytes)
    pub target_proof: StateProofBytes,
    /// BLAKE3 of entire receipt (tamper-evident binding)
    pub receipt_hash: ContentHash,
    /// When the transfer completed
    pub completed_at: SystemTime,
}

/// Result of validating a transfer intent
#[derive(Debug, Clone)]
pub enum TransferValidation {
    /// Transfer is valid and can proceed
    Valid,
    /// Source PoS proof failed authentication
    InvalidSourceProof(String),
    /// Target PoS proof failed authentication
    InvalidTargetProof(String),
    /// Asset not found at claimed address
    AssetNotFound,
    /// Shard map doesn't match asset
    ShardMapMismatch(String),
    /// Address doesn't match claimed coordinates
    AddressMismatch(String),
}

/// Transfer errors
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("PoS authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Address error: {0}")]
    Address(#[from] AddressError),
    #[error("Blockchain error: {0}")]
    Blockchain(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(TransferValidation),
}

impl std::fmt::Display for TransferValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferValidation::Valid => write!(f, "Valid"),
            TransferValidation::InvalidSourceProof(s) => {
                write!(f, "InvalidSourceProof: {s}")
            }
            TransferValidation::InvalidTargetProof(s) => {
                write!(f, "InvalidTargetProof: {s}")
            }
            TransferValidation::AssetNotFound => write!(f, "AssetNotFound"),
            TransferValidation::ShardMapMismatch(s) => {
                write!(f, "ShardMapMismatch: {s}")
            }
            TransferValidation::AddressMismatch(s) => {
                write!(f, "AddressMismatch: {s}")
            }
        }
    }
}

/// Orchestrates asset transfers with PoS authentication at both endpoints.
pub struct TransferEngine {
    validator: Arc<dyn StateAuthenticator>,
}

impl TransferEngine {
    /// Create a new transfer engine with the given PoS validator.
    pub fn new(validator: Arc<dyn StateAuthenticator>) -> Self {
        Self { validator }
    }

    /// Execute a full asset transfer between two nodes.
    ///
    /// Flow:
    /// 1. Authenticate source PoS proof (all 4: Space/Stake/Work/Time)
    /// 2. Authenticate target PoS proof
    /// 3. Verify asset address matches source coordinates
    /// 4. Compute new AssetAddress at target coordinates
    /// 5. Record "transfer-out" block on source chain
    /// 6. Record "transfer-in" block on target chain
    /// 7. Compute receipt_hash = BLAKE3(transfer_id + old + new + block indices)
    /// 8. Return TransferReceipt
    pub async fn execute_transfer(
        &self,
        intent: &TransferIntent,
        target_proof: &StateProofBytes,
        source_chain: &NodeBlockchain,
        target_chain: &NodeBlockchain,
    ) -> Result<TransferReceipt, TransferError> {
        // 1. Authenticate source PoS proof
        let source_valid = self
            .validator
            .validate(intent.source_proof.as_bytes())
            .await
            .map_err(|e| TransferError::AuthenticationFailed(format!("source: {e}")))?;
        if !source_valid {
            return Err(TransferError::ValidationFailed(
                TransferValidation::InvalidSourceProof("Source PoS authentication failed".into()),
            ));
        }

        // 2. Authenticate target PoS proof
        let target_valid = self
            .validator
            .validate(target_proof.as_bytes())
            .await
            .map_err(|e| TransferError::AuthenticationFailed(format!("target: {e}")))?;
        if !target_valid {
            return Err(TransferError::ValidationFailed(
                TransferValidation::InvalidTargetProof("Target PoS authentication failed".into()),
            ));
        }

        // 3. Verify asset address encodes the source coordinates
        self.verify_address_coords(&intent.asset_address, &intent.source_coord)?;

        // 4. Compute new AssetAddress at target coordinates (same content fingerprint)
        let old_address = intent.asset_address;
        let new_address = self.readdress_asset(&old_address, &intent.target_coord)?;

        // 5. Record "transfer-out" block on source chain
        let source_proof = StateProof::from_bytes(intent.source_proof.as_bytes())
            .map_err(|e| TransferError::AuthenticationFailed(format!("source proof decode: {e}")))?;
        let transfer_out_asset = build_transfer_record(
            "transfer-out",
            &old_address,
            &intent.transfer_id,
            &intent.target_coord,
        );
        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let source_entry = crate::blockchain::block::BlockAssetEntry::new_bound(
            transfer_out_asset.content_hash,
            &source_proof,
            crate::blockchain::block::StoragePointer::Genesis,
            transfer_out_asset,
        );
        let source_block = source_chain
            .add_block(vec![source_entry])
            .await
            .map_err(|e| TransferError::Blockchain(format!("source chain: {e}")))?;

        // 6. Record "transfer-in" block on target chain
        let target_proof_typed = StateProof::from_bytes(target_proof.as_bytes())
            .map_err(|e| TransferError::AuthenticationFailed(format!("target proof decode: {e}")))?;
        let transfer_in_asset = build_transfer_record(
            "transfer-in",
            &new_address,
            &intent.transfer_id,
            &intent.source_coord,
        );
        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let target_entry = crate::blockchain::block::BlockAssetEntry::new_bound(
            transfer_in_asset.content_hash,
            &target_proof_typed,
            crate::blockchain::block::StoragePointer::Genesis,
            transfer_in_asset,
        );
        let target_block = target_chain
            .add_block(vec![target_entry])
            .await
            .map_err(|e| TransferError::Blockchain(format!("target chain: {e}")))?;

        // 7. Compute receipt hash
        let receipt_hash = compute_receipt_hash(
            &intent.transfer_id,
            &old_address,
            &new_address,
            source_block.index,
            target_block.index,
        );

        // 8. Return receipt
        Ok(TransferReceipt {
            transfer_id: intent.transfer_id,
            old_address,
            new_address,
            source_block_index: source_block.index,
            target_block_index: target_block.index,
            target_proof: target_proof.clone(),
            receipt_hash,
            completed_at: SystemTime::now(),
        })
    }

    /// Validate a transfer intent without executing it.
    pub async fn validate_transfer(
        &self,
        intent: &TransferIntent,
        target_proof: &StateProofBytes,
    ) -> TransferValidation {
        // Validate source proof
        match self
            .validator
            .validate(intent.source_proof.as_bytes())
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                return TransferValidation::InvalidSourceProof("PoS authentication failed".into())
            }
            Err(e) => return TransferValidation::InvalidSourceProof(e.to_string()),
        }

        // Validate target proof
        match self.validator.validate(target_proof.as_bytes()).await {
            Ok(true) => {}
            Ok(false) => {
                return TransferValidation::InvalidTargetProof("PoS authentication failed".into())
            }
            Err(e) => return TransferValidation::InvalidTargetProof(e.to_string()),
        }

        // Validate address-coordinate alignment
        let (ax, ay, az) = intent.asset_address.matrix_coords();
        let (sx, sy, sz) = (
            intent.source_coord.x,
            intent.source_coord.y,
            intent.source_coord.z,
        );
        if ax != sx || ay != sy || az != sz {
            return TransferValidation::AddressMismatch(format!(
                "({ax},{ay},{az}) != ({sx},{sy},{sz})",
            ));
        }

        TransferValidation::Valid
    }

    /// Verify that an AssetAddress encodes the expected matrix coordinates.
    fn verify_address_coords(
        &self,
        addr: &AssetAddress,
        coord: &MatrixCoordinate,
    ) -> Result<(), TransferError> {
        let (ax, ay, az) = addr.matrix_coords();
        if ax != coord.x || ay != coord.y || az != coord.z {
            return Err(TransferError::ValidationFailed(
                TransferValidation::AddressMismatch(format!(
                    "Asset address coords ({},{},{}) != source coords ({},{},{})",
                    ax, ay, az, coord.x, coord.y, coord.z,
                )),
            ));
        }
        Ok(())
    }

    /// Compute a new AssetAddress at target coordinates preserving the content
    /// fingerprint from the original address.
    fn readdress_asset(
        &self,
        old_addr: &AssetAddress,
        target: &MatrixCoordinate,
    ) -> Result<AssetAddress, TransferError> {
        let fingerprint = old_addr.asset_fingerprint();
        // Reconstruct a ContentHash from the fingerprint bytes embedded in the
        // address. Only the first 5 bytes + high nibble of byte 6 are stored;
        // the rest of the 32-byte hash is zeroed (address carries 44 bits).
        let mut reconstructed = [0u8; 32];
        reconstructed[..5].copy_from_slice(&fingerprint[..5]);
        reconstructed[5] = fingerprint[5] & 0xF0;
        let content_hash = ContentHash::from_bytes(reconstructed);

        AssetAddress::new(target.x, target.y, target.z, &content_hash).map_err(TransferError::from)
    }
}

/// Build an AssetRegistration record for a transfer event.
fn build_transfer_record(
    direction: &str,
    address: &AssetAddress,
    transfer_id: &ContentHash,
    peer_coord: &MatrixCoordinate,
) -> AssetRegistration {
    AssetRegistration::from_asset_data(
        &AssetData {
            config: direction.as_bytes().to_vec(),
            definition: address.as_bytes().to_vec(),
            metadata: format!(
                "Transfer {} {} ({},{},{})",
                transfer_id, direction, peer_coord.x, peer_coord.y, peer_coord.z,
            )
            .into_bytes(),
        },
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Blockchain),
    )
}

/// Compute a deterministic receipt hash binding the transfer endpoints.
pub fn compute_receipt_hash(
    transfer_id: &ContentHash,
    old_addr: &AssetAddress,
    new_addr: &AssetAddress,
    source_block_idx: u64,
    target_block_idx: u64,
) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(transfer_id.as_bytes());
    hasher.update(old_addr.as_bytes());
    hasher.update(new_addr.as_bytes());
    hasher.update(&source_block_idx.to_le_bytes());
    hasher.update(&target_block_idx.to_le_bytes());
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

/// Create a transfer intent with a deterministic transfer_id.
pub fn create_transfer_intent(
    asset_address: AssetAddress,
    source_coord: MatrixCoordinate,
    target_coord: MatrixCoordinate,
    source_proof: StateProofBytes,
    shard_map: Vec<ShardTransferEntry>,
) -> TransferIntent {
    let now = SystemTime::now();
    let transfer_id = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(asset_address.as_bytes());
        hasher.update(&source_coord.x.to_le_bytes());
        hasher.update(&source_coord.y.to_le_bytes());
        hasher.update(&source_coord.z.to_le_bytes());
        hasher.update(&target_coord.x.to_le_bytes());
        hasher.update(&target_coord.y.to_le_bytes());
        hasher.update(&target_coord.z.to_le_bytes());
        let duration = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        hasher.update(&duration.as_nanos().to_le_bytes());
        ContentHash::from_bytes(*hasher.finalize().as_bytes())
    };

    TransferIntent {
        transfer_id,
        asset_address,
        source_coord,
        target_coord,
        source_proof,
        shard_map,
        created_at: now,
    }
}

/// Helper: serialize a trustchain proof into opaque StateProofBytes.
/// This is the ONLY place that touches trustchain's proof type.
pub fn proof_to_bytes(
    proof: &trustchain::proof_of_state::StateProof,
) -> Result<StateProofBytes, String> {
    let bytes = proof.to_bytes().map_err(|e| e.to_string())?;
    Ok(StateProofBytes::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::validation::DefaultStateAuthenticator;
    use trustchain::proof_of_state::StateProof;

    /// Helper to get valid test proof bytes
    fn test_proof_bytes() -> StateProofBytes {
        let proof = StateProof::new_for_testing();
        proof_to_bytes(&proof).expect("test: serialization")
    }

    #[tokio::test]
    async fn test_transfer_engine_creation() {
        let validator = Arc::new(DefaultStateAuthenticator::for_testing());
        let _engine = TransferEngine::new(validator);
    }

    #[tokio::test]
    async fn test_create_transfer_intent() {
        let hash = ContentHash::from_bytes([0xAB; 32]);
        let addr = AssetAddress::new(10, 20, 30, &hash).expect("test: valid address");
        let source = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let target = MatrixCoordinate::new(40, 50, 60).expect("test: valid coord");

        let intent = create_transfer_intent(addr, source, target, test_proof_bytes(), vec![]);
        assert_eq!(intent.asset_address, addr);
        assert_eq!(intent.source_coord, source);
        assert_eq!(intent.target_coord, target);
    }

    #[tokio::test]
    async fn test_execute_transfer() {
        let validator = Arc::new(DefaultStateAuthenticator::for_testing());
        let engine = TransferEngine::new(validator);

        let source_coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let target_coord = MatrixCoordinate::new(40, 50, 60).expect("test: valid coord");

        let hash = ContentHash::from_bytes([0xAB; 32]);
        let addr = AssetAddress::new(source_coord.x, source_coord.y, source_coord.z, &hash)
            .expect("test: valid address");

        let intent =
            create_transfer_intent(addr, source_coord, target_coord, test_proof_bytes(), vec![]);

        let source_chain = NodeBlockchain::new(source_coord);
        let target_chain = NodeBlockchain::new(target_coord);

        let receipt = engine
            .execute_transfer(&intent, &test_proof_bytes(), &source_chain, &target_chain)
            .await
            .expect("test: transfer should succeed");

        // Verify receipt addresses
        assert_eq!(receipt.old_address, addr);
        let (nx, ny, nz) = receipt.new_address.matrix_coords();
        assert_eq!((nx, ny, nz), (40, 50, 60));

        // Block indices should be 1 (first block after genesis)
        assert_eq!(receipt.source_block_index, 1);
        assert_eq!(receipt.target_block_index, 1);

        // Verify receipt hash is deterministic
        let expected = compute_receipt_hash(&intent.transfer_id, &addr, &receipt.new_address, 1, 1);
        assert_eq!(receipt.receipt_hash, expected);
    }

    #[tokio::test]
    async fn test_validate_transfer_passes_for_valid_intent() {
        let validator = Arc::new(DefaultStateAuthenticator::for_testing());
        let engine = TransferEngine::new(validator);

        let source_coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let target_coord = MatrixCoordinate::new(40, 50, 60).expect("test: valid coord");
        let hash = ContentHash::from_bytes([0xAB; 32]);
        let addr = AssetAddress::new(source_coord.x, source_coord.y, source_coord.z, &hash)
            .expect("test: valid address");

        let intent =
            create_transfer_intent(addr, source_coord, target_coord, test_proof_bytes(), vec![]);

        let result = engine.validate_transfer(&intent, &test_proof_bytes()).await;
        assert!(matches!(result, TransferValidation::Valid));
    }

    #[tokio::test]
    async fn test_empty_proof_rejected() {
        let validator = Arc::new(DefaultStateAuthenticator::for_testing());
        let engine = TransferEngine::new(validator);

        let source_coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let target_coord = MatrixCoordinate::new(40, 50, 60).expect("test: valid coord");
        let hash = ContentHash::from_bytes([0xAB; 32]);
        let addr = AssetAddress::new(source_coord.x, source_coord.y, source_coord.z, &hash)
            .expect("test: valid address");

        // Empty proof bytes — will always fail authentication
        let bad_proof = StateProofBytes::new(vec![]);

        let intent = create_transfer_intent(addr, source_coord, target_coord, bad_proof, vec![]);

        let source_chain = NodeBlockchain::new(source_coord);
        let target_chain = NodeBlockchain::new(target_coord);

        let result = engine
            .execute_transfer(&intent, &test_proof_bytes(), &source_chain, &target_chain)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_address_mismatch_rejected() {
        let validator = Arc::new(DefaultStateAuthenticator::for_testing());
        let engine = TransferEngine::new(validator);

        let source_coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let target_coord = MatrixCoordinate::new(40, 50, 60).expect("test: valid coord");
        let hash = ContentHash::from_bytes([0xAB; 32]);

        // Build address with DIFFERENT coords than source_coord
        let wrong_addr = AssetAddress::new(99, 99, 99, &hash).expect("test: valid address");

        let intent = create_transfer_intent(
            wrong_addr,
            source_coord,
            target_coord,
            test_proof_bytes(),
            vec![],
        );

        let source_chain = NodeBlockchain::new(source_coord);
        let target_chain = NodeBlockchain::new(target_coord);

        let result = engine
            .execute_transfer(&intent, &test_proof_bytes(), &source_chain, &target_chain)
            .await;

        assert!(
            matches!(
                result,
                Err(TransferError::ValidationFailed(
                    TransferValidation::AddressMismatch(_)
                ))
            ),
            "Expected AddressMismatch, got: {result:?}",
        );
    }

    #[tokio::test]
    async fn test_receipt_hash_deterministic() {
        let tid = ContentHash::from_bytes([0x01; 32]);
        let hash = ContentHash::from_bytes([0xAB; 32]);
        let old_addr = AssetAddress::new(1, 2, 3, &hash).expect("test: valid address");
        let new_addr = AssetAddress::new(4, 5, 6, &hash).expect("test: valid address");

        let h1 = compute_receipt_hash(&tid, &old_addr, &new_addr, 10, 20);
        let h2 = compute_receipt_hash(&tid, &old_addr, &new_addr, 10, 20);
        assert_eq!(h1, h2);

        // Different inputs produce different hash
        let h3 = compute_receipt_hash(&tid, &old_addr, &new_addr, 10, 21);
        assert_ne!(h1, h3);
    }
}

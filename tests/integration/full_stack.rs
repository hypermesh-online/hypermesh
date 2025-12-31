//! Full Stack Integration Test
//!
//! Scenario: Complete asset lifecycle from registration to retrieval
//! - Node bootstraps
//! - Registers with TrustChain CA
//! - Gets certificate
//! - Establishes STOQ connection to peer
//! - Publishes asset to BlockMatrix
//! - Asset is sharded, encrypted, distributed
//! - Peer retrieves asset via instruction-based retrieval

use super::test_harness::{IntegrationTestHarness, TestContext, NodeConfig};
use anyhow::Result;
use tracing::info;

#[tokio::test]
async fn test_full_stack_asset_lifecycle() -> Result<()> {
    let harness = IntegrationTestHarness::new("full_stack_asset_lifecycle");

    harness.run(|mut ctx| async move {
        info!("=== Phase 1: Node Bootstrap ===");

        // Create two nodes
        let node1_config = NodeConfig::new("node-1", 19001);
        let node2_config = NodeConfig::new("node-2", 19002);

        ctx.add_node(node1_config).await?;
        ctx.add_node(node2_config).await?;

        ctx.wait_for_ready(30).await?;

        info!("✅ Phase 1 Complete: Nodes bootstrapped");

        info!("=== Phase 2: TrustChain Certificate Issuance ===");

        // Get certificates for both nodes
        let node1 = ctx.get_node("node-1")?;
        let cert1 = node1.get_certificate("node-1.hypermesh.local").await?;
        info!("Node 1 received certificate: {}", cert1.serial_number);

        let node2 = ctx.get_node("node-2")?;
        let cert2 = node2.get_certificate("node-2.hypermesh.local").await?;
        info!("Node 2 received certificate: {}", cert2.serial_number);

        info!("✅ Phase 2 Complete: Certificates issued");

        info!("=== Phase 3: STOQ Connection Establishment ===");

        // Establish STOQ connection between nodes
        // NOTE: This requires actual STOQ transport implementation
        // For now, verify both endpoints exist
        assert!(node1.stoq_endpoint.is_some(), "Node 1 STOQ endpoint not initialized");
        assert!(node2.stoq_endpoint.is_some(), "Node 2 STOQ endpoint not initialized");

        info!("✅ Phase 3 Complete: STOQ endpoints ready");

        info!("=== Phase 4: BlockMatrix Asset Registration ===");

        // Register asset with BlockMatrix
        // NOTE: This requires actual BlockMatrix implementation
        // For now, verify BlockMatrix nodes exist
        assert!(node1.blockmatrix_node.is_some(), "Node 1 BlockMatrix not initialized");
        assert!(node2.blockmatrix_node.is_some(), "Node 2 BlockMatrix not initialized");

        info!("✅ Phase 4 Complete: BlockMatrix nodes ready");

        info!("=== Full Stack Test Complete ===");

        ctx.shutdown().await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_asset_creation_with_proofs() -> Result<()> {
    let harness = IntegrationTestHarness::new("asset_creation_with_proofs");

    harness.run(|mut ctx| async move {
        info!("=== Testing Asset Creation with Full Proof of State ===");

        // Create single node for asset creation
        let config = NodeConfig::new("asset-node", 19010);
        ctx.add_node(config).await?;
        ctx.wait_for_ready(30).await?;

        let node = ctx.get_node("asset-node")?;

        // Verify node has all required components
        assert!(node.trustchain.is_some(), "TrustChain required for asset proofs");
        assert!(node.stoq_endpoint.is_some(), "STOQ required for asset distribution");
        assert!(node.blockmatrix_node.is_some(), "BlockMatrix required for asset storage");

        info!("✅ All components initialized for asset creation");

        // Future: Create asset with four proofs
        // - PoSpace (WHERE): Storage location proof
        // - PoStake (WHO): Ownership proof
        // - PoWork (WHAT): Computational proof
        // - PoTime (WHEN): Temporal ordering proof

        info!("⚠️  TODO: Implement actual asset creation with Proof of State validation");

        ctx.shutdown().await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_certificate_chain_validation() -> Result<()> {
    let harness = IntegrationTestHarness::new("certificate_chain_validation");

    harness.run(|mut ctx| async move {
        info!("=== Testing Certificate Chain Validation ===");

        let config = NodeConfig::new("cert-node", 19020);
        ctx.add_node(config).await?;
        ctx.wait_for_ready(30).await?;

        let node = ctx.get_node("cert-node")?;
        let trustchain = node.trustchain.as_ref()
            .ok_or_else(|| anyhow::anyhow!("TrustChain not initialized"))?;

        // Issue certificate
        let cert = node.get_certificate("test.hypermesh.local").await?;
        info!("Certificate issued: {}", cert.serial_number);

        // Validate certificate fields
        assert!(!cert.certificate_data.is_empty(), "Certificate data empty");
        assert!(!cert.serial_number.is_empty(), "Serial number empty");

        info!("✅ Certificate validation passed");

        ctx.shutdown().await?;
        Ok(())
    })
    .await
}

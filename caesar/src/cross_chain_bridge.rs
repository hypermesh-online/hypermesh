// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-Chain Bridge for Caesar Token
//!
//! Implements the "mostly-stable" token mechanism with cross-chain support
//! for BTC, ETH, SOL, USDC and other major cryptocurrencies.
//!
//! Features:
//! - Dynamic fee adjustment for stability
//! - HyperMesh Asset System integration
//! - Cross-chain routing through LayerZero V2
//! - Self-stabilizing economic mechanisms

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Supported blockchain networks for cross-chain operations
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum NetworkType {
    Bitcoin,
    Ethereum,
    Solana,
    Polygon,
    Arbitrum,
    Optimism,
    BSC,
    Avalanche,
    HyperMesh, // Native network
}

impl NetworkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkType::Bitcoin => "bitcoin",
            NetworkType::Ethereum => "ethereum",
            NetworkType::Solana => "solana",
            NetworkType::Polygon => "polygon",
            NetworkType::Arbitrum => "arbitrum",
            NetworkType::Optimism => "optimism",
            NetworkType::BSC => "bsc",
            NetworkType::Avalanche => "avalanche",
            NetworkType::HyperMesh => "hypermesh",
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            NetworkType::Bitcoin => 0, // Bitcoin doesn't use EVM chain IDs
            NetworkType::Ethereum => 1,
            NetworkType::Solana => 900, // Custom identifier
            NetworkType::Polygon => 137,
            NetworkType::Arbitrum => 42161,
            NetworkType::Optimism => 10,
            NetworkType::BSC => 56,
            NetworkType::Avalanche => 43114,
            NetworkType::HyperMesh => 9999, // Custom HyperMesh chain ID
        }
    }
}

/// Cross-chain bridge operation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BridgeOperation {
    /// Lock tokens on source chain
    Lock {
        amount: Decimal,
        from_network: NetworkType,
        to_network: NetworkType,
        recipient: String,
    },
    /// Mint tokens on destination chain
    Mint {
        amount: Decimal,
        network: NetworkType,
        recipient: String,
        source_tx: String,
    },
    /// Burn tokens to unlock on source
    Burn {
        amount: Decimal,
        network: NetworkType,
        unlock_recipient: String,
    },
    /// Unlock tokens on source chain
    Unlock {
        amount: Decimal,
        network: NetworkType,
        recipient: String,
        burn_tx: String,
    },
}

/// Bridge transaction status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BridgeStatus {
    Initiated,
    Confirmed,
    Processing,
    Completed,
    Failed { reason: String },
    Reverted,
}

/// Cross-chain bridge transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeTransaction {
    pub id: String,
    pub operation: BridgeOperation,
    pub status: BridgeStatus,
    pub fee: Decimal,
    pub gas_fee: Decimal,
    pub stability_adjustment: Decimal,
    pub initiated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source_tx_hash: Option<String>,
    pub destination_tx_hash: Option<String>,
    pub confirmations: u32,
    pub required_confirmations: u32,
}

/// Network configuration for cross-chain operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network: NetworkType,
    pub rpc_url: String,
    pub contract_address: Option<String>,
    pub min_confirmations: u32,
    pub fee_rate: Decimal,
    pub gas_limit: u64,
    pub supported_tokens: Vec<String>,
    pub is_active: bool,
}

/// Cross-chain bridge manager
pub struct CrossChainBridge {
    /// Network configurations
    networks: Arc<RwLock<HashMap<NetworkType, NetworkConfig>>>,
    /// Active bridge transactions
    transactions: Arc<RwLock<HashMap<String, BridgeTransaction>>>,
    /// Bridge liquidity pools
    _liquidity_pools: Arc<RwLock<HashMap<NetworkType, Decimal>>>,
    /// Stability mechanism settings
    stability_config: StabilityConfig,
    /// Fee calculation engine
    fee_calculator: FeeCalculator,
}

/// Stability configuration for "mostly-stable" token
#[derive(Clone, Debug)]
pub struct StabilityConfig {
    /// Target stability range (e.g., ±2%)
    pub stability_threshold: Decimal,
    /// Maximum adjustment per operation
    pub max_adjustment: Decimal,
    /// Adjustment decay rate
    pub decay_rate: Decimal,
    /// Emergency circuit breaker threshold
    pub circuit_breaker_threshold: Decimal,
}

/// Dynamic fee calculator for cross-chain operations
#[derive(Clone, Debug)]
pub struct FeeCalculator {
    /// Base fee percentage
    pub base_fee: Decimal,
    /// Network-specific multipliers
    pub network_multipliers: HashMap<NetworkType, Decimal>,
    /// Congestion-based adjustments
    pub congestion_multiplier: Decimal,
    /// Stability adjustment factor
    pub stability_factor: Decimal,
}

impl CrossChainBridge {
    /// Create new cross-chain bridge
    pub async fn new() -> Result<Self> {
        let mut networks = HashMap::new();

        // Initialize default network configurations
        networks.insert(
            NetworkType::Ethereum,
            NetworkConfig {
                network: NetworkType::Ethereum,
                rpc_url: "https://eth-mainnet.alchemyapi.io/v2/".to_string(),
                contract_address: Some("0x0000000000000000000000000000000000000000".to_string()),
                min_confirmations: 12,
                fee_rate: dec!(0.003), // 0.3%
                gas_limit: 200000,
                supported_tokens: vec!["CAES".to_string(), "USDC".to_string(), "WETH".to_string()],
                is_active: true,
            },
        );

        networks.insert(
            NetworkType::Solana,
            NetworkConfig {
                network: NetworkType::Solana,
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                contract_address: Some("11111111111111111111111111111111".to_string()),
                min_confirmations: 32,
                fee_rate: dec!(0.002), // 0.2%
                gas_limit: 400000,
                supported_tokens: vec!["CAES".to_string(), "USDC".to_string(), "SOL".to_string()],
                is_active: true,
            },
        );

        networks.insert(
            NetworkType::Bitcoin,
            NetworkConfig {
                network: NetworkType::Bitcoin,
                rpc_url: "https://blockstream.info/api".to_string(),
                contract_address: None, // Bitcoin doesn't use contract addresses
                min_confirmations: 6,
                fee_rate: dec!(0.005), // 0.5% (higher due to Bitcoin complexity)
                gas_limit: 0,          // Bitcoin doesn't use gas
                supported_tokens: vec!["CAES".to_string()], // Wrapped via other protocols
                is_active: true,
            },
        );

        networks.insert(
            NetworkType::HyperMesh,
            NetworkConfig {
                network: NetworkType::HyperMesh,
                rpc_url: "http3://hypermesh".to_string(), // Native HyperMesh protocol
                contract_address: None,                   // Native asset system
                min_confirmations: 1,                     // Fast finality
                fee_rate: dec!(0.001),                    // 0.1% (lowest fees on native network)
                gas_limit: 100000,
                supported_tokens: vec!["CAES".to_string()],
                is_active: true,
            },
        );

        let stability_config = StabilityConfig {
            stability_threshold: dec!(0.02),       // 2%
            max_adjustment: dec!(0.01),            // 1% max adjustment
            decay_rate: dec!(0.95),                // 5% decay per period
            circuit_breaker_threshold: dec!(0.10), // 10% emergency threshold
        };

        let fee_calculator = FeeCalculator {
            base_fee: dec!(0.002), // 0.2% base fee
            network_multipliers: HashMap::new(),
            congestion_multiplier: dec!(1.0),
            stability_factor: dec!(1.0),
        };

        Ok(Self {
            networks: Arc::new(RwLock::new(networks)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            _liquidity_pools: Arc::new(RwLock::new(HashMap::new())),
            stability_config,
            fee_calculator,
        })
    }

    /// Initiate cross-chain transfer
    pub async fn initiate_bridge(&self, operation: BridgeOperation) -> Result<BridgeTransaction> {
        let tx_id = Uuid::new_v4().to_string();

        // Calculate fees based on operation
        let (base_fee, gas_fee, stability_adjustment) =
            self.calculate_bridge_fees(&operation).await?;

        // Validate operation
        self.validate_bridge_operation(&operation).await?;

        let transaction = BridgeTransaction {
            id: tx_id.clone(),
            operation: operation.clone(),
            status: BridgeStatus::Initiated,
            fee: base_fee,
            gas_fee,
            stability_adjustment,
            initiated_at: chrono::Utc::now(),
            completed_at: None,
            source_tx_hash: None,
            destination_tx_hash: None,
            confirmations: 0,
            required_confirmations: self.get_required_confirmations(&operation).await?,
        };

        // Store transaction
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx_id.clone(), transaction.clone());

        info!("Initiated cross-chain bridge transaction: {}", tx_id);

        // Start processing in background
        tokio::spawn(async move {
            // Processing logic would go here
            // For now, we'll just log the operation
            info!("Processing bridge operation: {:?}", operation);
        });

        Ok(transaction)
    }

    /// Calculate dynamic fees for bridge operation
    async fn calculate_bridge_fees(
        &self,
        operation: &BridgeOperation,
    ) -> Result<(Decimal, Decimal, Decimal)> {
        let networks = self.networks.read().await;

        let (from_network, to_network, amount) = match operation {
            BridgeOperation::Lock {
                amount,
                from_network,
                to_network,
                ..
            } => (Some(from_network), Some(to_network), *amount),
            BridgeOperation::Mint {
                amount, network, ..
            } => (None, Some(network), *amount),
            BridgeOperation::Burn {
                amount, network, ..
            } => (Some(network), None, *amount),
            BridgeOperation::Unlock {
                amount, network, ..
            } => (None, Some(network), *amount),
        };

        // Base fee calculation
        let mut base_fee = amount * self.fee_calculator.base_fee;

        // Network-specific adjustments
        if let Some(network) = from_network {
            if let Some(config) = networks.get(network) {
                base_fee += amount * config.fee_rate;
            }
        }

        if let Some(network) = to_network {
            if let Some(config) = networks.get(network) {
                base_fee += amount * config.fee_rate;
            }
        }

        // Gas fee estimation (simplified)
        let gas_fee = match to_network {
            Some(NetworkType::Ethereum) => dec!(0.01), // ~$10 USD equivalent
            Some(NetworkType::Solana) => dec!(0.001),  // ~$1 USD equivalent
            Some(NetworkType::Bitcoin) => dec!(0.005), // ~$5 USD equivalent
            Some(NetworkType::HyperMesh) => dec!(0.0001), // Minimal native fees
            _ => dec!(0.002),                          // Default
        };

        // Stability adjustment for "mostly-stable" mechanism
        let stability_adjustment = self.calculate_stability_adjustment(amount).await?;

        Ok((base_fee, gas_fee, stability_adjustment))
    }

    /// Calculate stability adjustment for "mostly-stable" token
    async fn calculate_stability_adjustment(&self, _amount: Decimal) -> Result<Decimal> {
        // Simplified stability calculation
        // In practice, this would analyze:
        // - Current token price vs target
        // - Market volatility
        // - Liquidity pool health
        // - Recent transaction volume

        let adjustment = dec!(0.0); // Placeholder

        Ok(adjustment
            .max(-self.stability_config.max_adjustment)
            .min(self.stability_config.max_adjustment))
    }

    /// Validate bridge operation
    async fn validate_bridge_operation(&self, operation: &BridgeOperation) -> Result<()> {
        let networks = self.networks.read().await;

        match operation {
            BridgeOperation::Lock {
                from_network,
                to_network,
                amount,
                ..
            } => {
                if !networks.contains_key(from_network) || !networks.contains_key(to_network) {
                    return Err(anyhow!("Unsupported network"));
                }
                if *amount <= Decimal::ZERO {
                    return Err(anyhow!("Invalid amount"));
                }
            }
            BridgeOperation::Mint {
                network, amount, ..
            } => {
                if !networks.contains_key(network) {
                    return Err(anyhow!("Unsupported network"));
                }
                if *amount <= Decimal::ZERO {
                    return Err(anyhow!("Invalid amount"));
                }
            }
            BridgeOperation::Burn {
                network, amount, ..
            } => {
                if !networks.contains_key(network) {
                    return Err(anyhow!("Unsupported network"));
                }
                if *amount <= Decimal::ZERO {
                    return Err(anyhow!("Invalid amount"));
                }
            }
            BridgeOperation::Unlock {
                network, amount, ..
            } => {
                if !networks.contains_key(network) {
                    return Err(anyhow!("Unsupported network"));
                }
                if *amount <= Decimal::ZERO {
                    return Err(anyhow!("Invalid amount"));
                }
            }
        }

        Ok(())
    }

    /// Get required confirmations for operation
    async fn get_required_confirmations(&self, operation: &BridgeOperation) -> Result<u32> {
        let networks = self.networks.read().await;

        match operation {
            BridgeOperation::Lock { from_network, .. } => Ok(networks
                .get(from_network)
                .map(|config| config.min_confirmations)
                .unwrap_or(12)),
            BridgeOperation::Mint { network, .. }
            | BridgeOperation::Burn { network, .. }
            | BridgeOperation::Unlock { network, .. } => Ok(networks
                .get(network)
                .map(|config| config.min_confirmations)
                .unwrap_or(12)),
        }
    }

    /// Get bridge transaction status
    pub async fn get_transaction(&self, tx_id: &str) -> Result<Option<BridgeTransaction>> {
        let transactions = self.transactions.read().await;
        Ok(transactions.get(tx_id).cloned())
    }

    /// List all bridge transactions for a user
    pub async fn list_transactions(&self, limit: usize) -> Result<Vec<BridgeTransaction>> {
        let transactions = self.transactions.read().await;
        let mut tx_list: Vec<BridgeTransaction> = transactions.values().cloned().collect();

        // Sort by initiated time, most recent first
        tx_list.sort_by(|a, b| b.initiated_at.cmp(&a.initiated_at));

        // Limit results
        tx_list.truncate(limit);

        Ok(tx_list)
    }

    /// Get supported networks
    pub async fn get_supported_networks(&self) -> Vec<NetworkType> {
        let networks = self.networks.read().await;
        networks
            .keys()
            .filter(|&k| {
                networks
                    .get(k)
                    .map(|config| config.is_active)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Update network configuration
    pub async fn update_network_config(&self, config: NetworkConfig) -> Result<()> {
        let mut networks = self.networks.write().await;
        networks.insert(config.network.clone(), config);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ChainBridge trait and implementations
// ---------------------------------------------------------------------------

/// Trait abstracting chain-level operations for cross-chain transfers.
/// External chain SDKs (ethers-rs, bitcoin, solana) would implement this;
/// for now we provide `InternalBridge` (Caesar's MeshCredit ledger) and
/// `MockChainBridge` for testing.
#[async_trait::async_trait]
pub trait ChainBridge: Send + Sync {
    /// Lock funds on the source chain, returning a lock proof (tx hash).
    async fn lock_funds(
        &self,
        amount: Decimal,
        recipient: &str,
    ) -> Result<String>;

    /// Verify that a lock transaction completed successfully.
    async fn verify_lock(&self, tx_hash: &str) -> Result<bool>;

    /// Release (unlock/mint) funds on the destination chain.
    async fn release_funds(
        &self,
        amount: Decimal,
        recipient: &str,
        source_tx: &str,
    ) -> Result<String>;

    /// Query the balance available on this chain.
    async fn query_balance(&self, address: &str) -> Result<Decimal>;

    /// The network type this bridge handles.
    fn network(&self) -> NetworkType;
}

/// Internal bridge that works with Caesar's MeshCredit in-memory ledger.
pub struct InternalBridge {
    /// In-memory balances keyed by address
    balances: Arc<RwLock<HashMap<String, Decimal>>>,
    /// Lock records keyed by tx hash
    lock_records: Arc<RwLock<HashMap<String, LockRecord>>>,
}

/// Record of a locked amount
#[derive(Debug, Clone)]
struct LockRecord {
    amount: Decimal,
    recipient: String,
    verified: bool,
}

impl InternalBridge {
    /// Create a new internal bridge with no initial balances.
    pub fn new() -> Self {
        Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            lock_records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Seed a balance for testing or initialization.
    pub async fn seed_balance(&self, address: &str, amount: Decimal) {
        let mut balances = self.balances.write().await;
        *balances.entry(address.to_string()).or_insert(dec!(0)) += amount;
    }
}

#[async_trait::async_trait]
impl ChainBridge for InternalBridge {
    async fn lock_funds(&self, amount: Decimal, recipient: &str) -> Result<String> {
        if amount <= Decimal::ZERO {
            return Err(anyhow!("Lock amount must be positive"));
        }

        // Deduct from recipient's available balance
        let mut balances = self.balances.write().await;
        let balance = balances.entry(recipient.to_string()).or_insert(dec!(0));
        if *balance < amount {
            return Err(anyhow!(
                "Insufficient balance: have {}, need {}",
                balance,
                amount
            ));
        }
        *balance -= amount;

        // Record the lock
        let tx_hash = Uuid::new_v4().to_string();
        let mut locks = self.lock_records.write().await;
        locks.insert(
            tx_hash.clone(),
            LockRecord {
                amount,
                recipient: recipient.to_string(),
                verified: true,
            },
        );

        Ok(tx_hash)
    }

    async fn verify_lock(&self, tx_hash: &str) -> Result<bool> {
        let locks = self.lock_records.read().await;
        Ok(locks.get(tx_hash).map(|r| r.verified).unwrap_or(false))
    }

    async fn release_funds(
        &self,
        amount: Decimal,
        recipient: &str,
        _source_tx: &str,
    ) -> Result<String> {
        if amount <= Decimal::ZERO {
            return Err(anyhow!("Release amount must be positive"));
        }

        let mut balances = self.balances.write().await;
        *balances.entry(recipient.to_string()).or_insert(dec!(0)) += amount;

        let tx_hash = Uuid::new_v4().to_string();
        Ok(tx_hash)
    }

    async fn query_balance(&self, address: &str) -> Result<Decimal> {
        let balances = self.balances.read().await;
        Ok(balances.get(address).cloned().unwrap_or(dec!(0)))
    }

    fn network(&self) -> NetworkType {
        NetworkType::HyperMesh
    }
}

/// Mock chain bridge for testing external chain integrations.
pub struct MockChainBridge {
    network_type: NetworkType,
    balances: Arc<RwLock<HashMap<String, Decimal>>>,
    locks: Arc<RwLock<HashMap<String, Decimal>>>,
}

impl MockChainBridge {
    /// Create a new mock bridge for a specific network.
    pub fn new(network_type: NetworkType) -> Self {
        Self {
            network_type,
            balances: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Seed balance for testing.
    pub async fn seed_balance(&self, address: &str, amount: Decimal) {
        let mut balances = self.balances.write().await;
        *balances.entry(address.to_string()).or_insert(dec!(0)) += amount;
    }
}

#[async_trait::async_trait]
impl ChainBridge for MockChainBridge {
    async fn lock_funds(&self, amount: Decimal, recipient: &str) -> Result<String> {
        let mut balances = self.balances.write().await;
        let balance = balances.entry(recipient.to_string()).or_insert(dec!(0));
        if *balance < amount {
            return Err(anyhow!("Mock: insufficient balance"));
        }
        *balance -= amount;

        let tx = format!("mock-tx-{}", Uuid::new_v4());
        let mut locks = self.locks.write().await;
        locks.insert(tx.clone(), amount);
        Ok(tx)
    }

    async fn verify_lock(&self, tx_hash: &str) -> Result<bool> {
        let locks = self.locks.read().await;
        Ok(locks.contains_key(tx_hash))
    }

    async fn release_funds(
        &self,
        amount: Decimal,
        recipient: &str,
        _source_tx: &str,
    ) -> Result<String> {
        let mut balances = self.balances.write().await;
        *balances.entry(recipient.to_string()).or_insert(dec!(0)) += amount;
        Ok(format!("mock-release-{}", Uuid::new_v4()))
    }

    async fn query_balance(&self, address: &str) -> Result<Decimal> {
        let balances = self.balances.read().await;
        Ok(balances.get(address).cloned().unwrap_or(dec!(0)))
    }

    fn network(&self) -> NetworkType {
        self.network_type.clone()
    }
}

// Request/Response models for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateBridgeRequest {
    pub operation: BridgeOperation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateBridgeResponse {
    pub transaction: BridgeTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeTransactionResponse {
    pub transaction: Option<BridgeTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeTransactionsResponse {
    pub transactions: Vec<BridgeTransaction>,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportedNetworksResponse {
    pub networks: Vec<NetworkType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_internal_bridge_lock_release_cycle() {
        let bridge = InternalBridge::new();

        // Seed balance
        bridge.seed_balance("alice", dec!(1000)).await;

        // Lock funds
        let tx = bridge
            .lock_funds(dec!(100), "alice")
            .await
            .expect("test: lock should succeed");

        // Verify lock
        let verified = bridge
            .verify_lock(&tx)
            .await
            .expect("test: verify should succeed");
        assert!(verified, "lock should be verified");

        // Check remaining balance
        let balance = bridge
            .query_balance("alice")
            .await
            .expect("test: query balance");
        assert_eq!(balance, dec!(900));

        // Release to bob
        let release_tx = bridge
            .release_funds(dec!(100), "bob", &tx)
            .await
            .expect("test: release should succeed");
        assert!(!release_tx.is_empty());

        let bob_balance = bridge
            .query_balance("bob")
            .await
            .expect("test: query bob balance");
        assert_eq!(bob_balance, dec!(100));
    }

    #[tokio::test]
    async fn test_internal_bridge_insufficient_funds() {
        let bridge = InternalBridge::new();
        bridge.seed_balance("carol", dec!(50)).await;

        let result = bridge.lock_funds(dec!(100), "carol").await;
        assert!(result.is_err(), "should fail with insufficient balance");
    }

    #[tokio::test]
    async fn test_mock_chain_bridge() {
        let mock = MockChainBridge::new(NetworkType::Ethereum);
        assert_eq!(mock.network().as_str(), "ethereum");

        mock.seed_balance("0xabc", dec!(500)).await;

        let tx = mock
            .lock_funds(dec!(200), "0xabc")
            .await
            .expect("test: mock lock");
        assert!(tx.starts_with("mock-tx-"));

        let verified = mock.verify_lock(&tx).await.expect("test: mock verify");
        assert!(verified);

        let balance = mock.query_balance("0xabc").await.expect("test: mock balance");
        assert_eq!(balance, dec!(300));
    }

    #[tokio::test]
    async fn test_cross_chain_bridge_initiate() {
        let bridge = CrossChainBridge::new()
            .await
            .expect("test: create bridge");

        let op = BridgeOperation::Lock {
            amount: dec!(100),
            from_network: NetworkType::Ethereum,
            to_network: NetworkType::HyperMesh,
            recipient: "test-recipient".to_string(),
        };

        let tx = bridge
            .initiate_bridge(op)
            .await
            .expect("test: initiate bridge");
        assert!(matches!(tx.status, BridgeStatus::Initiated));
        assert!(tx.fee > Decimal::ZERO);
    }
}

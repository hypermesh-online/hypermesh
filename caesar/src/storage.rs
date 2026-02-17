// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Storage Layer - BlockMatrix Asset-based operations
//!
//! Uses BlockMatrix asset storage for wallet and transaction management
//! instead of traditional databases.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tracing::{info, debug};
use uuid::Uuid;
use std::collections::{HashMap, BTreeMap};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::*;
use crate::DatabaseConfig;

/// Storage layer for Caesar economic system using BlockMatrix assets
pub struct CaesarStorage {
    /// Storage directory path
    storage_path: PathBuf,
    /// In-memory cache of wallets (keyed by wallet_id String)
    wallets: Arc<RwLock<HashMap<String, Wallet>>>,
    /// In-memory cache of transactions (keyed by transaction_id String)
    transactions: Arc<RwLock<BTreeMap<String, Transaction>>>,
    /// In-memory cache of rewards
    rewards: Arc<RwLock<Vec<RewardEntry>>>,
    /// In-memory cache of economic metrics
    metrics: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl CaesarStorage {
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        info!("Initializing Caesar storage layer with BlockMatrix assets");

        let storage_path = PathBuf::from(&config.url);

        // Ensure storage directory exists
        tokio::fs::create_dir_all(&storage_path).await?;

        // Initialize in-memory caches
        let wallets = Arc::new(RwLock::new(HashMap::new()));
        let transactions = Arc::new(RwLock::new(BTreeMap::new()));
        let rewards = Arc::new(RwLock::new(Vec::new()));
        let metrics = Arc::new(RwLock::new(HashMap::new()));

        // Load existing data from disk
        let storage = Self {
            storage_path: storage_path.clone(),
            wallets: wallets.clone(),
            transactions: transactions.clone(),
            rewards: rewards.clone(),
            metrics: metrics.clone(),
        };

        storage.load_from_disk().await?;

        Ok(storage)
    }

    /// Load data from disk storage
    async fn load_from_disk(&self) -> Result<()> {
        // Load wallets
        let wallets_file = self.storage_path.join("wallets.json");
        if wallets_file.exists() {
            let data = tokio::fs::read_to_string(&wallets_file).await?;
            if let Ok(loaded_wallets) = serde_json::from_str::<HashMap<String, Wallet>>(&data) {
                *self.wallets.write().await = loaded_wallets;
                debug!("Loaded {} wallets from disk", self.wallets.read().await.len());
            }
        }

        // Load transactions
        let transactions_file = self.storage_path.join("transactions.json");
        if transactions_file.exists() {
            let data = tokio::fs::read_to_string(&transactions_file).await?;
            if let Ok(loaded_transactions) = serde_json::from_str::<BTreeMap<String, Transaction>>(&data) {
                *self.transactions.write().await = loaded_transactions;
                debug!("Loaded {} transactions from disk", self.transactions.read().await.len());
            }
        }

        // Load rewards
        let rewards_file = self.storage_path.join("rewards.json");
        if rewards_file.exists() {
            let data = tokio::fs::read_to_string(&rewards_file).await?;
            if let Ok(loaded_rewards) = serde_json::from_str::<Vec<RewardEntry>>(&data) {
                *self.rewards.write().await = loaded_rewards;
                debug!("Loaded {} rewards from disk", self.rewards.read().await.len());
            }
        }

        Ok(())
    }

    /// Persist data to disk
    async fn persist_to_disk(&self) -> Result<()> {
        // Save wallets
        let wallets_file = self.storage_path.join("wallets.json");
        let wallets_data = serde_json::to_string_pretty(&*self.wallets.read().await)?;
        tokio::fs::write(&wallets_file, wallets_data).await?;

        // Save transactions
        let transactions_file = self.storage_path.join("transactions.json");
        let transactions_data = serde_json::to_string_pretty(&*self.transactions.read().await)?;
        tokio::fs::write(&transactions_file, transactions_data).await?;

        // Save rewards
        let rewards_file = self.storage_path.join("rewards.json");
        let rewards_data = serde_json::to_string_pretty(&*self.rewards.read().await)?;
        tokio::fs::write(&rewards_file, rewards_data).await?;

        Ok(())
    }

    // Wallet operations
    pub async fn create_wallet(&self, user_id: String) -> Result<Wallet> {
        let wallet = Wallet {
            wallet_id: Uuid::new_v4().to_string(),
            user_id,
            balance: Decimal::ZERO,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            is_active: true,
        };

        self.wallets.write().await.insert(wallet.wallet_id.clone(), wallet.clone());
        self.persist_to_disk().await?;

        info!("Created wallet {} for user {}", wallet.wallet_id, wallet.user_id);
        Ok(wallet)
    }

    pub async fn get_wallet(&self, wallet_id: &str) -> Result<Option<Wallet>> {
        Ok(self.wallets.read().await.get(wallet_id).cloned())
    }

    pub async fn get_wallets_by_user(&self, user_id: &str) -> Result<Vec<Wallet>> {
        let wallets = self.wallets.read().await;
        let user_wallets: Vec<Wallet> = wallets
            .values()
            .filter(|w| w.user_id == user_id)
            .cloned()
            .collect();
        Ok(user_wallets)
    }

    pub async fn update_balance(&self, wallet_id: &str, new_balance: Decimal) -> Result<()> {
        let mut wallets = self.wallets.write().await;
        if let Some(wallet) = wallets.get_mut(wallet_id) {
            wallet.balance = new_balance;
            wallet.last_activity = Utc::now();
        } else {
            return Err(anyhow!("Wallet not found"));
        }
        drop(wallets);
        self.persist_to_disk().await?;
        Ok(())
    }

    // Transaction operations
    pub async fn create_transaction(&self, tx: Transaction) -> Result<()> {
        self.transactions.write().await.insert(tx.transaction_id.clone(), tx.clone());
        self.persist_to_disk().await?;
        info!("Created transaction {}", tx.transaction_id);
        Ok(())
    }

    pub async fn get_transaction(&self, tx_id: &str) -> Result<Option<Transaction>> {
        Ok(self.transactions.read().await.get(tx_id).cloned())
    }

    pub async fn get_wallet_transactions(&self, wallet_id: &str) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().await;
        let wallet_txs: Vec<Transaction> = transactions
            .values()
            .filter(|tx| tx.from_wallet == wallet_id || tx.to_wallet == wallet_id)
            .cloned()
            .collect();
        Ok(wallet_txs)
    }

    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().await;
        let recent: Vec<Transaction> = transactions
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        Ok(recent)
    }

    // Reward operations
    pub async fn create_reward(&self, reward: RewardEntry) -> Result<()> {
        self.rewards.write().await.push(reward.clone());
        self.persist_to_disk().await?;
        info!("Created reward {} for wallet {}", reward.reward_id, reward.wallet_id);
        Ok(())
    }

    pub async fn get_wallet_rewards(&self, wallet_id: &str) -> Result<Vec<RewardEntry>> {
        let rewards = self.rewards.read().await;
        let wallet_rewards: Vec<RewardEntry> = rewards
            .iter()
            .filter(|r| r.wallet_id == wallet_id)
            .cloned()
            .collect();
        Ok(wallet_rewards)
    }

    pub async fn get_recent_rewards(&self, limit: usize) -> Result<Vec<RewardEntry>> {
        let rewards = self.rewards.read().await;
        let recent: Vec<RewardEntry> = rewards
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        Ok(recent)
    }

    // Economic metrics operations
    pub async fn save_metrics(&self, metrics: serde_json::Value) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        self.metrics.write().await.insert(timestamp, metrics);

        // Keep only last 1000 metric entries
        let mut metrics = self.metrics.write().await;
        if metrics.len() > 1000 {
            let to_remove: Vec<String> = metrics
                .keys()
                .take(metrics.len() - 1000)
                .cloned()
                .collect();
            for key in to_remove {
                metrics.remove(&key);
            }
        }
        drop(metrics);

        self.persist_to_disk().await?;
        Ok(())
    }

    pub async fn get_latest_metrics(&self) -> Result<Option<serde_json::Value>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.values().last().cloned())
    }

    pub async fn get_metrics_history(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let metrics = self.metrics.read().await;
        // Convert to sorted vec since HashMap values don't support reverse iteration
        let mut history: Vec<serde_json::Value> = metrics
            .values()
            .cloned()
            .collect();
        history.reverse();
        history.truncate(limit);
        Ok(history)
    }

    // Statistics
    pub async fn get_total_supply(&self) -> Result<Decimal> {
        let wallets = self.wallets.read().await;
        let total = wallets
            .values()
            .fold(Decimal::ZERO, |acc, w| acc + w.balance);
        Ok(total)
    }

    pub async fn get_active_wallets_count(&self) -> Result<usize> {
        let wallets = self.wallets.read().await;
        let active = wallets
            .values()
            .filter(|w| w.is_active)
            .count();
        Ok(active)
    }

    pub async fn get_transaction_volume(&self, since: DateTime<Utc>) -> Result<Decimal> {
        let transactions = self.transactions.read().await;
        let volume = transactions
            .values()
            .filter(|tx| tx.created_at >= since)
            .fold(Decimal::ZERO, |acc, tx| acc + tx.amount);
        Ok(volume)
    }

    // Staking operations (stub implementations for now)
    pub async fn get_stakes(&self, _wallet_id: &str) -> Result<Vec<StakeInfo>> {
        // TODO: Implement actual stake storage
        Ok(vec![])
    }

    pub async fn get_total_staked(&self, _wallet_id: &str) -> Result<Decimal> {
        // TODO: Implement actual stake tracking
        Ok(Decimal::ZERO)
    }

    pub async fn create_stake(&self, _stake: StakeInfo) -> Result<()> {
        // TODO: Implement actual stake creation
        Ok(())
    }

    pub async fn deactivate_stake(&self, _stake_id: &str) -> Result<()> {
        // TODO: Implement actual stake deactivation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let config = DatabaseConfig {
            url: temp_dir.path().to_str().unwrap().to_string(),
            redis_url: None,
            pool_size: 5,
        };

        let storage = CaesarStorage::new(config).await.unwrap();

        // Create wallet
        let user_id = Uuid::new_v4().to_string();
        let wallet = storage.create_wallet(user_id).await.unwrap();
        assert_eq!(wallet.balance, Decimal::ZERO);

        // Get wallet
        let retrieved = storage.get_wallet(&wallet.wallet_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().wallet_id, wallet.wallet_id);
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = DatabaseConfig {
            url: temp_dir.path().to_str().unwrap().to_string(),
            redis_url: None,
            pool_size: 5,
        };

        let user_id = Uuid::new_v4().to_string();
        let wallet_id;

        // Create and save
        {
            let storage = CaesarStorage::new(config.clone()).await.unwrap();
            let wallet = storage.create_wallet(user_id.clone()).await.unwrap();
            wallet_id = wallet.wallet_id;
        }

        // Reload and verify
        {
            let storage = CaesarStorage::new(config).await.unwrap();
            let wallet = storage.get_wallet(&wallet_id).await.unwrap();
            assert!(wallet.is_some());
            assert_eq!(wallet.unwrap().user_id, user_id);
        }
    }
}
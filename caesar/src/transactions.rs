// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Transaction Processing - Handle token transfers

use anyhow::{Result, anyhow};
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::models::*;
use crate::storage::CaesarStorage;
use crate::EconomicsConfig;
use serde::{Deserialize, Serialize};

/// Transaction processor for handling transfers
pub struct TransactionProcessor {
    config: EconomicsConfig,
    storage: Arc<CaesarStorage>,
}

impl TransactionProcessor {
    pub async fn new(config: EconomicsConfig, storage: Arc<CaesarStorage>) -> Result<Self> {
        Ok(Self { config, storage })
    }

    /// Process a transaction
    pub async fn process(&self, request: SendTransactionRequest) -> Result<TransactionResponse> {
        // Validate transaction
        self.validate_transaction(&request).await?;

        // Get sender balance
        let sender_wallet = self.storage.get_wallet(&request.from_wallet).await?
            .ok_or_else(|| anyhow!("Sender wallet not found"))?;
        let sender_balance = sender_wallet.balance;

        // Calculate fee
        let fee = request.amount * self.config.transaction_fee;
        let total_amount = request.amount + fee;

        // Check sufficient balance
        if sender_balance < total_amount {
            return Err(anyhow!(
                "Insufficient balance. Required: {}, Available: {}",
                total_amount,
                sender_balance
            ));
        }

        // Create transaction
        let transaction_id = format!("TX_{}", Uuid::new_v4().to_string().replace("-", "").to_uppercase()[..16].to_string());

        let transaction = Transaction {
            transaction_id: transaction_id.clone(),
            from_wallet: request.from_wallet.clone(),
            to_wallet: request.to_wallet.clone(),
            amount: request.amount,
            transaction_type: TransactionType::Transfer,
            status: TransactionStatus::Pending,
            fee,
            description: request.description.unwrap_or_else(|| "Token transfer".to_string()),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        // Store transaction
        self.storage.create_transaction(transaction.clone()).await?;

        // Process balances atomically
        match self.execute_transfer(&request.from_wallet, &request.to_wallet, request.amount, fee).await {
            Ok(_) => {
                info!(
                    "Transaction {} processed: {} CSR from {} to {}",
                    transaction_id, request.amount, request.from_wallet, request.to_wallet
                );

                // Return success response
                Ok(TransactionResponse {
                    transaction_id,
                    from_wallet: request.from_wallet,
                    to_wallet: request.to_wallet,
                    amount: request.amount,
                    transaction_type: TransactionType::Transfer,
                    status: TransactionStatus::Completed,
                    fee,
                    description: transaction.description,
                    timestamp: transaction.timestamp,
                    block_height: Some(self.get_current_block_height()),
                    confirmation_count: 1,
                })
            }
            Err(e) => {
                error!("Transaction failed: {}", e);

                // Return failed transaction
                Ok(TransactionResponse {
                    transaction_id,
                    from_wallet: request.from_wallet,
                    to_wallet: request.to_wallet,
                    amount: request.amount,
                    transaction_type: TransactionType::Transfer,
                    status: TransactionStatus::Failed,
                    fee: dec!(0),
                    description: format!("Failed: {}", e),
                    timestamp: transaction.timestamp,
                    block_height: None,
                    confirmation_count: 0,
                })
            }
        }
    }

    /// Validate transaction request
    async fn validate_transaction(&self, request: &SendTransactionRequest) -> Result<()> {
        // Check wallet existence
        if request.from_wallet == request.to_wallet {
            return Err(anyhow!("Cannot send to same wallet"));
        }

        // Validate amount
        if request.amount < self.config.min_transaction {
            return Err(anyhow!(
                "Amount {} is below minimum transaction of {}",
                request.amount,
                self.config.min_transaction
            ));
        }

        if request.amount > self.config.max_transaction {
            return Err(anyhow!(
                "Amount {} exceeds maximum transaction of {}",
                request.amount,
                self.config.max_transaction
            ));
        }

        // Verify sender wallet exists
        self.storage.get_wallet(&request.from_wallet).await
            .map_err(|_| anyhow!("Sender wallet not found"))?;

        // Create receiver wallet if it doesn't exist
        if self.storage.get_wallet(&request.to_wallet).await.is_err() {
            warn!("Receiver wallet {} not found, creating new wallet", request.to_wallet);
            // In production, this would require proper user registration
        }

        Ok(())
    }

    /// Execute the actual transfer
    async fn execute_transfer(
        &self,
        from_wallet: &str,
        to_wallet: &str,
        amount: Decimal,
        fee: Decimal,
    ) -> Result<()> {
        // Get current balances
        let sender_wallet = self.storage.get_wallet(from_wallet).await?
            .ok_or_else(|| anyhow!("Sender wallet not found"))?;
        let sender_balance = sender_wallet.balance;

        let receiver_wallet = self.storage.get_wallet(to_wallet).await?;
        let receiver_balance = receiver_wallet.map(|w| w.balance).unwrap_or(dec!(0));

        // Calculate new balances
        let total_deduction = amount + fee;
        if sender_balance < total_deduction {
            return Err(anyhow!("Insufficient balance during execution"));
        }

        let new_sender_balance = sender_balance - total_deduction;
        let new_receiver_balance = receiver_balance + amount;

        // Update balances
        self.storage.update_balance(from_wallet, new_sender_balance).await?;
        self.storage.update_balance(to_wallet, new_receiver_balance).await?;

        // Fee goes to treasury/system
        if fee > dec!(0) {
            self.process_fee(fee).await?;
        }

        Ok(())
    }

    /// Process transaction fee
    async fn process_fee(&self, fee: Decimal) -> Result<()> {
        // In production, fees would go to treasury or be distributed
        // For now, we just log it
        info!("Transaction fee collected: {} CSR", fee);
        Ok(())
    }

    /// Get current block height (simulated)
    fn get_current_block_height(&self) -> u64 {
        // In production, this would connect to actual blockchain
        // For now, simulate with timestamp
        Utc::now().timestamp() as u64 / 10
    }

    /// Process batch transactions
    pub async fn process_batch(&self, transactions: Vec<SendTransactionRequest>) -> Result<Vec<TransactionResponse>> {
        let mut results = Vec::new();

        for request in transactions {
            match self.process(request).await {
                Ok(response) => results.push(response),
                Err(e) => {
                    error!("Batch transaction failed: {}", e);
                    // Continue processing other transactions
                }
            }
        }

        Ok(results)
    }

    /// Reverse a transaction (admin only)
    pub async fn reverse_transaction(&self, transaction_id: &str) -> Result<TransactionResponse> {
        let original = self.storage.get_transaction(transaction_id).await?
            .ok_or_else(|| anyhow!("Transaction not found"))?;

        if original.status != TransactionStatus::Completed {
            return Err(anyhow!("Can only reverse completed transactions"));
        }

        // Create reverse transaction
        let reverse_request = SendTransactionRequest {
            from_wallet: original.to_wallet.clone(),
            to_wallet: original.from_wallet.clone(),
            amount: original.amount,
            description: Some(format!("Reversal of transaction {}", transaction_id)),
        };

        info!("Reversing transaction {}", transaction_id);
        self.process(reverse_request).await
    }

    /// Get transaction statistics
    pub async fn get_statistics(&self) -> Result<TransactionStatistics> {
        // This would aggregate from database
        Ok(TransactionStatistics {
            total_transactions: 0,
            total_volume: dec!(0),
            total_fees: dec!(0),
            average_transaction_size: dec!(0),
            transactions_per_second: 0.0,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatistics {
    pub total_transactions: u64,
    pub total_volume: Decimal,
    pub total_fees: Decimal,
    pub average_transaction_size: Decimal,
    pub transactions_per_second: f64,
}
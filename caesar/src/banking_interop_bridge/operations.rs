// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Banking interoperability bridge operations - bridge transactions and scheduling

use anyhow::{anyhow, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use uuid::Uuid;

use super::types::*;
use super::BankingInteropBridge;

impl BankingInteropBridge {
    /// Execute fiat to crypto bridge transaction
    pub async fn bridge_fiat_to_crypto(
        &self,
        banking_creds: &BankingCredentials,
        from_account: &str,
        to_crypto_address: &str,
        amount: Decimal,
        target_crypto: &str,
        velocity_zone: Option<&str>,
    ) -> Result<InteropTransaction> {
        let transaction_id = format!("FIAT_CRYPTO_{}", Uuid::new_v4());

        let banking_provider = self
            .banking_providers
            .get(&banking_creds.provider)
            .ok_or_else(|| anyhow!("Banking provider not registered"))?;

        let auth = banking_provider.authenticate(banking_creds).await?;
        let balance = banking_provider
            .get_account_balance(&auth, from_account)
            .await?;
        if balance.available < amount {
            return Err(anyhow!("Insufficient funds"));
        }

        let velocity_adjustment = self
            .calculate_velocity_adjustment(velocity_zone, amount)
            .await?;
        let exchange_rate = self.get_crypto_exchange_rate("USD", target_crypto).await?;

        let fees = BridgeFees {
            network_fee: dec!(0.001),
            provider_fee: amount * dec!(0.0029),
            bridge_fee: amount * dec!(0.005),
            velocity_adjustment,
            total_fee: dec!(0),
        };
        let mut total_fees =
            fees.network_fee + fees.provider_fee + fees.bridge_fee + fees.velocity_adjustment;

        if velocity_adjustment < dec!(0) {
            total_fees = total_fees.max(dec!(0));
        }

        let final_fees = BridgeFees {
            total_fee: total_fees,
            ..fees
        };

        let transaction = InteropTransaction {
            transaction_id: transaction_id.clone(),
            bridge_type: BridgeType::FiatToCrypto,
            source_asset: AssetType::Fiat {
                currency: "USD".to_string(),
            },
            destination_asset: AssetType::Crypto {
                symbol: target_crypto.to_string(),
                chain: "ethereum".to_string(),
            },
            amount,
            source_provider: format!("{:?}", banking_creds.provider),
            destination_provider: "LayerZero".to_string(),
            exchange_rate,
            fees: final_fees,
            status: InteropStatus::Processing,
            velocity_zone: velocity_zone.map(String::from),
            contract_reference: None,
            timestamp: Utc::now(),
            completion_time: None,
            metadata: HashMap::new(),
        };

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.insert(transaction_id.clone(), transaction.clone());
        }

        let payment_request = PaymentRequest {
            from_account: from_account.to_string(),
            to_account: "CAESAR_BRIDGE_ACCOUNT".to_string(),
            amount: amount + total_fees,
            currency: "USD".to_string(),
            reference: transaction_id.clone(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("bridge_transaction".to_string(), "true".to_string());
                meta.insert("target_crypto".to_string(), target_crypto.to_string());
                meta.insert("target_address".to_string(), to_crypto_address.to_string());
                meta
            },
        };

        let _payment_response = banking_provider
            .initiate_payment(&auth, &payment_request)
            .await?;
        self.schedule_crypto_minting(&transaction_id, to_crypto_address, &transaction)
            .await?;

        Ok(transaction)
    }

    /// Execute crypto to fiat bridge transaction
    pub async fn bridge_crypto_to_fiat(
        &self,
        _crypto_creds: &CryptoCredentials,
        banking_creds: &BankingCredentials,
        from_crypto_address: &str,
        to_account: &str,
        amount: Decimal,
        source_crypto: &str,
        velocity_zone: Option<&str>,
    ) -> Result<InteropTransaction> {
        let transaction_id = format!("CRYPTO_FIAT_{}", Uuid::new_v4());

        let exchange_rate = self.get_crypto_exchange_rate(source_crypto, "USD").await?;
        let velocity_adjustment = self
            .calculate_velocity_adjustment(velocity_zone, amount)
            .await?;

        let fees = BridgeFees {
            network_fee: dec!(0.002),
            provider_fee: amount * dec!(0.0029),
            bridge_fee: amount * dec!(0.007),
            velocity_adjustment,
            total_fee: dec!(0),
        };

        let total_fees =
            fees.network_fee + fees.provider_fee + fees.bridge_fee + fees.velocity_adjustment;

        let transaction = InteropTransaction {
            transaction_id: transaction_id.clone(),
            bridge_type: BridgeType::CryptoToFiat,
            source_asset: AssetType::Crypto {
                symbol: source_crypto.to_string(),
                chain: "ethereum".to_string(),
            },
            destination_asset: AssetType::Fiat {
                currency: "USD".to_string(),
            },
            amount,
            source_provider: "LayerZero".to_string(),
            destination_provider: format!("{:?}", banking_creds.provider),
            exchange_rate,
            fees: BridgeFees {
                total_fee: total_fees,
                ..fees
            },
            status: InteropStatus::Processing,
            velocity_zone: velocity_zone.map(String::from),
            contract_reference: None,
            timestamp: Utc::now(),
            completion_time: None,
            metadata: HashMap::new(),
        };

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.insert(transaction_id.clone(), transaction.clone());
        }

        self.schedule_crypto_burning(&transaction_id, from_crypto_address, &transaction)
            .await?;
        self.schedule_fiat_transfer(&transaction_id, to_account, banking_creds, &transaction)
            .await?;

        Ok(transaction)
    }

    /// Execute crypto to crypto exchange
    pub async fn bridge_crypto_to_crypto(
        &self,
        crypto_creds: &CryptoCredentials,
        from_crypto: &str,
        to_crypto: &str,
        amount: Decimal,
        exchange: CryptoExchange,
        velocity_zone: Option<&str>,
    ) -> Result<InteropTransaction> {
        let transaction_id = format!("CRYPTO_CRYPTO_{}", Uuid::new_v4());

        let exchange_provider = self
            .crypto_providers
            .get(&exchange)
            .ok_or_else(|| anyhow!("Crypto exchange provider not registered"))?;

        let quote = exchange_provider
            .get_quote(from_crypto, to_crypto, amount)
            .await?;
        let velocity_adjustment = self
            .calculate_velocity_adjustment(velocity_zone, amount)
            .await?;

        let fees = BridgeFees {
            network_fee: dec!(0.003),
            provider_fee: quote.fees,
            bridge_fee: dec!(0),
            velocity_adjustment,
            total_fee: dec!(0),
        };

        let total_fees = fees.network_fee + fees.provider_fee + fees.velocity_adjustment;

        let transaction = InteropTransaction {
            transaction_id: transaction_id.clone(),
            bridge_type: BridgeType::CryptoToCrypto,
            source_asset: AssetType::Crypto {
                symbol: from_crypto.to_string(),
                chain: "ethereum".to_string(),
            },
            destination_asset: AssetType::Crypto {
                symbol: to_crypto.to_string(),
                chain: "ethereum".to_string(),
            },
            amount,
            source_provider: format!("{exchange:?}"),
            destination_provider: format!("{exchange:?}"),
            exchange_rate: quote.exchange_rate,
            fees: BridgeFees {
                total_fee: total_fees,
                ..fees
            },
            status: InteropStatus::Processing,
            velocity_zone: velocity_zone.map(String::from),
            contract_reference: None,
            timestamp: Utc::now(),
            completion_time: None,
            metadata: HashMap::new(),
        };

        let swap_request = SwapRequest {
            from_token: from_crypto.to_string(),
            to_token: to_crypto.to_string(),
            amount,
            slippage_tolerance: quote.slippage_tolerance,
            recipient: crypto_creds.address.clone(),
        };

        let swap_result = exchange_provider
            .execute_swap(crypto_creds, &swap_request)
            .await?;

        let mut updated_transaction = transaction;
        updated_transaction.status = InteropStatus::Completed;
        updated_transaction.completion_time = Some(Utc::now());
        updated_transaction
            .metadata
            .insert("tx_hash".to_string(), swap_result.transaction_hash);

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.insert(transaction_id, updated_transaction.clone());
        }

        Ok(updated_transaction)
    }

    /// Get transaction status
    pub async fn get_transaction_status(&self, transaction_id: &str) -> Result<InteropTransaction> {
        let transactions = self.active_transactions.read().await;
        transactions
            .get(transaction_id)
            .cloned()
            .ok_or_else(|| anyhow!("Transaction not found"))
    }

    /// List all active transactions
    pub async fn list_active_transactions(&self) -> Result<Vec<InteropTransaction>> {
        let transactions = self.active_transactions.read().await;
        Ok(transactions.values().cloned().collect())
    }

    /// Add new velocity zone
    pub async fn add_velocity_zone(&self, zone: VelocityZone) -> Result<()> {
        let mut zones = self.velocity_zones.write().await;
        zones.insert(zone.zone_id.clone(), zone);
        Ok(())
    }

    /// Update exchange rates (called by price feed service)
    pub async fn update_exchange_rates(
        &self,
        from: &str,
        rates: HashMap<String, Decimal>,
    ) -> Result<()> {
        let mut exchange_rates = self.exchange_rates.write().await;
        exchange_rates.insert(from.to_string(), rates);
        Ok(())
    }

    /// Schedule crypto minting (integration point with LayerZero/contracts)
    pub(crate) async fn schedule_crypto_minting(
        &self,
        transaction_id: &str,
        recipient: &str,
        _transaction: &InteropTransaction,
    ) -> Result<()> {
        let mut transactions = self.active_transactions.write().await;
        if let Some(tx) = transactions.get_mut(transaction_id) {
            tx.status = InteropStatus::Completed;
            tx.completion_time = Some(Utc::now());
            tx.metadata
                .insert("recipient".to_string(), recipient.to_string());
            tx.metadata
                .insert("action".to_string(), "crypto_minted".to_string());
        }
        Ok(())
    }

    /// Schedule crypto burning (integration point with LayerZero/contracts)
    pub(crate) async fn schedule_crypto_burning(
        &self,
        transaction_id: &str,
        from_address: &str,
        _transaction: &InteropTransaction,
    ) -> Result<()> {
        let mut transactions = self.active_transactions.write().await;
        if let Some(tx) = transactions.get_mut(transaction_id) {
            tx.metadata
                .insert("from_address".to_string(), from_address.to_string());
            tx.metadata
                .insert("action".to_string(), "crypto_burned".to_string());
        }
        Ok(())
    }

    /// Schedule fiat transfer (integration point with banking providers)
    pub(crate) async fn schedule_fiat_transfer(
        &self,
        transaction_id: &str,
        to_account: &str,
        _banking_creds: &BankingCredentials,
        _transaction: &InteropTransaction,
    ) -> Result<()> {
        let mut transactions = self.active_transactions.write().await;
        if let Some(tx) = transactions.get_mut(transaction_id) {
            tx.metadata
                .insert("to_account".to_string(), to_account.to_string());
            tx.metadata
                .insert("action".to_string(), "fiat_transferred".to_string());
            tx.status = InteropStatus::Completed;
            tx.completion_time = Some(Utc::now());
        }
        Ok(())
    }
}

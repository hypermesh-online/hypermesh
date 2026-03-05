// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS Registration System
//!
//! Handles DNS record registration with blockchain integration.

use super::{DnsError, DnsPoolManager, DnsRecord, DnsResult, Domain};
use super::domain::DomainRegistration;
use super::pools::PoolVisibility;
use crate::blockchain::NodeBlockchain;
use crate::bootstrap::PrivacyMode;
use crate::proof_of_state::StateProof;
use crate::dns::validation::DnsValidator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Registration status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistrationStatus {
    /// Pending blockchain confirmation
    Pending,
    /// Registered and active
    Active,
    /// Registration failed
    Failed { reason: String },
    /// Registration expired
    Expired,
}

/// DNS registration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRegistration {
    /// Domain
    pub domain: Domain,
    /// DNS record
    pub record: DnsRecord,
    /// Registration status
    pub status: RegistrationStatus,
    /// Blockchain transaction hash
    pub tx_hash: Option<String>,
    /// State proof used for registration
    pub state_proof: Option<Vec<u8>>,
}

/// DNS registrar
pub struct DnsRegistrar {
    /// Pool manager
    pool_manager: Arc<DnsPoolManager>,
    /// DNS validator
    validator: Arc<DnsValidator>,
    /// Node blockchain (for registration)
    blockchain: Arc<RwLock<Option<Arc<RwLock<NodeBlockchain>>>>>,
    /// Active registrations (domain -> registration)
    registrations: Arc<RwLock<HashMap<String, DnsRegistration>>>,
    /// Domain registrations (domain_name -> DomainRegistration)
    domain_registrations: Arc<RwLock<HashMap<String, DomainRegistration>>>,
}

impl DnsRegistrar {
    /// Create new DNS registrar
    pub fn new(pool_manager: Arc<DnsPoolManager>, validator: Arc<DnsValidator>) -> Self {
        Self {
            pool_manager,
            validator,
            blockchain: Arc::new(RwLock::new(None)),
            registrations: Arc::new(RwLock::new(HashMap::new())),
            domain_registrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set blockchain instance
    pub async fn set_blockchain(&self, blockchain: Arc<RwLock<NodeBlockchain>>) {
        let mut bc = self.blockchain.write().await;
        *bc = Some(blockchain);
    }

    /// Register public DNS record
    pub async fn register_public(
        &self,
        domain: Domain,
        record: DnsRecord,
        proof: StateProof,
    ) -> DnsResult<DnsRegistration> {
        info!("Registering public DNS: {}", domain.full);

        // Validate registration
        let validation = self
            .validator
            .validate_registration(&domain, &proof)
            .await?;

        if !validation.valid {
            warn!("Registration validation failed for: {}", domain.full);
            return Err(DnsError::RegistrationFailed {
                reason: validation.reason.unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        // Register to blockchain
        let tx_hash = self
            .register_to_blockchain(&domain, &record, &proof)
            .await?;

        // Add to public pool
        self.pool_manager.register_public(record.clone()).await?;

        // Create registration record
        let registration = DnsRegistration {
            domain: domain.clone(),
            record: record.clone(),
            status: RegistrationStatus::Active,
            tx_hash: Some(tx_hash.clone()),
            state_proof: Some(proof.to_bytes().unwrap_or_default()),
        };

        // Store registration
        let mut registrations = self.registrations.write().await;
        registrations.insert(domain.full.clone(), registration.clone());

        info!(
            "✅ Public DNS registered: {} (tx: {})",
            domain.full, tx_hash
        );
        Ok(registration)
    }

    /// Register federated DNS record
    pub async fn register_federated(
        &self,
        domain: Domain,
        network_id: String,
        record: DnsRecord,
        proof: StateProof,
    ) -> DnsResult<DnsRegistration> {
        info!(
            "Registering federated DNS: {} (network: {})",
            domain.full, network_id
        );

        // Validate registration
        let validation = self
            .validator
            .validate_registration(&domain, &proof)
            .await?;

        if !validation.valid {
            warn!("Registration validation failed for: {}", domain.full);
            return Err(DnsError::RegistrationFailed {
                reason: validation.reason.unwrap_or_else(|| "Unknown".to_string()),
            });
        }

        // Register to blockchain
        let tx_hash = self
            .register_to_blockchain(&domain, &record, &proof)
            .await?;

        // Add to federated pool
        self.pool_manager
            .register_federated(network_id.clone(), record.clone())
            .await?;

        // Create registration record
        let registration = DnsRegistration {
            domain: domain.clone(),
            record: record.clone(),
            status: RegistrationStatus::Active,
            tx_hash: Some(tx_hash.clone()),
            state_proof: Some(proof.to_bytes().unwrap_or_default()),
        };

        // Store registration
        let mut registrations = self.registrations.write().await;
        registrations.insert(domain.full.clone(), registration.clone());

        info!(
            "✅ Federated DNS registered: {} (network: {}, tx: {})",
            domain.full, network_id, tx_hash
        );
        Ok(registration)
    }

    /// Get registration status
    pub async fn get_registration(&self, domain: &str) -> DnsResult<Option<DnsRegistration>> {
        let registrations = self.registrations.read().await;
        Ok(registrations.get(domain).cloned())
    }

    /// List all registrations
    pub async fn list_registrations(&self) -> Vec<DnsRegistration> {
        let registrations = self.registrations.read().await;
        registrations.values().cloned().collect()
    }

    /// Register a domain as a blockchain asset.
    ///
    /// Creates a Network-scope chain ID, validates parent hierarchy,
    /// registers to blockchain, and creates the corresponding DNS pool.
    pub async fn register_domain(
        &self,
        domain_name: &str,
        privacy_mode: PrivacyMode,
        owner_node_id: String,
        proof: StateProof,
    ) -> DnsResult<DomainRegistration> {
        // Validate domain name format
        Self::validate_domain_name(domain_name)?;

        // Check not already registered
        {
            let domains = self.domain_registrations.read().await;
            if domains.contains_key(domain_name) {
                return Err(DnsError::DomainAlreadyRegistered {
                    domain: domain_name.to_string(),
                });
            }
        }

        // If domain has a parent, verify parent is registered
        if let Some(parent) = extract_parent(domain_name) {
            let domains = self.domain_registrations.read().await;
            if !domains.contains_key(&parent) {
                return Err(DnsError::ParentDomainRequired { parent });
            }
        }

        // Create the registration
        let mut reg = DomainRegistration::new(domain_name, privacy_mode, owner_node_id);
        reg.state_proof_bytes = Some(proof.to_bytes().unwrap_or_default());

        // Register to blockchain as DNS asset
        let domain_parsed = Domain::parse(domain_name).map_err(|_| DnsError::InvalidDomain {
            domain: domain_name.to_string(),
        })?;
        let record = DnsRecord::new(
            domain_name.to_string(),
            crate::dns::DnsRecordType::TXT,
            crate::dns::DnsRecordData::TXT(format!(
                "DOMAIN:REGISTER:{}:{}",
                reg.network_id, reg.chain_id.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )),
            3600,
            reg.owner_node_id.clone(),
        );
        let _tx_hash = self
            .register_to_blockchain(&domain_parsed, &record, &proof)
            .await?;

        // Create domain pool
        let visibility = match privacy_mode {
            p if p == PrivacyMode::PUBLIC => PoolVisibility::Public,
            p if p == PrivacyMode::ANONYMOUS => PoolVisibility::FullyFederated,
            _ => PoolVisibility::NetworkRestricted,
        };
        self.pool_manager
            .create_domain_pool(&reg.network_id, visibility)
            .await?;

        // Store
        let mut domains = self.domain_registrations.write().await;
        domains.insert(domain_name.to_string(), reg.clone());

        info!("Domain registered: {} (network: {})", domain_name, reg.network_id);
        Ok(reg)
    }

    /// Get a domain registration by name.
    pub async fn get_domain(&self, domain_name: &str) -> Option<DomainRegistration> {
        let domains = self.domain_registrations.read().await;
        domains.get(domain_name).cloned()
    }

    /// List all domain registrations.
    pub async fn list_domains(&self) -> Vec<DomainRegistration> {
        let domains = self.domain_registrations.read().await;
        domains.values().cloned().collect()
    }

    /// Validate domain name: max 63 chars per component, max 253 total, non-empty.
    fn validate_domain_name(domain_name: &str) -> DnsResult<()> {
        if domain_name.is_empty() || domain_name.len() > 253 {
            return Err(DnsError::InvalidDomain {
                domain: domain_name.to_string(),
            });
        }
        for component in domain_name.split('.') {
            if component.is_empty() || component.len() > 63 {
                return Err(DnsError::InvalidDomain {
                    domain: domain_name.to_string(),
                });
            }
        }
        Ok(())
    }

    // Internal helper methods

    async fn register_to_blockchain(
        &self,
        domain: &Domain,
        record: &DnsRecord,
        proof: &StateProof,
    ) -> DnsResult<String> {
        let blockchain_opt = self.blockchain.read().await;

        match blockchain_opt.as_ref() {
            Some(blockchain) => {
                // Create blockchain transaction for DNS registration
                let bc = blockchain.write().await;
                let tx_data = format!("DNS Registration: {} -> {:?}", domain.full, record.data);
                let block = bc
                    .add_block_with_data(tx_data.into_bytes(), proof)
                    .await
                    .map_err(|e| DnsError::BlockchainError(e.to_string()))?;

                Ok(block.hash.clone())
            }
            None => {
                // No blockchain available - generate mock transaction hash
                warn!("No blockchain instance available, using mock transaction");
                Ok(format!("mock-tx-{}", uuid::Uuid::new_v4()))
            }
        }
    }
}

/// Extract the parent domain name (everything after first dot).
fn extract_parent(domain_name: &str) -> Option<String> {
    let first_dot = domain_name.find('.')?;
    let parent = &domain_name[first_dot + 1..];
    if parent.is_empty() {
        None
    } else {
        Some(parent.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use crate::dns::{DnsRecordData, DnsRecordType};
    use std::net::Ipv6Addr;
    use std::time::Duration;

    fn create_test_record(domain: &str) -> DnsRecord {
        DnsRecord::new(
            domain.to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            "node-1".to_string(),
        )
    }

    fn create_test_proof() -> StateProof {
        let stake = StakeProof::new("holder".to_string(), "holder-id".to_string(), 1000);
        let time = TimeProof::new(Duration::from_secs(10));
        let space = SpaceProof::new("node".to_string(), "/storage".to_string(), 1024 * 1024);
        let work = WorkProof::new(
            "owner".to_string(),
            "workload".to_string(),
            12345,
            100,
            WorkloadType::Compute,
            WorkState::Completed,
        );

        StateProof::new(stake, time, space, work)
    }

    #[tokio::test]
    async fn test_public_registration() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let registrar = DnsRegistrar::new(pool_manager, validator);

        let domain = Domain::parse("nike").expect("test: expected success");
        let record = create_test_record("nike");
        let proof = create_test_proof();

        let registration = registrar
            .register_public(domain, record, proof)
            .await
            .expect("test: expected success");

        assert_eq!(registration.status, RegistrationStatus::Active);
        assert!(registration.tx_hash.is_some());
    }

    #[tokio::test]
    async fn test_federated_registration() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let registrar = DnsRegistrar::new(pool_manager, validator);

        let domain = Domain::parse("admin.nike").expect("test: expected success");
        let record = create_test_record("admin.nike");
        let proof = create_test_proof();

        let registration = registrar
            .register_federated(domain, "nike-internal".to_string(), record, proof)
            .await
            .expect("test: expected success");

        assert_eq!(registration.status, RegistrationStatus::Active);
        assert!(registration.tx_hash.is_some());
    }

    #[tokio::test]
    async fn test_get_registration() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let registrar = DnsRegistrar::new(pool_manager, validator);

        let domain = Domain::parse("nike").expect("test: expected success");
        let record = create_test_record("nike");
        let proof = create_test_proof();

        registrar
            .register_public(domain, record, proof)
            .await
            .expect("test: expected success");

        let registration = registrar.get_registration("nike").await.expect("test: async operation");
        assert!(registration.is_some());
        assert_eq!(registration.expect("test: assertion value").domain.full, "nike");
    }
}

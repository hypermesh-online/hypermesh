// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bootstrap provider implementations for service discovery,
//! certificate management, transport, and Proof of State.
//!
//! Each provider tier matches a bootstrap phase:
//! - Traditional (Phase 0): Self-signed certs, DNS, basic transport, no-op state proof
//! - Hybrid (Phase 1): TrustChain certs, hybrid discovery, optional state proof
//! - Partial Federation (Phase 2): Federated discovery, required state proof
//! - Full Federation (Phase 3): Full federated discovery, full state proof

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
pub use trustchain::proof_of_state::StateProof;

use super::{
    BootstrapPhase, Certificate, CertificateProvider, Connection, StateProofProvider, Listener,
    ServiceDiscovery, ServiceEndpoint, ServiceRegistration, ServiceType, TransportProvider,
};

// --- Discovery Providers ---

pub(crate) struct TraditionalDNS {
    _servers: Vec<String>,
}

impl TraditionalDNS {
    pub fn new(servers: Vec<String>) -> Self {
        Self { _servers: servers }
    }
}

#[async_trait]
impl ServiceDiscovery for TraditionalDNS {
    async fn resolve(&self, _service: &str) -> Result<ServiceEndpoint> {
        Ok(ServiceEndpoint {
            address: format!("::1:{}", 8080).parse()?,
            service_type: ServiceType::HyperMesh,
            metadata: HashMap::new(),
        })
    }

    async fn register(&self, _registration: ServiceRegistration) -> Result<()> {
        Ok(())
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Traditional
    }
}

pub(crate) struct HybridDiscovery {
    _traditional_dns: Vec<String>,
    trustchain_addr: SocketAddr,
}

impl HybridDiscovery {
    pub fn new(traditional_dns: Vec<String>, trustchain_addr: SocketAddr) -> Self {
        Self {
            _traditional_dns: traditional_dns,
            trustchain_addr,
        }
    }
}

#[async_trait]
impl ServiceDiscovery for HybridDiscovery {
    async fn resolve(&self, _service: &str) -> Result<ServiceEndpoint> {
        Ok(ServiceEndpoint {
            address: self.trustchain_addr,
            service_type: ServiceType::TrustChain,
            metadata: HashMap::new(),
        })
    }

    async fn register(&self, _registration: ServiceRegistration) -> Result<()> {
        Ok(())
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Hybrid
    }
}

pub(crate) struct FederatedDiscovery {
    hypermesh_addr: SocketAddr,
    fallback_dns: Option<Vec<String>>,
}

impl FederatedDiscovery {
    pub fn new(hypermesh_addr: SocketAddr, fallback_dns: Option<Vec<String>>) -> Self {
        Self {
            hypermesh_addr,
            fallback_dns,
        }
    }
}

#[async_trait]
impl ServiceDiscovery for FederatedDiscovery {
    async fn resolve(&self, _service: &str) -> Result<ServiceEndpoint> {
        Ok(ServiceEndpoint {
            address: self.hypermesh_addr,
            service_type: ServiceType::HyperMesh,
            metadata: HashMap::new(),
        })
    }

    async fn register(&self, _registration: ServiceRegistration) -> Result<()> {
        Ok(())
    }

    fn phase(&self) -> BootstrapPhase {
        if self.fallback_dns.is_some() {
            BootstrapPhase::PartialFederation
        } else {
            BootstrapPhase::FullFederation
        }
    }
}

// --- Certificate Providers ---

pub(crate) struct SelfSignedProvider;

impl SelfSignedProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CertificateProvider for SelfSignedProvider {
    async fn get_certificate(&self, domain: &str) -> Result<Certificate> {
        Ok(Certificate {
            subject: domain.to_string(),
            issuer: "Self-Signed".to_string(),
            not_before: SystemTime::now(),
            not_after: SystemTime::now() + Duration::from_secs(86400),
            fingerprint: "self-signed-fingerprint".to_string(),
            is_self_signed: true,
        })
    }

    async fn validate(&self, cert: &Certificate) -> Result<bool> {
        Ok(cert.is_self_signed)
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Traditional
    }
}

pub(crate) struct TrustChainProvider {
    _trustchain_addr: SocketAddr,
}

impl TrustChainProvider {
    pub fn new(trustchain_addr: SocketAddr) -> Self {
        Self {
            _trustchain_addr: trustchain_addr,
        }
    }
}

#[async_trait]
impl CertificateProvider for TrustChainProvider {
    async fn get_certificate(&self, domain: &str) -> Result<Certificate> {
        Ok(Certificate {
            subject: domain.to_string(),
            issuer: "TrustChain CA".to_string(),
            not_before: SystemTime::now(),
            not_after: SystemTime::now() + Duration::from_secs(86400 * 90),
            fingerprint: "trustchain-fingerprint".to_string(),
            is_self_signed: false,
        })
    }

    async fn validate(&self, cert: &Certificate) -> Result<bool> {
        Ok(!cert.is_self_signed)
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Hybrid
    }
}

// --- Transport Providers ---

pub(crate) struct BasicTransport;

impl BasicTransport {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TransportProvider for BasicTransport {
    async fn connect(&self, endpoint: &ServiceEndpoint) -> Result<Connection> {
        Ok(Connection {
            id: uuid::Uuid::new_v4().to_string(),
            remote_addr: endpoint.address,
            local_addr: "[::1]:0".parse()?,
            established_at: SystemTime::now(),
        })
    }

    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>> {
        Ok(Box::new(BasicListener { addr }))
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Traditional
    }
}

pub(crate) struct BasicListener {
    addr: SocketAddr,
}

#[async_trait]
impl Listener for BasicListener {
    async fn accept(&self) -> Result<Connection> {
        Ok(Connection {
            id: uuid::Uuid::new_v4().to_string(),
            remote_addr: "[::1]:0".parse()?,
            local_addr: self.addr,
            established_at: SystemTime::now(),
        })
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.addr)
    }
}

// --- State Proof Providers ---

pub(crate) struct NoOpStateProof;

impl NoOpStateProof {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StateProofProvider for NoOpStateProof {
    async fn validate_proof(&self, _proof: &StateProof) -> Result<bool> {
        Ok(true)
    }

    async fn generate_proof(&self, _data: &[u8]) -> Result<StateProof> {
        use trustchain::proof_of_state::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        Ok(StateProof::new(
            StakeProof::new("noop".to_string(), "noop".to_string()),
            TimeProof::new(Duration::from_secs(0)),
            SpaceProof::new("noop".to_string(), "/dev/null".to_string(), 0),
            WorkProof::new("noop".to_string(), "noop_work".to_string(), *blake3::hash(format!("{}:{}", "noop".to_string(), "noop_work".to_string()).as_bytes()).as_bytes()),
        ))
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Traditional
    }
    fn is_required(&self) -> bool {
        false
    }
}

pub(crate) struct OptionalStateProof {
    _hypermesh_addr: SocketAddr,
}

impl OptionalStateProof {
    pub fn new(hypermesh_addr: SocketAddr) -> Self {
        Self {
            _hypermesh_addr: hypermesh_addr,
        }
    }
}

#[async_trait]
impl StateProofProvider for OptionalStateProof {
    async fn validate_proof(&self, _proof: &StateProof) -> Result<bool> {
        Ok(true)
    }

    async fn generate_proof(&self, _data: &[u8]) -> Result<StateProof> {
        use trustchain::proof_of_state::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        Ok(StateProof::new(
            StakeProof::new("optional".to_string(), "validator1".to_string()),
            TimeProof::new(Duration::from_secs(1)),
            SpaceProof::new("optional".to_string(), "/tmp/optional".to_string(), 1024),
            WorkProof::new("optional".to_string(), "optional_work".to_string(), *blake3::hash(format!("{}:{}", "optional".to_string(), "optional_work".to_string()).as_bytes()).as_bytes()),
        ))
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::Hybrid
    }
    fn is_required(&self) -> bool {
        false
    }
}

pub(crate) struct RequiredStateProof {
    _hypermesh_addr: SocketAddr,
}

impl RequiredStateProof {
    pub fn new(hypermesh_addr: SocketAddr) -> Self {
        Self {
            _hypermesh_addr: hypermesh_addr,
        }
    }
}

#[async_trait]
impl StateProofProvider for RequiredStateProof {
    async fn validate_proof(&self, proof: &StateProof) -> Result<bool> {
        // CANONICAL MODEL: PoStake is authorization (WHO) — require a bound
        // identity, never a stake amount.
        Ok(!proof.stake_proof.stake_holder_id.is_empty())
    }

    async fn generate_proof(&self, _data: &[u8]) -> Result<StateProof> {
        use trustchain::proof_of_state::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        Ok(StateProof::new(
            StakeProof::new("required".to_string(), "validator2".to_string()),
            TimeProof::new(Duration::from_secs(5)),
            SpaceProof::new("required".to_string(), "/tmp/required".to_string(), 10240),
            WorkProof::new("required".to_string(), "required_work".to_string(), *blake3::hash(format!("{}:{}", "required".to_string(), "required_work".to_string()).as_bytes()).as_bytes()),
        ))
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::PartialFederation
    }
    fn is_required(&self) -> bool {
        true
    }
}

pub(crate) struct FullStateProof {
    _hypermesh_addr: SocketAddr,
}

impl FullStateProof {
    pub fn new(hypermesh_addr: SocketAddr) -> Self {
        Self {
            _hypermesh_addr: hypermesh_addr,
        }
    }
}

#[async_trait]
impl StateProofProvider for FullStateProof {
    async fn validate_proof(&self, proof: &StateProof) -> Result<bool> {
        // CANONICAL MODEL: PoStake is authorization (WHO) — require a bound
        // identity, never a stake amount. PoSpace is WHERE — require a bound
        // location, never a capacity floor. PoWork is the HASH of work done.
        Ok(!proof.stake_proof.stake_holder_id.is_empty()
            && !(proof.space_proof.node_id.is_empty()
                && proof.space_proof.storage_path.is_empty())
            && proof.work_proof.work_hash != [0u8; 32])
    }

    async fn generate_proof(&self, _data: &[u8]) -> Result<StateProof> {
        use trustchain::proof_of_state::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        Ok(StateProof::new(
            StakeProof::new("full".to_string(), "validator4".to_string()),
            TimeProof::new(Duration::from_secs(10)),
            SpaceProof::new("full".to_string(), "/var/hypermesh".to_string(), 1048576),
            WorkProof::new("full".to_string(), "full_work".to_string(), *blake3::hash(format!("{}:{}", "full".to_string(), "full_work".to_string()).as_bytes()).as_bytes()),
        ))
    }

    fn phase(&self) -> BootstrapPhase {
        BootstrapPhase::FullFederation
    }
    fn is_required(&self) -> bool {
        true
    }
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Security Layer Interface - Team 3 Implementation Contract
// Production Cryptography and Asset Adapter Security

pub use blockmatrix::assets::core::AssetType;

use std::collections::HashMap;
use std::result::Result;
use std::time::{Duration, SystemTime};

use hypermesh_lib::PrivacyMode as PrivacyLevel;

/// Raw 32-byte asset hash. Distinct from hypermesh_lib::AssetId (string identifier).
pub type RawAssetId = [u8; 32];

/// Cryptographic key pair for assets
pub type AssetKeyPair = ([u8; 32], [u8; 64]); // (private_key, public_key)

/// Certificate chain for trust validation
pub type CertificateChain = Vec<Certificate>;

/// Secure communication channel (imported from network layer)
pub use super::network_layer::SecureChannel;

/// Certificate representation in TrustChain hierarchy
#[derive(Debug, Clone)]
pub struct Certificate {
    pub issuer: [u8; 32],
    pub subject: [u8; 32],
    pub public_key: [u8; 64],
    pub signature: [u8; 128],
    pub valid_from: SystemTime,
    pub valid_until: SystemTime,
    pub extensions: CertificateExtensions,
}

/// Certificate extensions for additional functionality
#[derive(Debug, Clone)]
pub struct CertificateExtensions {
    pub key_usage: KeyUsage,
    pub asset_permissions: Vec<AssetPermission>,
    pub privacy_level: PrivacyLevel,
    pub quantum_resistant: bool,
}

/// Key usage specification
#[derive(Debug, Clone)]
pub struct KeyUsage {
    pub digital_signature: bool,
    pub key_encipherment: bool,
    pub data_encipherment: bool,
    pub key_agreement: bool,
    pub certificate_signing: bool,
}

/// Asset-specific permissions
#[derive(Debug, Clone)]
pub struct AssetPermission {
    pub asset_type: AssetType,
    pub access_level: AccessLevel,
    pub resource_limits: ResourceLimits,
    pub time_constraints: Option<Duration>,
}

/// Access levels for asset permissions
#[derive(Debug, Clone)]
pub enum AccessLevel {
    ReadOnly,
    ReadWrite,
    Execute,
    FullControl,
    Administrative,
}

/// Resource limits for security enforcement
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_cores: Option<u32>,
    pub max_memory_gb: Option<u64>,
    pub max_storage_gb: Option<u64>,
    pub max_bandwidth_mbps: Option<u64>,
    pub max_execution_time: Option<Duration>,
}

/// Trust level classification for certificates
#[derive(Debug, Clone)]
pub enum TrustLevel {
    Untrusted,
    SelfSigned,
    LocallyTrusted,
    ChainValidated,
    FullyValidated,
    QuantumSecure,
}

/// Asset Adapter security configuration
#[derive(Debug, Clone)]
pub struct AssetAdapterSecurity {
    pub adapter_type: AssetType,
    pub encryption_algorithm: EncryptionAlgorithm,
    pub key_management: KeyManagement,
    pub access_control: AccessControl,
    pub audit_logging: AuditConfiguration,
}

/// Supported encryption algorithms (production implementations)
#[derive(Debug, Clone)]
pub enum EncryptionAlgorithm {
    FALCON1024,    // Post-quantum digital signatures
    Kyber1024,     // Post-quantum key encapsulation
    AES256GCM,     // Symmetric encryption
    ChaCha20Poly1305, // Alternative symmetric encryption
    X25519,        // Elliptic curve key exchange (legacy support)
}

/// Key management configuration
#[derive(Debug, Clone)]
pub struct KeyManagement {
    pub key_rotation_interval: Duration,
    pub key_escrow_enabled: bool,
    pub hardware_security_module: bool,
    pub key_derivation_function: KeyDerivationFunction,
    pub entropy_source: EntropySource,
}

/// Key derivation functions
#[derive(Debug, Clone)]
pub enum KeyDerivationFunction {
    Argon2id,
    PBKDF2,
    HKDF,
    Scrypt,
}

/// Entropy sources for key generation
#[derive(Debug, Clone)]
pub enum EntropySource {
    HardwareRNG,
    SystemEntropy,
    CombinedSources,
    QuantumRNG,
}

/// Access control configuration
#[derive(Debug, Clone)]
pub struct AccessControl {
    pub authentication_method: AuthenticationMethod,
    pub authorization_policy: AuthorizationPolicy,
    pub session_management: SessionManagement,
    pub privilege_escalation_protection: bool,
}

/// Authentication methods
#[derive(Debug, Clone)]
pub enum AuthenticationMethod {
    CertificateBased,
    MultiFactor,
    BiometricAuthentication,
    HardwareToken,
    ZeroKnowledgeProof,
}

/// Authorization policies
#[derive(Debug, Clone)]
pub enum AuthorizationPolicy {
    RoleBased,
    AttributeBased,
    CapabilityBased,
    ZeroTrust,
}

/// Session management configuration
#[derive(Debug, Clone)]
pub struct SessionManagement {
    pub session_timeout: Duration,
    pub concurrent_session_limit: u32,
    pub session_token_rotation: bool,
    pub secure_session_storage: bool,
}

/// Audit logging configuration
#[derive(Debug, Clone)]
pub struct AuditConfiguration {
    pub log_all_access: bool,
    pub log_data_changes: bool,
    pub log_security_events: bool,
    pub log_retention_period: Duration,
    pub log_encryption: bool,
}

/// Security validation result
#[derive(Debug)]
pub struct SecurityValidationResult {
    pub is_secure: bool,
    pub security_level: SecurityLevel,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub recommendations: Vec<SecurityRecommendation>,
    pub compliance_status: ComplianceStatus,
}

/// Security level classification
#[derive(Debug, Clone)]
pub enum SecurityLevel {
    Insecure,          // No security measures
    BasicProtection,   // Minimal security
    StandardSecurity,  // Industry standard
    HighSecurity,      // Enhanced protection
    QuantumSecure,     // Post-quantum security
    MilitaryGrade,     // Maximum security
}

/// Security vulnerabilities detected
#[derive(Debug)]
pub struct SecurityVulnerability {
    pub vulnerability_type: VulnerabilityType,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
    pub cve_reference: Option<String>,
}

/// Types of security vulnerabilities
#[derive(Debug)]
pub enum VulnerabilityType {
    WeakEncryption,
    KeyManagementFlaw,
    AccessControlBypass,
    AuthenticationWeakness,
    DataExposure,
    InjectionAttack,
    DenialOfService,
    PrivilegeEscalation,
}

/// Vulnerability severity levels
#[derive(Debug)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security recommendations
#[derive(Debug)]
pub struct SecurityRecommendation {
    pub recommendation_type: RecommendationType,
    pub priority: Priority,
    pub description: String,
    pub implementation_effort: ImplementationEffort,
}

/// Types of security recommendations
#[derive(Debug)]
pub enum RecommendationType {
    UpgradeEncryption,
    ImplementAccessControl,
    EnableAuditLogging,
    UpdateKeyManagement,
    AddAuthentication,
    ImplementMonitoring,
}

/// Priority levels for recommendations
#[derive(Debug)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
    Immediate,
}

/// Implementation effort estimation
#[derive(Debug)]
pub enum ImplementationEffort {
    Minimal,    // < 1 day
    Low,        // 1-3 days
    Medium,     // 1-2 weeks
    High,       // 2-4 weeks
    Extensive,  // > 1 month
}

/// Compliance status with security standards
#[derive(Debug)]
pub struct ComplianceStatus {
    pub standards_met: Vec<String>,
    pub standards_violated: Vec<String>,
    pub compliance_score: f32,
    pub certification_eligible: bool,
}

/// Errors that can occur in security operations
#[derive(Debug)]
pub enum SecurityError {
    EncryptionFailed,
    DecryptionFailed,
    KeyGenerationFailed,
    CertificateValidationFailed,
    AccessDenied,
    AuthenticationFailed,
    AuthorizationFailed,
    AuditLogFailed,
    SecurityPolicyViolation,
    QuantumResistanceRequired,
}

/// Main security layer interface - CRITICAL FOR TEAMS 1,2
pub trait SecurityLayer {
    /// Encrypt data for secure transport (Team 3 → Team 1)
    fn encrypt_transport(&self, data: &[u8], channel: &SecureChannel) -> Result<Vec<u8>, SecurityError>;
    
    /// Decrypt data from secure transport (Team 3 → Team 1)
    fn decrypt_transport(&self, encrypted_data: &[u8], channel: &SecureChannel) -> Result<Vec<u8>, SecurityError>;
    
    /// Validate certificate chain in TrustChain hierarchy (Team 3 core functionality)
    fn validate_certificates(&self, cert_chain: &CertificateChain) -> Result<TrustLevel, SecurityError>;
    
    /// Generate cryptographic keys for assets (Team 3 → Team 2)
    fn generate_asset_keys(&self, asset_id: RawAssetId) -> Result<AssetKeyPair, SecurityError>;
    
    /// Configure asset adapter security (Team 3 core responsibility)
    fn configure_asset_adapter_security(&self, asset_type: AssetType) -> Result<AssetAdapterSecurity, SecurityError>;
    
    /// Validate security compliance (Team 3 → All teams)
    fn validate_security_compliance(&self, component: &str) -> Result<SecurityValidationResult, SecurityError>;
    
    /// Implement privacy-aware access control (Team 3 → Team 2)
    fn enforce_privacy_access_control(&self, asset_id: RawAssetId, privacy_level: PrivacyLevel) -> Result<bool, SecurityError>;
}

/// Implementation requirements for Team 3
pub trait SecurityImplementationTeam: SecurityLayer {
    /// Replace XOR cipher simulations with production FALCON-1024 (CRITICAL REQUIREMENT)
    fn implement_falcon_1024(&mut self) -> Result<(), SecurityError>;
    
    /// Replace placeholder encryption with production Kyber (CRITICAL REQUIREMENT)
    fn implement_kyber_encryption(&mut self) -> Result<(), SecurityError>;
    
    /// Implement production key management system (SECURITY CRITICAL)
    fn implement_key_management(&mut self) -> Result<(), SecurityError>;
    
    /// Configure all asset adapters with proper security (ASSET SYSTEM CRITICAL)
    fn secure_all_asset_adapters(&mut self) -> Result<HashMap<AssetType, AssetAdapterSecurity>, SecurityError>;
    
    /// Implement TrustChain certificate hierarchy (TRUST SYSTEM CRITICAL)
    fn implement_trustchain_hierarchy(&mut self) -> Result<(), SecurityError>;
    
    /// Configure privacy-aware resource allocation (PRIVACY CRITICAL)
    fn configure_privacy_allocation(&mut self) -> Result<(), SecurityError>;
}

/// Asset Adapter implementations for Team 3
pub trait AssetAdapterSecurityImplementations {
    /// CPU Asset Adapter - PoWk validation, time-based scheduling (Team 3 → Team 2)
    fn secure_cpu_adapter(&self) -> Result<AssetAdapterSecurity, SecurityError>;
    
    /// GPU Asset Adapter - FALCON-1024 for GPU access control (Team 3 core)
    fn secure_gpu_adapter(&self) -> Result<AssetAdapterSecurity, SecurityError>;
    
    /// Memory Asset Adapter - NAT-like memory addressing with PoSp proofs (Team 3 → Team 1,2)
    fn secure_memory_adapter(&self) -> Result<AssetAdapterSecurity, SecurityError>;
    
    /// Storage Asset Adapter - Kyber encryption, content-aware segmentation (Team 3 core)
    fn secure_storage_adapter(&self) -> Result<AssetAdapterSecurity, SecurityError>;
    
    /// Network Asset Adapter - Secure transport integration (Team 3 → Team 1)
    fn secure_network_adapter(&self) -> Result<AssetAdapterSecurity, SecurityError>;
}

/// Critical security targets Team 3 must achieve
pub const MINIMUM_KEY_SIZE_BITS: u32 = 256;          // Minimum encryption key size
pub const QUANTUM_RESISTANCE_REQUIRED: bool = true;   // Must implement post-quantum crypto
pub const CERTIFICATE_VALIDATION_TIME_MS: u64 = 50;  // Maximum certificate validation time
pub const KEY_ROTATION_INTERVAL_HOURS: u64 = 24;     // Maximum key rotation interval

/// Interface validation for cross-team integration
pub trait SecurityIntegrationValidator {
    /// Validate Team 1 network security integration (Team 3 → Team 1)
    fn validate_network_security_integration(&self) -> Result<bool, SecurityError>;
    
    /// Validate Team 2 consensus security requirements (Team 3 → Team 2)
    fn validate_consensus_security_integration(&self) -> Result<bool, SecurityError>;
    
    /// Validate enterprise entity security requirements (ALL TEAMS)
    fn validate_enterprise_security_compliance(&self, entity_types: &[String]) -> Result<bool, SecurityError>;
    
    /// Validate quantum resistance across all components (FUTURE-PROOFING)
    fn validate_quantum_resistance_compliance(&self) -> Result<bool, SecurityError>;
    
    /// Validate HyperMesh asset security integration (CORE REQUIREMENT)
    fn validate_hypermesh_security_integration(&self) -> Result<bool, SecurityError>;
}
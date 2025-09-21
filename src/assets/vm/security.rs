//! VM Security Configuration and Management
//!
//! Handles security policies, isolation, encryption, and access control for VM execution.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// VM security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSecurityConfig {
    /// Isolation level
    pub isolation_level: IsolationLevel,

    /// Security policies
    pub security_policies: Vec<SecurityPolicy>,

    /// Access control
    pub access_control: AccessControl,

    /// Network policies
    pub network_policies: Vec<NetworkPolicy>,

    /// Encryption configuration
    pub encryption: Option<EncryptionConfig>,

    /// Audit logging
    pub audit_logging: bool,
}

impl Default for VmSecurityConfig {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            security_policies: Vec::new(),
            access_control: AccessControl::default(),
            network_policies: Vec::new(),
            encryption: None,
            audit_logging: false,
        }
    }
}

/// Isolation levels for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Standard container isolation
    Standard,
    /// Enhanced isolation with additional restrictions
    Enhanced,
    /// Maximum isolation with hardware-level separation
    Maximum,
    /// Custom isolation configuration
    Custom,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,

    /// Policy rules
    pub rules: Vec<String>,

    /// Enforcement level
    pub enforcement: EnforcementLevel,
}

/// Policy enforcement levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    Advisory,
    Warning,
    Enforced,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// Allowed users
    pub allowed_users: Vec<String>,

    /// Allowed groups
    pub allowed_groups: Vec<String>,

    /// Required capabilities
    pub required_capabilities: Vec<String>,

    /// Denied capabilities
    pub denied_capabilities: Vec<String>,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self {
            allowed_users: Vec::new(),
            allowed_groups: Vec::new(),
            required_capabilities: Vec::new(),
            denied_capabilities: Vec::new(),
        }
    }
}

/// Network security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Policy name
    pub name: String,

    /// Traffic direction
    pub direction: NetworkDirection,

    /// Allowed endpoints
    pub allowed_endpoints: Vec<String>,

    /// Denied endpoints
    pub denied_endpoints: Vec<String>,

    /// Protocol restrictions
    pub protocol_restrictions: HashMap<String, Vec<u16>>,
}

/// Network traffic direction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkDirection {
    Ingress,
    Egress,
    Both,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encrypt data at rest
    pub encrypt_at_rest: bool,

    /// Encrypt data in transit
    pub encrypt_in_transit: bool,

    /// Encryption algorithm
    pub algorithm: String,

    /// Key management
    pub key_management: KeyManagement,
}

/// Key management options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyManagement {
    /// System-managed keys
    System,
    /// User-provided keys
    UserProvided,
    /// Hardware security module
    Hsm,
    /// Key management service
    Kms,
}

/// Security validation result
#[derive(Debug, Clone)]
pub struct SecurityValidationResult {
    pub is_valid: bool,
    pub violations: Vec<SecurityViolation>,
    pub warnings: Vec<String>,
}

/// Security violation details
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    pub policy: String,
    pub rule: String,
    pub description: String,
    pub severity: ViolationSeverity,
}

/// Violation severity levels
#[derive(Debug, Clone)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security validator for VM configurations
pub struct SecurityValidator;

impl SecurityValidator {
    /// Validate security configuration
    pub fn validate(config: &VmSecurityConfig) -> SecurityValidationResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Check isolation level
        Self::validate_isolation(&config.isolation_level, &mut violations, &mut warnings);

        // Check security policies
        for policy in &config.security_policies {
            Self::validate_policy(policy, &mut violations, &mut warnings);
        }

        // Check access control
        Self::validate_access_control(&config.access_control, &mut violations, &mut warnings);

        // Check network policies
        for network_policy in &config.network_policies {
            Self::validate_network_policy(network_policy, &mut violations, &mut warnings);
        }

        // Check encryption
        if let Some(encryption) = &config.encryption {
            Self::validate_encryption(encryption, &mut violations, &mut warnings);
        }

        SecurityValidationResult {
            is_valid: violations.is_empty(),
            violations,
            warnings,
        }
    }

    fn validate_isolation(
        level: &IsolationLevel,
        violations: &mut Vec<SecurityViolation>,
        warnings: &mut Vec<String>,
    ) {
        match level {
            IsolationLevel::Standard => {
                warnings.push("Standard isolation may not be sufficient for sensitive workloads".to_string());
            }
            IsolationLevel::Custom => {
                warnings.push("Custom isolation requires manual security review".to_string());
            }
            _ => {}
        }
    }

    fn validate_policy(
        policy: &SecurityPolicy,
        _violations: &mut Vec<SecurityViolation>,
        warnings: &mut Vec<String>,
    ) {
        if policy.rules.is_empty() {
            warnings.push(format!("Security policy '{}' has no rules defined", policy.name));
        }
    }

    fn validate_access_control(
        access: &AccessControl,
        _violations: &mut Vec<SecurityViolation>,
        warnings: &mut Vec<String>,
    ) {
        if access.allowed_users.is_empty() && access.allowed_groups.is_empty() {
            warnings.push("No users or groups specified in access control".to_string());
        }
    }

    fn validate_network_policy(
        policy: &NetworkPolicy,
        _violations: &mut Vec<SecurityViolation>,
        warnings: &mut Vec<String>,
    ) {
        if policy.allowed_endpoints.is_empty() && policy.denied_endpoints.is_empty() {
            warnings.push(format!("Network policy '{}' has no endpoint rules", policy.name));
        }
    }

    fn validate_encryption(
        encryption: &EncryptionConfig,
        _violations: &mut Vec<SecurityViolation>,
        warnings: &mut Vec<String>,
    ) {
        if !encryption.encrypt_at_rest {
            warnings.push("Data at rest encryption is disabled".to_string());
        }
        if !encryption.encrypt_in_transit {
            warnings.push("Data in transit encryption is disabled".to_string());
        }
    }
}

/// Security context for VM execution
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub user_id: String,
    pub group_ids: Vec<String>,
    pub capabilities: Vec<String>,
    pub labels: HashMap<String, String>,
    pub selinux_context: Option<String>,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            group_ids: Vec::new(),
            capabilities: Vec::new(),
            labels: HashMap::new(),
            selinux_context: None,
        }
    }

    /// Check if context has required capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Check if context is in group
    pub fn is_in_group(&self, group: &str) -> bool {
        self.group_ids.iter().any(|g| g == group)
    }
}

/// Security audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEntry {
    pub timestamp: std::time::SystemTime,
    pub action: String,
    pub user_id: String,
    pub vm_id: String,
    pub result: String,
    pub details: HashMap<String, String>,
}

impl SecurityAuditEntry {
    /// Create a new audit entry
    pub fn new(action: String, user_id: String, vm_id: String, result: String) -> Self {
        Self {
            timestamp: std::time::SystemTime::now(),
            action,
            user_id,
            vm_id,
            result,
            details: HashMap::new(),
        }
    }

    /// Add detail to audit entry
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
    }
}
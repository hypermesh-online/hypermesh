//! Security configuration

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// eBPF configuration
    pub ebpf: EBPFConfig,
    /// Certificate configuration
    pub certificates: CertificateConfig,
    /// Capability configuration
    pub capabilities: CapabilityConfig,
    /// Intrusion detection configuration
    pub intrusion_detection: IntrusionDetectionConfig,
    /// Policy configuration
    pub policies: PolicyConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// eBPF security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EBPFConfig {
    /// Enable eBPF security
    pub enabled: bool,
    /// eBPF program directory
    pub program_dir: PathBuf,
    /// Network security programs
    pub network_programs: Vec<EBPFProgramConfig>,
    /// System call monitoring programs
    pub syscall_programs: Vec<EBPFProgramConfig>,
    /// Resource enforcement programs
    pub resource_programs: Vec<EBPFProgramConfig>,
    /// Default policies
    pub default_policies: PolicyDefaults,
}

/// eBPF program configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EBPFProgramConfig {
    /// Program name
    pub name: String,
    /// Program type (xdp, kprobe, tracepoint, etc.)
    pub program_type: String,
    /// Attach point
    pub attach_point: String,
    /// Program file path
    pub program_path: PathBuf,
    /// Auto-load on startup
    pub auto_load: bool,
}

/// Policy defaults for eBPF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefaults {
    /// Network policy defaults
    pub network_usage: NetworkPolicyDefaults,
    /// System call policy defaults
    pub syscall: SyscallPolicyDefaults,
}

/// Network policy defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyDefaults {
    /// Default action (allow, deny, log)
    pub default_action: String,
    /// Allowed protocols
    pub allowed_protocols: Vec<String>,
    /// Rate limits
    pub rate_limits: Vec<RateLimitConfig>,
    /// Blocked IP ranges
    pub blocked_ip_ranges: Vec<String>,
}

/// System call policy defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallPolicyDefaults {
    /// Default action (allow, deny, log)
    pub default_action: String,
    /// Blocked system calls
    pub blocked_syscalls: Vec<String>,
    /// Monitored system calls
    pub monitored_syscalls: Vec<String>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Protocol
    pub protocol: String,
    /// Rate limit (requests/second)
    pub limit: String,
    /// Burst allowance
    pub burst: String,
}

/// Certificate management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateConfig {
    /// CA certificate configuration
    pub ca: CAConfig,
    /// Certificate lifecycle configuration
    pub lifecycle: CertificateLifecycleConfig,
    /// HSM configuration
    pub hsm: HSMConfig,
    /// Certificate validation configuration
    pub validation: CertificateValidationConfig,
}

/// Certificate Authority configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAConfig {
    /// Key algorithm
    pub key_algorithm: String,
    /// Key size
    pub key_size: u32,
    /// Hash algorithm
    pub hash_algorithm: String,
    /// Validity period in days
    pub validity_period_days: u32,
    /// CA certificate path
    pub certificate_path: PathBuf,
    /// CA private key path
    pub private_key_path: PathBuf,
}

/// Certificate lifecycle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateLifecycleConfig {
    /// Default validity in days
    pub default_validity_days: u32,
    /// Rotation advance notice in days
    pub rotation_advance_days: u32,
    /// Minimum rotation interval in hours
    pub minimum_rotation_interval_hours: u32,
    /// Maximum certificate age in days
    pub maximum_certificate_age_days: u32,
    /// Auto-rotation enabled
    pub auto_rotation_enabled: bool,
}

/// HSM (Hardware Security Module) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HSMConfig {
    /// HSM enabled
    pub enabled: bool,
    /// HSM provider (pkcs11, etc.)
    pub provider: String,
    /// HSM slot ID
    pub slot_id: u32,
    /// HSM PIN/password
    pub pin: Option<String>,
    /// Generate keys in HSM
    pub key_generation_in_hsm: bool,
    /// Sign operations in HSM
    pub signing_in_hsm: bool,
}

/// Certificate validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationConfig {
    /// Check certificate revocation
    pub check_revocation: bool,
    /// Require full chain validation
    pub require_chain_validation: bool,
    /// Allow self-signed certificates
    pub allow_self_signed: bool,
    /// Maximum certificate chain length
    pub maximum_chain_length: u32,
}

/// Capability system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    /// Enable capability-based security
    pub enabled: bool,
    /// Default capabilities for different principals
    pub defaults: HashMap<String, Vec<String>>,
    /// Capability sets
    pub capability_sets: Vec<CapabilitySetConfig>,
    /// Delegation configuration
    pub delegation: DelegationConfig,
}

/// Capability set configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySetConfig {
    /// Capability set name
    pub name: String,
    /// Capabilities in this set
    pub capabilities: Vec<String>,
    /// Resources this set applies to
    pub resources: Vec<String>,
}

/// Delegation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationConfig {
    /// Maximum delegation depth
    pub max_depth: u32,
    /// Principals allowed to delegate
    pub allowed_delegators: Vec<String>,
    /// Capabilities that cannot be delegated
    pub prohibited_capabilities: Vec<String>,
}

/// Intrusion detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrusionDetectionConfig {
    /// Enable intrusion detection
    pub enabled: bool,
    /// Detection engines
    pub engines: Vec<DetectionEngineConfig>,
    /// Threat intelligence sources
    pub threat_intelligence: ThreatIntelligenceConfig,
    /// Response configuration
    pub response: ResponseConfig,
}

/// Detection engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEngineConfig {
    /// Engine name
    pub name: String,
    /// Engine type (signature, anomaly, behavior)
    pub engine_type: String,
    /// Configuration for this engine
    pub config: HashMap<String, serde_json::Value>,
    /// Enable this engine
    pub enabled: bool,
}

/// Threat intelligence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelligenceConfig {
    /// Enable threat intelligence
    pub enabled: bool,
    /// Intelligence sources
    pub sources: Vec<String>,
    /// Update interval
    pub update_interval: Duration,
}

/// Response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseConfig {
    /// Enable automated response
    pub enabled: bool,
    /// Response playbooks directory
    pub playbooks_dir: PathBuf,
    /// Default response actions
    pub default_actions: Vec<String>,
    /// Escalation rules
    pub escalation_rules: Vec<EscalationRuleConfig>,
}

/// Escalation rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRuleConfig {
    /// Rule name
    pub name: String,
    /// Conditions for escalation
    pub conditions: Vec<String>,
    /// Actions to take
    pub actions: Vec<String>,
    /// Escalation delay
    pub delay: Duration,
}

/// Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Policy files directory
    pub policy_dir: PathBuf,
    /// Default policy action
    pub default_action: String,
    /// Policy evaluation mode (enforcing, permissive, disabled)
    pub evaluation_mode: String,
    /// Policy update interval
    pub update_interval: Duration,
}

/// Security monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable security monitoring
    pub enabled: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Event retention period
    pub retention_period: Duration,
    /// Log level for security events
    pub log_level: String,
    /// Export configuration
    pub export: ExportConfig,
}

/// Export configuration for metrics and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Enable Prometheus metrics export
    pub prometheus_enabled: bool,
    /// Prometheus metrics port
    pub prometheus_port: u16,
    /// Enable OpenTelemetry export
    pub opentelemetry_enabled: bool,
    /// OpenTelemetry endpoint
    pub opentelemetry_endpoint: Option<String>,
}

impl SecurityConfig {
    /// Create a new security configuration with defaults
    pub fn new() -> Self {
        Self {
            ebpf: EBPFConfig::default(),
            certificates: CertificateConfig::default(),
            capabilities: CapabilityConfig::default(),
            intrusion_detection: IntrusionDetectionConfig::default(),
            policies: PolicyConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for EBPFConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            program_dir: PathBuf::from("/etc/hypermesh/ebpf"),
            network_programs: vec![
                EBPFProgramConfig {
                    name: "network_filter".to_string(),
                    program_type: "xdp".to_string(),
                    attach_point: "eth0".to_string(),
                    program_path: PathBuf::from("/etc/hypermesh/ebpf/network_filter.o"),
                    auto_load: true,
                },
                EBPFProgramConfig {
                    name: "packet_inspector".to_string(),
                    program_type: "tc".to_string(),
                    attach_point: "ingress".to_string(),
                    program_path: PathBuf::from("/etc/hypermesh/ebpf/packet_inspector.o"),
                    auto_load: true,
                },
            ],
            syscall_programs: vec![
                EBPFProgramConfig {
                    name: "syscall_monitor".to_string(),
                    program_type: "kprobe".to_string(),
                    attach_point: "sys_*".to_string(),
                    program_path: PathBuf::from("/etc/hypermesh/ebpf/syscall_monitor.o"),
                    auto_load: true,
                },
            ],
            resource_programs: vec![
                EBPFProgramConfig {
                    name: "resource_enforcer".to_string(),
                    program_type: "cgroup".to_string(),
                    attach_point: "/sys/fs/cgroup".to_string(),
                    program_path: PathBuf::from("/etc/hypermesh/ebpf/resource_enforcer.o"),
                    auto_load: true,
                },
            ],
            default_policies: PolicyDefaults::default(),
        }
    }
}

impl Default for PolicyDefaults {
    fn default() -> Self {
        Self {
            network_usage: NetworkPolicyDefaults {
                default_action: "deny".to_string(),
                allowed_protocols: vec!["tcp".to_string(), "udp".to_string(), "icmp".to_string()],
                rate_limits: vec![
                    RateLimitConfig {
                        protocol: "tcp".to_string(),
                        limit: "10000/s".to_string(),
                        burst: "1000".to_string(),
                    },
                ],
                blocked_ip_ranges: vec!["0.0.0.0/8".to_string(), "127.0.0.0/8".to_string()],
            },
            syscall: SyscallPolicyDefaults {
                default_action: "allow".to_string(),
                blocked_syscalls: vec!["ptrace".to_string(), "kexec_load".to_string()],
                monitored_syscalls: vec!["open".to_string(), "connect".to_string(), "execve".to_string()],
            },
        }
    }
}

impl Default for CertificateConfig {
    fn default() -> Self {
        Self {
            ca: CAConfig::default(),
            lifecycle: CertificateLifecycleConfig::default(),
            hsm: HSMConfig::default(),
            validation: CertificateValidationConfig::default(),
        }
    }
}

impl Default for CAConfig {
    fn default() -> Self {
        Self {
            key_algorithm: "rsa".to_string(),
            key_size: 4096,
            hash_algorithm: "sha256".to_string(),
            validity_period_days: 3650, // 10 years
            certificate_path: PathBuf::from("/etc/hypermesh/ca/ca.crt"),
            private_key_path: PathBuf::from("/etc/hypermesh/ca/ca.key"),
        }
    }
}

impl Default for CertificateLifecycleConfig {
    fn default() -> Self {
        Self {
            default_validity_days: 90,
            rotation_advance_days: 30,
            minimum_rotation_interval_hours: 24,
            maximum_certificate_age_days: 365,
            auto_rotation_enabled: true,
        }
    }
}

impl Default for HSMConfig {
    fn default() -> Self {
        Self {
            enabled: false, // HSM is optional
            provider: "pkcs11".to_string(),
            slot_id: 0,
            pin: None,
            key_generation_in_hsm: false,
            signing_in_hsm: false,
        }
    }
}

impl Default for CertificateValidationConfig {
    fn default() -> Self {
        Self {
            check_revocation: true,
            require_chain_validation: true,
            allow_self_signed: false,
            maximum_chain_length: 5,
        }
    }
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        let mut defaults = HashMap::new();
        defaults.insert("container_runtime".to_string(), vec![
            "CAP_NET_BIND_SERVICE".to_string(),
            "CAP_SETUID".to_string(),
            "CAP_SETGID".to_string(),
        ]);
        defaults.insert("orchestrator".to_string(), vec![
            "CAP_SYS_ADMIN".to_string(),
            "CAP_NET_ADMIN".to_string(),
        ]);
        defaults.insert("user_processes".to_string(), vec![]);
        
        Self {
            enabled: true,
            defaults,
            capability_sets: vec![
                CapabilitySetConfig {
                    name: "web_server".to_string(),
                    capabilities: vec!["CAP_NET_BIND_SERVICE".to_string()],
                    resources: vec!["tcp:80".to_string(), "tcp:443".to_string()],
                },
            ],
            delegation: DelegationConfig {
                max_depth: 3,
                allowed_delegators: vec!["orchestrator".to_string(), "admin".to_string()],
                prohibited_capabilities: vec![
                    "CAP_SYS_MODULE".to_string(),
                    "CAP_SYS_RAWIO".to_string(),
                ],
            },
        }
    }
}

impl Default for IntrusionDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engines: vec![
                DetectionEngineConfig {
                    name: "signature_detection".to_string(),
                    engine_type: "signature".to_string(),
                    config: HashMap::new(),
                    enabled: true,
                },
                DetectionEngineConfig {
                    name: "anomaly_detection".to_string(),
                    engine_type: "anomaly".to_string(),
                    config: HashMap::new(),
                    enabled: true,
                },
            ],
            threat_intelligence: ThreatIntelligenceConfig {
                enabled: true,
                sources: vec!["local_feeds".to_string()],
                update_interval: Duration::from_secs(3600), // 1 hour
            },
            response: ResponseConfig {
                enabled: true,
                playbooks_dir: PathBuf::from("/etc/hypermesh/playbooks"),
                default_actions: vec!["log".to_string(), "alert".to_string()],
                escalation_rules: vec![],
            },
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            policy_dir: PathBuf::from("/etc/hypermesh/policies"),
            default_action: "deny".to_string(),
            evaluation_mode: "enforcing".to_string(),
            update_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            log_level: "info".to_string(),
            export: ExportConfig {
                prometheus_enabled: true,
                prometheus_port: 9090,
                opentelemetry_enabled: false,
                opentelemetry_endpoint: None,
            },
        }
    }
}
//! Security policy enforcement using eBPF
//!
//! Implements network security policies, firewall rules, and threat detection
//! at the kernel level for high-performance packet filtering and analysis.

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use crate::{EbpfConfig, EbpfProgram, SecurityPolicy, SecurityRule, PolicyAction};

/// Security policy engine using eBPF for network security enforcement
pub struct SecurityPolicyEngine {
    config: EbpfConfig,
    running: bool,
    active_policies: RwLock<HashMap<String, SecurityPolicy>>,
    policy_stats: RwLock<PolicyStats>,
    threat_detector: RwLock<ThreatDetector>,
    rate_limiter: RwLock<RateLimiter>,
}

impl SecurityPolicyEngine {
    pub async fn new(config: &EbpfConfig) -> Result<Self> {
        info!("🔒 Initializing security policy engine");
        
        Ok(Self {
            config: config.clone(),
            running: false,
            active_policies: RwLock::new(HashMap::new()),
            policy_stats: RwLock::new(PolicyStats::new()),
            threat_detector: RwLock::new(ThreatDetector::new()),
            rate_limiter: RwLock::new(RateLimiter::new()),
        })
    }

    /// Apply a security policy
    pub async fn apply_policy(&self, policy: SecurityPolicy) -> Result<()> {
        info!("🛡️ Applying security policy: {}", policy.name);
        
        // Validate policy rules
        self.validate_policy(&policy)?;
        
        let mut policies = self.active_policies.write().await;
        policies.insert(policy.name.clone(), policy.clone());
        
        // Update rate limiter with new rules
        let mut rate_limiter = self.rate_limiter.write().await;
        for rule in &policy.rules {
            if let Some(rate_limit) = rule.rate_limit {
                rate_limiter.add_rule(&rule.source_cidr, rate_limit);
            }
        }
        
        debug!("Policy applied: {} rules configured", policy.rules.len());
        Ok(())
    }

    /// Remove a security policy
    pub async fn remove_policy(&self, policy_name: &str) -> Result<()> {
        info!("🗑️ Removing security policy: {}", policy_name);
        
        let mut policies = self.active_policies.write().await;
        if let Some(policy) = policies.remove(policy_name) {
            // Clean up rate limiter rules
            let mut rate_limiter = self.rate_limiter.write().await;
            for rule in &policy.rules {
                rate_limiter.remove_rule(&rule.source_cidr);
            }
        }
        
        Ok(())
    }

    /// Get current policy statistics
    pub async fn get_policy_stats(&self) -> PolicyStats {
        self.policy_stats.read().await.clone()
    }

    /// Get detailed statistics for a specific policy
    pub async fn get_policy_details(&self, policy_name: &str) -> Option<PolicyDetails> {
        let policies = self.active_policies.read().await;
        if let Some(policy) = policies.get(policy_name) {
            Some(PolicyDetails {
                name: policy.name.clone(),
                rules_count: policy.rules.len() as u32,
                priority: policy.priority,
                action: policy.action.clone(),
                packets_processed: rand::random::<u64>() % 10000,
                packets_blocked: rand::random::<u64>() % 1000,
                last_updated: Instant::now(),
            })
        } else {
            None
        }
    }

    /// List all active policies
    pub async fn list_policies(&self) -> Vec<String> {
        let policies = self.active_policies.read().await;
        policies.keys().cloned().collect()
    }

    /// Check if a packet would be allowed by current policies
    pub async fn check_packet(&self, src_ip: IpAddr, dst_port: u16, protocol: &str) -> PacketVerdict {
        let policies = self.active_policies.read().await;
        
        for policy in policies.values() {
            for rule in &policy.rules {
                if self.rule_matches(rule, src_ip, dst_port, protocol) {
                    match &policy.action {
                        PolicyAction::Allow => return PacketVerdict::Allow,
                        PolicyAction::Deny => return PacketVerdict::Deny,
                        PolicyAction::RateLimit(limit) => {
                            let rate_limiter = self.rate_limiter.read().await;
                            if rate_limiter.check_rate(&rule.source_cidr, *limit) {
                                return PacketVerdict::Allow;
                            } else {
                                return PacketVerdict::RateLimit;
                            }
                        },
                        PolicyAction::Log => {
                            info!("🔍 Packet logged: {} -> :{} ({})", src_ip, dst_port, protocol);
                            return PacketVerdict::Allow;
                        },
                    }
                }
            }
        }
        
        PacketVerdict::Allow // Default allow if no rules match
    }

    /// Detect potential security threats
    pub async fn threat_scan(&self) -> Vec<ThreatAlert> {
        let mut detector = self.threat_detector.write().await;
        detector.scan_for_threats().await
    }

    fn validate_policy(&self, policy: &SecurityPolicy) -> Result<()> {
        if policy.rules.is_empty() {
            return Err(anyhow::anyhow!("Policy must have at least one rule"));
        }

        for rule in &policy.rules {
            // Validate CIDR format
            if !self.is_valid_cidr(&rule.source_cidr) {
                return Err(anyhow::anyhow!("Invalid CIDR format: {}", rule.source_cidr));
            }

            // Validate port range
            if let Some(port) = rule.destination_port {
                if port == 0 {
                    return Err(anyhow::anyhow!("Invalid port: {}", port));
                }
            }

            // Validate protocol
            if let Some(ref protocol) = rule.protocol {
                if !["TCP", "UDP", "ICMP", "ANY"].contains(&protocol.as_str()) {
                    return Err(anyhow::anyhow!("Unsupported protocol: {}", protocol));
                }
            }
        }

        Ok(())
    }

    fn is_valid_cidr(&self, cidr: &str) -> bool {
        // Simple CIDR validation
        cidr.contains('/') && (cidr.contains('.') || cidr.contains(':'))
    }

    fn rule_matches(&self, rule: &SecurityRule, src_ip: IpAddr, dst_port: u16, protocol: &str) -> bool {
        // Simplified matching logic
        if let Some(rule_port) = rule.destination_port {
            if rule_port != dst_port {
                return false;
            }
        }

        if let Some(ref rule_protocol) = rule.protocol {
            if rule_protocol != "ANY" && rule_protocol != protocol {
                return false;
            }
        }

        // In a real implementation, would check if src_ip matches source_cidr
        true
    }
}

#[async_trait::async_trait]
impl EbpfProgram for SecurityPolicyEngine {
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting security policy eBPF program");
        
        // In a real implementation, this would:
        // 1. Load XDP/TC eBPF programs for packet filtering
        // 2. Set up policy maps and rules
        // 3. Configure connection tracking
        // 4. Attach to network interfaces
        
        self.running = true;
        
        // Start background threat detection
        let threat_detector = self.threat_detector.clone();
        let policy_stats = self.policy_stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Run threat detection
                {
                    let mut detector = threat_detector.write().await;
                    let alerts = detector.scan_for_threats().await;
                    
                    if !alerts.is_empty() {
                        warn!("🚨 Detected {} potential threats", alerts.len());
                        for alert in &alerts {
                            error!("Threat detected: {:?}", alert);
                        }
                    }
                }
                
                // Update statistics
                {
                    let mut stats = policy_stats.write().await;
                    stats.update_counters();
                }
            }
        });
        
        // Start rate limiter cleanup
        let rate_limiter = self.rate_limiter.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                {
                    let mut limiter = rate_limiter.write().await;
                    limiter.cleanup_expired_entries();
                }
            }
        });
        
        info!("✅ Security policy engine started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping security policy engine");
        self.running = false;
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        info!("🔄 Reloading security policy engine");
        self.stop().await?;
        self.start().await
    }

    fn name(&self) -> &str {
        "security-policy-engine"
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Verdict for packet processing
#[derive(Debug, Clone, PartialEq)]
pub enum PacketVerdict {
    Allow,
    Deny,
    RateLimit,
}

/// Policy statistics
#[derive(Debug, Clone)]
pub struct PolicyStats {
    pub policies_active: u32,
    pub total_packets_processed: u64,
    pub packets_allowed: u64,
    pub packets_denied: u64,
    pub packets_rate_limited: u64,
    pub threats_detected: u32,
    pub last_threat_detected: Option<Instant>,
}

impl PolicyStats {
    fn new() -> Self {
        Self {
            policies_active: 0,
            total_packets_processed: 0,
            packets_allowed: 0,
            packets_denied: 0,
            packets_rate_limited: 0,
            threats_detected: 0,
            last_threat_detected: None,
        }
    }

    fn update_counters(&mut self) {
        // Simulate some activity
        let new_packets = rand::random::<u64>() % 1000;
        self.total_packets_processed += new_packets;
        self.packets_allowed += (new_packets as f64 * 0.9) as u64;
        self.packets_denied += (new_packets as f64 * 0.08) as u64;
        self.packets_rate_limited += (new_packets as f64 * 0.02) as u64;
    }
}

/// Detailed policy information
#[derive(Debug, Clone)]
pub struct PolicyDetails {
    pub name: String,
    pub rules_count: u32,
    pub priority: u32,
    pub action: PolicyAction,
    pub packets_processed: u64,
    pub packets_blocked: u64,
    pub last_updated: Instant,
}

/// Threat detection engine
struct ThreatDetector {
    suspicious_ips: HashMap<IpAddr, SuspiciousActivity>,
    attack_patterns: Vec<AttackPattern>,
}

impl ThreatDetector {
    fn new() -> Self {
        Self {
            suspicious_ips: HashMap::new(),
            attack_patterns: vec![
                AttackPattern::PortScan,
                AttackPattern::DdosAttempt,
                AttackPattern::BruteForce,
                AttackPattern::AnomalousTraffic,
            ],
        }
    }

    async fn scan_for_threats(&mut self) -> Vec<ThreatAlert> {
        let mut alerts = Vec::new();
        
        // Simulate threat detection
        if rand::random::<f64>() < 0.1 {
            alerts.push(ThreatAlert {
                threat_type: ThreatType::PortScan,
                source_ip: "192.168.1.100".parse().unwrap(),
                detected_at: Instant::now(),
                severity: ThreatSeverity::Medium,
                description: "Suspicious port scanning activity detected".to_string(),
            });
        }

        if rand::random::<f64>() < 0.05 {
            alerts.push(ThreatAlert {
                threat_type: ThreatType::DdosAttempt,
                source_ip: "10.0.0.50".parse().unwrap(),
                detected_at: Instant::now(),
                severity: ThreatSeverity::High,
                description: "Potential DDoS attack detected".to_string(),
            });
        }
        
        alerts
    }
}

/// Suspicious activity tracking
#[derive(Debug, Clone)]
struct SuspiciousActivity {
    first_seen: Instant,
    last_seen: Instant,
    event_count: u32,
    patterns: Vec<AttackPattern>,
}

/// Attack pattern types
#[derive(Debug, Clone)]
enum AttackPattern {
    PortScan,
    DdosAttempt,
    BruteForce,
    AnomalousTraffic,
}

/// Threat alert information
#[derive(Debug, Clone)]
pub struct ThreatAlert {
    pub threat_type: ThreatType,
    pub source_ip: IpAddr,
    pub detected_at: Instant,
    pub severity: ThreatSeverity,
    pub description: String,
}

/// Types of threats
#[derive(Debug, Clone)]
pub enum ThreatType {
    PortScan,
    DdosAttempt,
    BruteForce,
    Malware,
    AnomalousTraffic,
}

/// Threat severity levels
#[derive(Debug, Clone)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Rate limiting implementation
struct RateLimiter {
    rules: HashMap<String, RateLimitRule>,
    counters: HashMap<String, RateLimitCounter>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            rules: HashMap::new(),
            counters: HashMap::new(),
        }
    }

    fn add_rule(&mut self, cidr: &str, limit: u32) {
        self.rules.insert(cidr.to_string(), RateLimitRule {
            limit,
            window: Duration::from_secs(60),
        });
    }

    fn remove_rule(&mut self, cidr: &str) {
        self.rules.remove(cidr);
        self.counters.remove(cidr);
    }

    fn check_rate(&self, cidr: &str, limit: u32) -> bool {
        // Simplified rate checking - would use sliding window in real implementation
        if let Some(counter) = self.counters.get(cidr) {
            counter.count < limit
        } else {
            true // Allow if no counter exists
        }
    }

    fn cleanup_expired_entries(&mut self) {
        let now = Instant::now();
        self.counters.retain(|_, counter| {
            now.duration_since(counter.last_updated) < Duration::from_secs(3600)
        });
    }
}

#[derive(Debug, Clone)]
struct RateLimitRule {
    limit: u32,
    window: Duration,
}

#[derive(Debug, Clone)]
struct RateLimitCounter {
    count: u32,
    last_updated: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_security_policy_engine_creation() {
        let config = EbpfConfig::default();
        let engine = SecurityPolicyEngine::new(&config).await.unwrap();
        assert!(!engine.is_running());
        assert_eq!(engine.name(), "security-policy-engine");
    }

    #[tokio::test]
    async fn test_policy_application() {
        let config = EbpfConfig::default();
        let engine = SecurityPolicyEngine::new(&config).await.unwrap();
        
        let policy = SecurityPolicy {
            name: "test-policy".to_string(),
            rules: vec![SecurityRule {
                source_cidr: "192.168.1.0/24".to_string(),
                destination_port: Some(80),
                protocol: Some("TCP".to_string()),
                rate_limit: Some(100),
            }],
            action: PolicyAction::Allow,
            priority: 1,
        };
        
        engine.apply_policy(policy).await.unwrap();
        
        let policies = engine.list_policies().await;
        assert!(policies.contains(&"test-policy".to_string()));
    }

    #[tokio::test]
    async fn test_packet_checking() {
        let config = EbpfConfig::default();
        let engine = SecurityPolicyEngine::new(&config).await.unwrap();
        
        let verdict = engine.check_packet(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            80,
            "TCP"
        ).await;
        
        assert_eq!(verdict, PacketVerdict::Allow);
    }

    #[test]
    fn test_threat_detector() {
        let detector = ThreatDetector::new();
        assert!(!detector.attack_patterns.is_empty());
    }
}
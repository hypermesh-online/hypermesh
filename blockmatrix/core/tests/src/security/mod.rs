//! Security testing and penetration testing framework
//! Tests system security, vulnerability scanning, and compliance validation

pub mod certificate_security;
pub mod network_security;
pub mod authentication_security;
pub mod authorization_security;
pub mod compliance_tests;

use crate::{TestResult, init_test_logging};
use tracing::{info, error};

/// Security test configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_penetration_tests: bool,
    pub enable_vulnerability_scanning: bool,
    pub enable_compliance_tests: bool,
    pub test_timeout_seconds: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_penetration_tests: true,
            enable_vulnerability_scanning: true,
            enable_compliance_tests: true,
            test_timeout_seconds: 300, // 5 minutes for security tests
        }
    }
}

/// Security test results
#[derive(Debug, Clone)]
pub struct SecurityTestResults {
    pub vulnerabilities_found: usize,
    pub critical_vulnerabilities: usize,
    pub high_vulnerabilities: usize,
    pub medium_vulnerabilities: usize,
    pub low_vulnerabilities: usize,
    pub compliance_score: f64, // 0.0-1.0
    pub authentication_tests_passed: usize,
    pub authorization_tests_passed: usize,
    pub certificate_tests_passed: usize,
}

impl SecurityTestResults {
    pub fn is_secure(&self) -> bool {
        self.critical_vulnerabilities == 0 && 
        self.high_vulnerabilities == 0 &&
        self.compliance_score >= 0.9
    }
    
    pub fn print_summary(&self) {
        println!("\n🔒 Security Test Results:");
        println!("=========================");
        println!("Total Vulnerabilities: {}", self.vulnerabilities_found);
        println!("  Critical: {}", self.critical_vulnerabilities);
        println!("  High: {}", self.high_vulnerabilities);
        println!("  Medium: {}", self.medium_vulnerabilities);
        println!("  Low: {}", self.low_vulnerabilities);
        println!("Compliance Score: {:.1}%", self.compliance_score * 100.0);
        println!("Authentication Tests Passed: {}", self.authentication_tests_passed);
        println!("Authorization Tests Passed: {}", self.authorization_tests_passed);
        println!("Certificate Tests Passed: {}", self.certificate_tests_passed);
        
        if self.is_secure() {
            println!("✅ Security Status: SECURE");
        } else {
            println!("⚠️  Security Status: VULNERABILITIES DETECTED");
        }
    }
}

/// Run all security tests
pub async fn run_all_security_tests() -> TestResult {
    init_test_logging();
    info!("🔒 Starting comprehensive security test suite");
    
    let config = SecurityConfig::default();
    let mut failed_tests = Vec::new();
    
    let test_suites = vec![
        ("certificate_security", certificate_security::run_certificate_security_tests),
        ("network_security", network_security::run_network_security_tests),
        ("authentication_security", authentication_security::run_authentication_security_tests),
        ("authorization_security", authorization_security::run_authorization_security_tests),
        ("compliance_tests", compliance_tests::run_compliance_tests),
    ];
    
    for (test_name, test_fn) in test_suites {
        info!("Running {} security tests", test_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} security tests passed", test_name);
            }
            Err(e) => {
                error!("❌ {} security tests failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }
    
    if failed_tests.is_empty() {
        info!("🎉 All security tests completed!");
        Ok(())
    } else {
        error!("Security tests failed: {}", failed_tests.join(", "));
        Err(format!("Security tests failed: {}", failed_tests.join(", ")).into())
    }
}

/// Vulnerability severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Security vulnerability report
#[derive(Debug, Clone)]
pub struct SecurityVulnerability {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: VulnerabilitySeverity,
    pub component: String,
    pub remediation: String,
}

impl SecurityVulnerability {
    pub fn new(
        id: String,
        title: String,
        description: String,
        severity: VulnerabilitySeverity,
        component: String,
        remediation: String,
    ) -> Self {
        Self {
            id,
            title,
            description,
            severity,
            component,
            remediation,
        }
    }
}

/// Security test runner that performs comprehensive security validation
pub struct SecurityTestRunner {
    config: SecurityConfig,
    vulnerabilities: Vec<SecurityVulnerability>,
}

impl SecurityTestRunner {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            vulnerabilities: Vec::new(),
        }
    }
    
    pub fn add_vulnerability(&mut self, vulnerability: SecurityVulnerability) {
        self.vulnerabilities.push(vulnerability);
    }
    
    pub fn generate_report(&self) -> SecurityTestResults {
        let mut critical_vulnerabilities = 0;
        let mut high_vulnerabilities = 0;
        let mut medium_vulnerabilities = 0;
        let mut low_vulnerabilities = 0;
        
        for vuln in &self.vulnerabilities {
            match vuln.severity {
                VulnerabilitySeverity::Critical => critical_vulnerabilities += 1,
                VulnerabilitySeverity::High => high_vulnerabilities += 1,
                VulnerabilitySeverity::Medium => medium_vulnerabilities += 1,
                VulnerabilitySeverity::Low => low_vulnerabilities += 1,
                VulnerabilitySeverity::Info => {},
            }
        }
        
        // Calculate compliance score based on vulnerabilities
        let compliance_score = if critical_vulnerabilities > 0 {
            0.0
        } else if high_vulnerabilities > 0 {
            0.5
        } else if medium_vulnerabilities > 3 {
            0.8
        } else {
            0.95
        };
        
        SecurityTestResults {
            vulnerabilities_found: self.vulnerabilities.len(),
            critical_vulnerabilities,
            high_vulnerabilities,
            medium_vulnerabilities,
            low_vulnerabilities,
            compliance_score,
            authentication_tests_passed: 0, // Will be filled by actual tests
            authorization_tests_passed: 0,  // Will be filled by actual tests
            certificate_tests_passed: 0,    // Will be filled by actual tests
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfig::default();
        assert!(config.enable_penetration_tests);
        assert!(config.enable_vulnerability_scanning);
        assert!(config.enable_compliance_tests);
        assert_eq!(config.test_timeout_seconds, 300);
    }
    
    #[test]
    fn test_vulnerability_creation() {
        let vuln = SecurityVulnerability::new(
            "CVE-2024-TEST".to_string(),
            "Test Vulnerability".to_string(),
            "Test description".to_string(),
            VulnerabilitySeverity::High,
            "test-component".to_string(),
            "Update to latest version".to_string(),
        );
        
        assert_eq!(vuln.id, "CVE-2024-TEST");
        assert_eq!(vuln.severity, VulnerabilitySeverity::High);
    }
    
    #[test]
    fn test_security_runner() {
        let config = SecurityConfig::default();
        let mut runner = SecurityTestRunner::new(config);
        
        let vuln = SecurityVulnerability::new(
            "TEST-001".to_string(),
            "Test Critical Vulnerability".to_string(),
            "Critical vulnerability for testing".to_string(),
            VulnerabilitySeverity::Critical,
            "nexus-core".to_string(),
            "Apply security patch".to_string(),
        );
        
        runner.add_vulnerability(vuln);
        
        let report = runner.generate_report();
        assert_eq!(report.vulnerabilities_found, 1);
        assert_eq!(report.critical_vulnerabilities, 1);
        assert!(!report.is_secure());
    }
}
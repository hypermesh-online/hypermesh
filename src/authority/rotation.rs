//! Certificate Rotation for TrustChain
//! 
//! Automated certificate rotation and key management

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Duration};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Certificate rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Certificate lifetime before rotation
    pub certificate_lifetime: Duration,
    
    /// Warning period before expiration
    pub warning_period: Duration,
    
    /// Grace period after expiration
    pub grace_period: Duration,
    
    /// Enable automatic rotation
    pub auto_rotate: bool,
    
    /// Overlap period for smooth transition
    pub overlap_period: Duration,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            certificate_lifetime: Duration::from_secs(365 * 24 * 60 * 60), // 1 year
            warning_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            grace_period: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            auto_rotate: true,
            overlap_period: Duration::from_secs(24 * 60 * 60), // 1 day
        }
    }
}

/// Certificate rotation status
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RotationStatus {
    Active,
    Warning,
    Critical,
    Expired,
    Rotating,
    Failed,
}

/// Certificate rotation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRotationInfo {
    /// Current rotation status
    pub status: RotationStatus,
    
    /// Certificate creation time
    pub created_at: SystemTime,
    
    /// Certificate expiration time
    pub expires_at: SystemTime,
    
    /// Next rotation scheduled time
    pub next_rotation: Option<SystemTime>,
    
    /// Last rotation attempt
    pub last_rotation_attempt: Option<SystemTime>,
    
    /// Rotation attempt count
    pub rotation_attempts: u32,
    
    /// Previous certificate ID (if rotated)
    pub previous_certificate_id: Option<String>,
}

impl CertificateRotationInfo {
    /// Create new rotation info
    pub fn new(certificate_lifetime: Duration) -> Self {
        let now = SystemTime::now();
        let expires_at = now + certificate_lifetime;
        
        Self {
            status: RotationStatus::Active,
            created_at: now,
            expires_at,
            next_rotation: None,
            last_rotation_attempt: None,
            rotation_attempts: 0,
            previous_certificate_id: None,
        }
    }
    
    /// Update rotation status based on current time
    pub fn update_status(&mut self, config: &RotationConfig) {
        let now = SystemTime::now();
        
        if now > self.expires_at + config.grace_period {
            self.status = RotationStatus::Expired;
        } else if now > self.expires_at {
            self.status = RotationStatus::Critical;
        } else if now > self.expires_at - config.warning_period {
            self.status = RotationStatus::Warning;
        } else {
            self.status = RotationStatus::Active;
        }
    }
    
    /// Check if certificate needs rotation
    pub fn needs_rotation(&self, config: &RotationConfig) -> bool {
        if !config.auto_rotate {
            return false;
        }
        
        let now = SystemTime::now();
        let rotation_threshold = self.expires_at - config.warning_period;
        
        now >= rotation_threshold && self.status != RotationStatus::Rotating
    }
    
    /// Mark rotation as starting
    pub fn start_rotation(&mut self) {
        self.status = RotationStatus::Rotating;
        self.last_rotation_attempt = Some(SystemTime::now());
        self.rotation_attempts += 1;
    }
    
    /// Mark rotation as completed
    pub fn complete_rotation(&mut self, new_certificate_id: String, config: &RotationConfig) {
        self.status = RotationStatus::Active;
        self.previous_certificate_id = Some(new_certificate_id);
        self.created_at = SystemTime::now();
        self.expires_at = self.created_at + config.certificate_lifetime;
        self.next_rotation = Some(self.expires_at - config.warning_period);
    }
    
    /// Mark rotation as failed
    pub fn fail_rotation(&mut self, error: &str) {
        self.status = RotationStatus::Failed;
        warn!("❌ Certificate rotation failed: {}", error);
    }
}

/// Certificate rotation manager
pub struct CertificateRotationManager {
    /// Rotation configuration
    config: RotationConfig,
    
    /// Certificate rotation info by certificate ID
    rotation_info: Arc<RwLock<std::collections::HashMap<String, CertificateRotationInfo>>>,
}

impl CertificateRotationManager {
    /// Create new rotation manager
    pub fn new(config: RotationConfig) -> Self {
        Self {
            config,
            rotation_info: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Register certificate for rotation tracking
    pub async fn register_certificate(&self, certificate_id: String) {
        let rotation_info = CertificateRotationInfo::new(self.config.certificate_lifetime);
        self.rotation_info.write().await.insert(certificate_id.clone(), rotation_info);
        
        info!("📝 Registered certificate for rotation tracking: {}", certificate_id);
    }
    
    /// Update rotation status for all certificates
    pub async fn update_all_statuses(&self) {
        let mut rotation_info_guard = self.rotation_info.write().await;
        for (cert_id, rotation_info) in rotation_info_guard.iter_mut() {
            let old_status = rotation_info.status.clone();
            rotation_info.update_status(&self.config);
            
            if rotation_info.status != old_status {
                info!("🔄 Certificate {} status changed: {:?} -> {:?}", 
                      cert_id, old_status, rotation_info.status);
            }
        }
    }
    
    /// Get certificates that need rotation
    pub async fn get_certificates_needing_rotation(&self) -> Vec<String> {
        let rotation_info_guard = self.rotation_info.read().await;
        rotation_info_guard.iter()
            .filter(|(_, info)| info.needs_rotation(&self.config))
            .map(|(cert_id, _)| cert_id.clone())
            .collect()
    }
    
    /// Get rotation info for certificate
    pub async fn get_rotation_info(&self, certificate_id: &str) -> Option<CertificateRotationInfo> {
        let rotation_info_guard = self.rotation_info.read().await;
        rotation_info_guard.get(certificate_id).cloned()
    }
    
    /// Get rotation info (mutable) for certificate
    pub async fn get_rotation_info_mut(&self, certificate_id: &str) -> Option<CertificateRotationInfo> {
        let rotation_info_guard = self.rotation_info.read().await;
        rotation_info_guard.get(certificate_id).cloned()
    }
    
    /// Start rotation for certificate
    pub async fn start_rotation(&self, certificate_id: &str) -> Result<()> {
        let mut rotation_info_guard = self.rotation_info.write().await;
        if let Some(rotation_info) = rotation_info_guard.get_mut(certificate_id) {
            rotation_info.start_rotation();
            info!("🔄 Started rotation for certificate: {}", certificate_id);
        } else {
            warn!("⚠️ Certificate not found for rotation: {}", certificate_id);
        }
        Ok(())
    }
    
    /// Complete rotation for certificate
    pub async fn complete_rotation(&self, old_certificate_id: &str, new_certificate_id: String) -> Result<()> {
        let mut rotation_info_guard = self.rotation_info.write().await;
        if let Some(rotation_info) = rotation_info_guard.get_mut(old_certificate_id) {
            rotation_info.complete_rotation(new_certificate_id.clone(), &self.config);
            info!("✅ Completed rotation for certificate: {} -> {}", old_certificate_id, new_certificate_id);
        } else {
            warn!("⚠️ Certificate not found for rotation completion: {}", old_certificate_id);
        }
        Ok(())
    }
    
    /// Fail rotation for certificate
    pub async fn fail_rotation(&self, certificate_id: &str, error: &str) -> Result<()> {
        let mut rotation_info_guard = self.rotation_info.write().await;
        if let Some(rotation_info) = rotation_info_guard.get_mut(certificate_id) {
            rotation_info.fail_rotation(error);
            warn!("❌ Failed rotation for certificate: {} - {}", certificate_id, error);
        } else {
            warn!("⚠️ Certificate not found for rotation failure: {}", certificate_id);
        }
        Ok(())
    }
    
    /// Get rotation statistics
    pub async fn get_statistics(&self) -> RotationStatistics {
        let rotation_info_guard = self.rotation_info.read().await;
        let mut status_counts = std::collections::HashMap::new();
        let mut needs_rotation = 0;

        for (_, rotation_info) in rotation_info_guard.iter() {
            // Count status
            *status_counts.entry(rotation_info.status.clone()).or_insert(0) += 1;

            // Check if needs rotation
            if rotation_info.needs_rotation(&self.config) {
                needs_rotation += 1;
            }
        }

        RotationStatistics {
            total_certificates: rotation_info_guard.len(),
            status_counts,
            needs_rotation,
            auto_rotation_enabled: self.config.auto_rotate,
        }
    }
}

/// Rotation statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct RotationStatistics {
    pub total_certificates: usize,
    pub status_counts: std::collections::HashMap<RotationStatus, usize>,
    pub needs_rotation: usize,
    pub auto_rotation_enabled: bool,
}
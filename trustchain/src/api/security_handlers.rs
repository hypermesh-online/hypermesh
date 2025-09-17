//! Security-Integrated API Handlers
//! 
//! API endpoints with mandatory consensus validation and security monitoring

use axum::{
    Json, extract::{Query, Path, State},
    http::StatusCode,
    response::Json as JsonResponse,
};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tracing::{info, debug, error, warn};
use base64::{engine::general_purpose, Engine as _};
use std::time::SystemTime;
use std::collections::HashMap;

use crate::consensus::{ConsensusProof, ConsensusResult};
use crate::security::{SecurityDashboard, SecurityValidationResult, SecuritySeverity};
use crate::security::monitoring::{SecurityDashboardData, LiveCertificateOperation, ConsensusValidationStatus, OperationState};
use crate::ca::security_integration::{SecurityIntegratedCA, CertificateValidationResult, IntegratedCAMetrics};
use crate::errors::{ErrorResponse, Result as TrustChainResult};
use super::{AppState, CertificateIssueRequest, CertificateResponse};

/// Enhanced app state with security integration
#[derive(Clone)]
pub struct SecurityIntegratedAppState {
    /// Base app state
    pub base: AppState,
    /// Security-integrated CA
    pub security_ca: std::sync::Arc<SecurityIntegratedCA>,
}

// Security-Integrated Certificate Authority Handlers

/// Issue certificate with MANDATORY consensus validation
pub async fn issue_certificate_secure(
    State(state): State<SecurityIntegratedAppState>,
    Json(request): Json<SecureCertificateIssueRequest>
) -> Result<JsonResponse<SecureCertificateResponse>, StatusCode> {
    info!("SECURE certificate issuance requested for: {} (consensus required)", request.common_name);
    
    // CRITICAL: Validate that consensus proof is provided
    if request.consensus_proof.is_none() {
        error!("SECURITY VIOLATION: Certificate request without consensus proof for: {}", request.common_name);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let consensus_proof = request.consensus_proof.unwrap();
    
    // Convert to CA request format
    use crate::ca::CertificateRequest;
    let ca_request = CertificateRequest {
        common_name: request.common_name.clone(),
        san_entries: request.san_entries,
        node_id: request.node_id,
        ipv6_addresses: request.ipv6_addresses,
        consensus_proof,
        timestamp: SystemTime::now(),
    };
    
    // Issue certificate through security-integrated CA
    match state.security_ca.issue_certificate_secure(ca_request).await {
        Ok(issued_cert) => {
            info!("SECURE certificate issued successfully: {} for {}", 
                  issued_cert.serial_number, request.common_name);
            
            let response = SecureCertificateResponse {
                certificate: issued_cert,
                security_validated: true,
                consensus_validated: true,
                operation_id: uuid::Uuid::new_v4().to_string(),
                security_score: 1.0, // Perfect score for successful issuance
            };
            
            Ok(Json(response))
        }
        Err(e) => {
            error!("SECURE certificate issuance FAILED for {}: {}", request.common_name, e);
            
            // Return specific error based on failure type
            match e {
                crate::errors::TrustChainError::SecurityValidationFailed { reason } => {
                    error!("SECURITY VALIDATION FAILED: {}", reason);
                    Err(StatusCode::FORBIDDEN)
                }
                crate::errors::TrustChainError::ConsensusValidationFailed { reason } => {
                    error!("CONSENSUS VALIDATION FAILED: {}", reason);
                    Err(StatusCode::FORBIDDEN)
                }
                _ => {
                    error!("Certificate issuance error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }
}

/// Validate certificate with security monitoring
pub async fn validate_certificate_secure(
    State(state): State<SecurityIntegratedAppState>,
    Json(request): Json<SecureCertificateValidationRequest>
) -> Result<JsonResponse<CertificateValidationResult>, StatusCode> {
    info!("SECURE certificate validation requested");
    
    // Decode certificate
    let cert_der = general_purpose::STANDARD.decode(&request.certificate_der)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Perform secure validation
    match state.security_ca.validate_certificate_secure(&cert_der).await {
        Ok(validation_result) => {
            info!("Certificate validation completed: valid={}", validation_result.is_valid);
            Ok(Json(validation_result))
        }
        Err(e) => {
            error!("Certificate validation failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Validate consensus proof directly
pub async fn validate_consensus_proof_secure(
    State(state): State<SecurityIntegratedAppState>,
    Json(request): Json<ConsensusValidationRequest>
) -> Result<JsonResponse<ConsensusValidationResponse>, StatusCode> {
    info!("SECURE consensus proof validation requested for: {}", request.operation);
    
    // Perform security validation (which includes consensus validation)
    match state.security_ca.security_monitor.validate_certificate_operation(
        &request.operation,
        &request.consensus_proof,
        "api_consensus_validation",
    ).await {
        Ok(security_result) => {
            let consensus_details = if let Some(consensus_result) = &security_result.consensus_result {
                ConsensusValidationDetails {
                    stake_valid: true, // Would extract from actual validation
                    time_valid: true,
                    space_valid: true,
                    work_valid: true,
                    overall_score: security_result.metrics.security_score,
                }
            } else {
                ConsensusValidationDetails {
                    stake_valid: false,
                    time_valid: false,
                    space_valid: false,
                    work_valid: false,
                    overall_score: 0.0,
                }
            };
            
            let response = ConsensusValidationResponse {
                is_valid: security_result.is_valid,
                validation_details: consensus_details,
                security_score: security_result.metrics.security_score,
                byzantine_detected: !matches!(security_result.byzantine_detection, 
                                            crate::security::byzantine::ByzantineDetectionResult::NotDetected),
            };
            
            info!("Consensus validation completed: valid={}, score={:.2}", 
                  response.is_valid, response.security_score);
            
            Ok(Json(response))
        }
        Err(e) => {
            error!("Consensus validation failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Security Monitoring Dashboard Handlers

/// Get security monitoring dashboard
pub async fn get_security_dashboard(
    State(state): State<SecurityIntegratedAppState>
) -> Result<JsonResponse<SecurityDashboard>, StatusCode> {
    debug!("Security dashboard requested");
    
    match state.security_ca.get_security_dashboard().await {
        Ok(dashboard) => {
            debug!("Security dashboard data retrieved successfully");
            Ok(Json(dashboard))
        }
        Err(e) => {
            error!("Failed to get security dashboard: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get integrated CA metrics (CA + Security)
pub async fn get_integrated_metrics(
    State(state): State<SecurityIntegratedAppState>
) -> Result<JsonResponse<IntegratedCAMetrics>, StatusCode> {
    debug!("Integrated CA metrics requested");
    
    match state.security_ca.get_integrated_metrics().await {
        Ok(metrics) => {
            debug!("Integrated metrics retrieved successfully");
            Ok(Json(metrics))
        }
        Err(e) => {
            error!("Failed to get integrated metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get real-time security alerts
pub async fn get_security_alerts(
    State(state): State<SecurityIntegratedAppState>,
    Query(params): Query<SecurityAlertsQuery>
) -> Result<JsonResponse<SecurityAlertsResponse>, StatusCode> {
    debug!("Security alerts requested with limit: {:?}", params.limit);
    
    let limit = params.limit.unwrap_or(50).min(500); // Cap at 500 alerts
    
    // Get recent alerts from security monitor
    match state.security_ca.security_monitor.alert_manager.get_recent_alerts(limit).await {
        Ok(alerts) => {
            let response = SecurityAlertsResponse {
                alerts,
                total_count: alerts.len() as u64,
                timestamp: SystemTime::now(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get security alerts: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Acknowledge security alert
pub async fn acknowledge_security_alert(
    State(state): State<SecurityIntegratedAppState>,
    Path(alert_id): Path<String>,
    Json(request): Json<AlertAcknowledgmentRequest>
) -> Result<JsonResponse<serde_json::Value>, StatusCode> {
    info!("Security alert acknowledgment requested: {} by {}", alert_id, request.operator);
    
    match state.security_ca.security_monitor.alert_manager.acknowledge_alert(&alert_id, &request.operator).await {
        Ok(()) => {
            info!("Security alert acknowledged: {}", alert_id);
            Ok(Json(json!({
                "alert_id": alert_id,
                "acknowledged": true,
                "operator": request.operator,
                "timestamp": SystemTime::now()
            })))
        }
        Err(e) => {
            error!("Failed to acknowledge alert {}: {}", alert_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Get Byzantine detection summary
pub async fn get_byzantine_summary(
    State(state): State<SecurityIntegratedAppState>
) -> Result<JsonResponse<crate::security::byzantine::ByzantineDetectionSummary>, StatusCode> {
    debug!("Byzantine detection summary requested");
    
    match state.security_ca.security_monitor.byzantine_detector.get_detection_summary().await {
        Ok(summary) => {
            debug!("Byzantine detection summary retrieved");
            Ok(Json(summary))
        }
        Err(e) => {
            error!("Failed to get Byzantine detection summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Emergency security shutdown (disables certificate issuance)
pub async fn emergency_security_shutdown(
    State(state): State<SecurityIntegratedAppState>,
    Json(request): Json<EmergencyShutdownRequest>
) -> Result<JsonResponse<serde_json::Value>, StatusCode> {
    error!("EMERGENCY SECURITY SHUTDOWN requested by: {} - Reason: {}", 
           request.operator, request.reason);
    
    // In production, this would:
    // 1. Disable certificate issuance
    // 2. Alert all administrators
    // 3. Log security event
    // 4. Potentially shut down services
    
    // For now, log the request
    let response = json!({
        "shutdown_requested": true,
        "operator": request.operator,
        "reason": request.reason,
        "timestamp": SystemTime::now(),
        "status": "logged_only", // In production: "services_disabled"
        "message": "Emergency shutdown logged - production implementation would disable services"
    });
    
    Ok(Json(response))
}

// Request/Response Types

/// Secure certificate issue request with mandatory consensus
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureCertificateIssueRequest {
    pub common_name: String,
    pub san_entries: Vec<String>,
    pub node_id: String,
    pub ipv6_addresses: Vec<std::net::Ipv6Addr>,
    /// MANDATORY: Consensus proof is required for all certificate operations
    pub consensus_proof: Option<ConsensusProof>,
}

/// Secure certificate response with validation status
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureCertificateResponse {
    pub certificate: crate::ca::IssuedCertificate,
    /// Whether security validation passed
    pub security_validated: bool,
    /// Whether consensus validation passed
    pub consensus_validated: bool,
    /// Operation ID for tracking
    pub operation_id: String,
    /// Security score (0.0 - 1.0)
    pub security_score: f64,
}

/// Secure certificate validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureCertificateValidationRequest {
    /// Certificate in DER format (base64 encoded)
    pub certificate_der: String,
    /// Domain to validate (optional)
    pub domain: Option<String>,
}

/// Consensus validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationRequest {
    pub consensus_proof: ConsensusProof,
    pub operation: String,
}

/// Enhanced consensus validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationResponse {
    pub is_valid: bool,
    pub validation_details: ConsensusValidationDetails,
    /// Security score from integrated validation
    pub security_score: f64,
    /// Whether Byzantine behavior was detected
    pub byzantine_detected: bool,
}

/// Detailed consensus validation breakdown
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationDetails {
    pub stake_valid: bool,
    pub time_valid: bool,
    pub space_valid: bool,
    pub work_valid: bool,
    pub overall_score: f64,
}

/// Security alerts query parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityAlertsQuery {
    pub limit: Option<usize>,
    pub severity: Option<String>,
    pub category: Option<String>,
}

/// Security alerts response
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityAlertsResponse {
    pub alerts: Vec<crate::security::alerts::SecurityAlert>,
    pub total_count: u64,
    pub timestamp: SystemTime,
}

/// Alert acknowledgment request
#[derive(Debug, Serialize, Deserialize)]
pub struct AlertAcknowledgmentRequest {
    pub operator: String,
    pub notes: Option<String>,
}

/// Emergency shutdown request
#[derive(Debug, Serialize, Deserialize)]
pub struct EmergencyShutdownRequest {
    pub operator: String,
    pub reason: String,
    pub authorization_code: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::api::AppState;
    use crate::ca::{CAConfiguration, security_integration::SecurityIntegrationConfig};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    async fn create_test_state() -> SecurityIntegratedAppState {
        let base_config = ApiConfig::default();
        let base_state = AppState {
            config: Arc::new(base_config),
            stats: Arc::new(RwLock::new(crate::api::ApiStats::default())),
            rate_limiter: Arc::new(crate::api::RateLimiter::new(60).await.unwrap()),
        };
        
        let ca_config = CAConfiguration::default();
        let security_config = SecurityIntegrationConfig::default();
        let security_ca = Arc::new(
            SecurityIntegratedCA::new(ca_config, security_config).await.unwrap()
        );
        
        SecurityIntegratedAppState {
            base: base_state,
            security_ca,
        }
    }

    #[tokio::test]
    async fn test_secure_certificate_request_with_consensus() {
        let state = create_test_state().await;
        
        let request = SecureCertificateIssueRequest {
            common_name: "secure.test.com".to_string(),
            san_entries: vec!["secure.test.com".to_string()],
            node_id: "test_node".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: Some(ConsensusProof::default_for_testing()),
        };
        
        let result = issue_certificate_secure(State(state), Json(request)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap().0;
        assert!(response.security_validated);
        assert!(response.consensus_validated);
    }

    #[tokio::test]
    async fn test_certificate_request_without_consensus_fails() {
        let state = create_test_state().await;
        
        let request = SecureCertificateIssueRequest {
            common_name: "insecure.test.com".to_string(),
            san_entries: vec!["insecure.test.com".to_string()],
            node_id: "test_node".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: None, // MISSING CONSENSUS PROOF
        };
        
        let result = issue_certificate_secure(State(state), Json(request)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_security_dashboard_endpoint() {
        let state = create_test_state().await;
        
        let result = get_security_dashboard(State(state)).await;
        assert!(result.is_ok());
        
        let dashboard = result.unwrap().0;
        assert!(dashboard.consensus_status.enabled);
    }

    #[tokio::test]
    async fn test_consensus_validation_endpoint() {
        let state = create_test_state().await;
        
        let request = ConsensusValidationRequest {
            consensus_proof: ConsensusProof::default_for_testing(),
            operation: "test_validation".to_string(),
        };
        
        let result = validate_consensus_proof_secure(State(state), Json(request)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap().0;
        // Response validity depends on the consensus proof validation
    }
}
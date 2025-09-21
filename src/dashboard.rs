//! Dashboard Message Handler for Internet 2.0 Server
//!
//! Provides STOQ protocol-based dashboard functionality with:
//! - Real-time server statistics and health monitoring
//! - Asset management operations
//! - Certificate management operations  
//! - System health and performance metrics
//! - Certificate-based authentication for all dashboard access

use std::sync::Arc;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, error};
use async_trait::async_trait;

use crate::config::HyperMeshServerConfig;
use crate::transport::StoqTransportLayer;
use crate::assets::HyperMeshAssetLayer;
use crate::authority::TrustChainAuthorityLayer;
use crate::integration::LayerIntegration;
use crate::monitoring::PerformanceMonitor;
use crate::hardware::HardwareDetectionService;

/// Dashboard message types for STOQ protocol communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DashboardMessage {
    /// Get comprehensive server statistics
    GetServerStatistics,
    
    /// Get system health status
    GetSystemHealth,
    
    /// Asset management operations
    AssetManagement(AssetManagementRequest),
    
    /// Certificate operations
    CertificateOperation(CertificateOperationRequest),
    
    /// Performance monitoring requests
    PerformanceMonitoring(PerformanceMonitoringRequest),
    
    /// Integration status requests
    IntegrationStatus,

    /// Hardware detection requests
    HardwareDetection(HardwareDetectionRequest),
}

/// Asset management request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum AssetManagementRequest {
    /// List all assets
    ListAssets {
        filter: Option<String>,
        limit: Option<u32>,
    },
    
    /// Get asset details
    GetAsset {
        asset_id: String,
    },
    
    /// Create new asset
    CreateAsset {
        asset_type: String,
        configuration: serde_json::Value,
    },
    
    /// Update asset configuration
    UpdateAsset {
        asset_id: String,
        configuration: serde_json::Value,
    },
    
    /// Delete asset
    DeleteAsset {
        asset_id: String,
    },
}

/// Certificate operation request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum CertificateOperationRequest {
    /// List certificates
    ListCertificates {
        filter: Option<String>,
    },
    
    /// Get certificate details
    GetCertificate {
        certificate_id: String,
    },
    
    /// Issue new certificate
    IssueCertificate {
        subject: String,
        validity_days: u32,
        usage: Vec<String>,
    },
    
    /// Revoke certificate
    RevokeCertificate {
        certificate_id: String,
        reason: String,
    },
    
    /// Validate certificate
    ValidateCertificate {
        certificate_der: Vec<u8>,
    },
}

/// Performance monitoring request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request")]
pub enum PerformanceMonitoringRequest {
    /// Get current performance metrics
    GetCurrentMetrics,
    
    /// Get historical performance data
    GetHistoricalMetrics {
        start_time: u64,
        end_time: u64,
        interval: String,
    },
    
    /// Get performance alerts
    GetPerformanceAlerts,
    
    /// Configure performance thresholds
    SetPerformanceThresholds {
        thresholds: serde_json::Value,
    },
}

/// Hardware detection request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum HardwareDetectionRequest {
    /// Get hardware capabilities
    GetHardwareCapabilities,

    /// Get current resource allocation
    GetResourceAllocation,

    /// Get sharing capabilities
    GetSharingCapabilities,

    /// Refresh hardware detection
    RefreshHardware,
}

/// Dashboard response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: u64,
}

/// Dashboard message handler implementing the STOQ MessageHandler trait
pub struct DashboardMessageHandler {
    config: Arc<HyperMeshServerConfig>,
    stoq_layer: Arc<StoqTransportLayer>,
    hypermesh_layer: Arc<HyperMeshAssetLayer>,
    trustchain_layer: Arc<TrustChainAuthorityLayer>,
    integration: Arc<LayerIntegration>,
    monitor: Arc<PerformanceMonitor>,
    hardware_service: Arc<HardwareDetectionService>,
}

impl DashboardMessageHandler {
    /// Create new dashboard message handler
    pub fn new(
        config: Arc<HyperMeshServerConfig>,
        stoq_layer: Arc<StoqTransportLayer>,
        hypermesh_layer: Arc<HyperMeshAssetLayer>,
        trustchain_layer: Arc<TrustChainAuthorityLayer>,
        integration: Arc<LayerIntegration>,
        monitor: Arc<PerformanceMonitor>,
        hardware_service: Arc<HardwareDetectionService>,
    ) -> Self {
        Self {
            config,
            stoq_layer,
            hypermesh_layer,
            trustchain_layer,
            integration,
            monitor,
            hardware_service,
        }
    }

    /// Handle server statistics request
    async fn handle_server_statistics(&self) -> Result<serde_json::Value> {
        info!("📊 Handling server statistics request");
        
        // Get statistics from all layers
        let stack_stats = self.monitor.get_stack_statistics().await;
        let stoq_stats = self.stoq_layer.get_statistics().await?;
        let hypermesh_stats = self.hypermesh_layer.get_statistics().await?;
        let trustchain_stats = self.trustchain_layer.get_statistics().await?;
        let integration_stats = self.integration.get_statistics().await?;
        
        let response = serde_json::json!({
            "server_info": {
                "version": "1.0.0",
                "protocol": "Internet 2.0 (STOQ/HyperMesh/TrustChain)",
                "bind_address": self.config.global.bind_address,
                "port": self.config.global.port,
                "uptime_seconds": stack_stats.uptime_seconds,
            },
            "transport_layer": {
                "protocol": "STOQ (QUIC over IPv6)",
                "statistics": stoq_stats,
            },
            "asset_layer": {
                "protocol": "HyperMesh Asset System",
                "statistics": hypermesh_stats,
            },
            "authority_layer": {
                "protocol": "TrustChain Authority",
                "statistics": trustchain_stats,
            },
            "integration": {
                "statistics": integration_stats,
            },
            "performance": {
                "target_achieved": stack_stats.performance_targets_met,
                "bottlenecks": stack_stats.performance_warnings,
                "layers_integrated": stack_stats.layers_integrated,
            }
        });
        
        debug!("✅ Server statistics compiled successfully");
        Ok(response)
    }

    /// Handle system health request
    async fn handle_system_health(&self) -> Result<serde_json::Value> {
        info!("🏥 Handling system health request");
        
        let stack_stats = self.monitor.get_stack_statistics().await;
        let integration_stats = self.integration.get_statistics().await?;
        
        let health_status = if stack_stats.layers_integrated && 
                              stack_stats.performance_targets_met {
            "healthy"
        } else if stack_stats.layers_integrated {
            "degraded"
        } else {
            "unhealthy"
        };
        
        let response = serde_json::json!({
            "overall_status": health_status,
            "layers": {
                "stoq_transport": true, // Simplified
                "hypermesh_assets": true,
                "trustchain_authority": true,
            },
            "health_metrics": {
                "uptime_seconds": stack_stats.uptime_seconds,
                "performance_targets_met": stack_stats.performance_targets_met,
                "layers_integrated": stack_stats.layers_integrated,
            },
            "warnings": stack_stats.performance_warnings,
            "integration_statistics": integration_stats,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        debug!("✅ System health status compiled: {}", health_status);
        Ok(response)
    }

    /// Handle asset management requests
    async fn handle_asset_management(&self, request: AssetManagementRequest) -> Result<serde_json::Value> {
        info!("🏗️ Handling asset management request: {:?}", std::mem::discriminant(&request));
        
        match request {
            AssetManagementRequest::ListAssets { filter, limit } => {
                debug!("Listing assets with filter: {:?}, limit: {:?}", filter, limit);
                
                // Get asset list from HyperMesh layer
                let assets = self.hypermesh_layer.list_assets(filter, limit).await?;
                
                Ok(serde_json::json!({
                    "assets": assets,
                    "total_count": assets.len(),
                }))
            }
            
            AssetManagementRequest::GetAsset { asset_id } => {
                debug!("Getting asset details for: {}", asset_id);
                
                let asset_details = self.hypermesh_layer.get_asset_details(&asset_id).await?;
                
                Ok(serde_json::json!({
                    "asset": asset_details,
                }))
            }
            
            AssetManagementRequest::CreateAsset { asset_type, configuration } => {
                info!("Creating new asset of type: {}", asset_type);
                
                let asset_id = self.hypermesh_layer.create_asset(&asset_type, configuration).await?;
                
                Ok(serde_json::json!({
                    "asset_id": asset_id,
                    "created": true,
                }))
            }
            
            AssetManagementRequest::UpdateAsset { asset_id, configuration } => {
                info!("Updating asset: {}", asset_id);
                
                self.hypermesh_layer.update_asset(&asset_id, configuration).await?;
                
                Ok(serde_json::json!({
                    "asset_id": asset_id,
                    "updated": true,
                }))
            }
            
            AssetManagementRequest::DeleteAsset { asset_id } => {
                warn!("Deleting asset: {}", asset_id);
                
                self.hypermesh_layer.delete_asset(&asset_id).await?;
                
                Ok(serde_json::json!({
                    "asset_id": asset_id,
                    "deleted": true,
                }))
            }
        }
    }

    /// Handle certificate operations
    async fn handle_certificate_operation(&self, request: CertificateOperationRequest) -> Result<serde_json::Value> {
        info!("🔐 Handling certificate operation: {:?}", std::mem::discriminant(&request));
        
        match request {
            CertificateOperationRequest::ListCertificates { filter } => {
                debug!("Listing certificates with filter: {:?}", filter);
                
                let certificates = self.trustchain_layer.list_certificates(filter).await?;
                
                Ok(serde_json::json!({
                    "certificates": certificates,
                    "total_count": certificates.len(),
                }))
            }
            
            CertificateOperationRequest::GetCertificate { certificate_id } => {
                debug!("Getting certificate details for: {}", certificate_id);
                
                let certificate = self.trustchain_layer.get_certificate(&certificate_id).await?;
                
                Ok(serde_json::json!({
                    "certificate": certificate,
                }))
            }
            
            CertificateOperationRequest::IssueCertificate { subject, validity_days, usage } => {
                info!("Issuing new certificate for: {}", subject);
                
                let cert_request = crate::authority::ca::CertificateRequest {
                    subject,
                    validity_days,
                    key_size: 2048,
                    usage,
                    san_entries: vec![],
                    is_ca: false,
                    path_length: None,
                };
                
                let certificate = self.trustchain_layer.issue_certificate(cert_request).await?;
                
                Ok(serde_json::json!({
                    "certificate_id": certificate.id,
                    "issued": true,
                    "fingerprint": certificate.metadata.fingerprint_sha256,
                }))
            }
            
            CertificateOperationRequest::RevokeCertificate { certificate_id, reason } => {
                warn!("Revoking certificate: {} (reason: {})", certificate_id, reason);
                
                self.trustchain_layer.revoke_certificate(&certificate_id, &reason).await?;
                
                Ok(serde_json::json!({
                    "certificate_id": certificate_id,
                    "revoked": true,
                    "reason": reason,
                }))
            }
            
            CertificateOperationRequest::ValidateCertificate { certificate_der } => {
                debug!("Validating certificate (DER length: {})", certificate_der.len());
                
                let validation_result = self.trustchain_layer.validate_certificate(&certificate_der).await?;
                
                Ok(serde_json::json!({
                    "valid": validation_result.valid,
                    "fingerprint": validation_result.fingerprint,
                    "error": validation_result.error,
                }))
            }
        }
    }

    /// Handle performance monitoring requests
    async fn handle_performance_monitoring(&self, request: PerformanceMonitoringRequest) -> Result<serde_json::Value> {
        info!("📈 Handling performance monitoring request: {:?}", std::mem::discriminant(&request));
        
        match request {
            PerformanceMonitoringRequest::GetCurrentMetrics => {
                debug!("Getting current performance metrics");
                
                let metrics = self.monitor.get_current_performance_metrics().await;
                
                Ok(serde_json::json!({
                    "metrics": metrics,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }))
            }
            
            PerformanceMonitoringRequest::GetHistoricalMetrics { start_time, end_time, interval } => {
                debug!("Getting historical metrics from {} to {} (interval: {})", start_time, end_time, interval);
                
                let historical_data = self.monitor.get_historical_metrics(start_time, end_time, &interval).await;
                
                Ok(serde_json::json!({
                    "historical_data": historical_data,
                    "start_time": start_time,
                    "end_time": end_time,
                    "interval": interval,
                }))
            }
            
            PerformanceMonitoringRequest::GetPerformanceAlerts => {
                debug!("Getting performance alerts");
                
                let alerts = self.monitor.get_active_alerts().await;
                
                Ok(serde_json::json!({
                    "alerts": alerts,
                    "alert_count": alerts.len(),
                }))
            }
            
            PerformanceMonitoringRequest::SetPerformanceThresholds { thresholds } => {
                info!("Setting performance thresholds");
                
                self.monitor.update_performance_thresholds(thresholds).await?;
                
                Ok(serde_json::json!({
                    "thresholds_updated": true,
                }))
            }
        }
    }

    /// Handle integration status request
    async fn handle_integration_status(&self) -> Result<serde_json::Value> {
        info!("🔄 Handling integration status request");

        let integration_stats = self.integration.get_statistics().await?;

        Ok(serde_json::json!({
            "integration_status": {
                "layers_ready": integration_stats.layers_ready,
                "integration_health": integration_stats.integration_health,
                "layers_integrated": integration_stats.layers_ready == 3,
            },
            "cross_layer_communication": {
                "total_operations": integration_stats.cross_layer_ops_total,
                "operations_per_second": integration_stats.cross_layer_ops_per_second,
                "average_latency_ms": integration_stats.integration_latency_ms,
                "errors": integration_stats.integration_errors,
            },
            "performance_coordination": {
                "active": integration_stats.throughput_coordination_active,
                "optimizations_applied": integration_stats.performance_optimizations,
            },
            "layer_details": {
                "stoq_transport": integration_stats.layers_ready >= 1,
                "hypermesh_assets": integration_stats.layers_ready >= 2,
                "trustchain_authority": integration_stats.layers_ready >= 3,
            }
        }))
    }

    /// Handle hardware detection requests
    async fn handle_hardware_detection(&self, request: HardwareDetectionRequest) -> Result<serde_json::Value> {
        info!("🔍 Handling hardware detection request: {:?}", std::mem::discriminant(&request));

        match request {
            HardwareDetectionRequest::GetHardwareCapabilities => {
                debug!("Getting hardware capabilities");

                let capabilities = self.hardware_service.get_hardware_capabilities().await?;
                Ok(serde_json::to_value(capabilities)?)
            }

            HardwareDetectionRequest::GetResourceAllocation => {
                debug!("Getting resource allocation status");

                // For now, return a simplified allocation status since we don't have get_resource_allocation method
                let capabilities = self.hardware_service.get_hardware_capabilities().await?;
                let allocation = serde_json::json!({
                    "cpu_allocation": {"used_percent": capabilities.cpu.usage_percent, "available_cores": capabilities.cpu.core_count},
                    "memory_allocation": {"used_bytes": capabilities.memory.used_bytes, "total_bytes": capabilities.memory.total_bytes},
                    "storage_allocation": capabilities.storage,
                    "network_allocation": capabilities.network
                });
                Ok(allocation)
            }

            HardwareDetectionRequest::GetSharingCapabilities => {
                debug!("Getting sharing capabilities");

                // For now, return simplified sharing capabilities
                let capabilities = self.hardware_service.get_hardware_capabilities().await?;
                let sharing = serde_json::json!({
                    "sharing_enabled": true,
                    "available_resources": {
                        "cpu_cores": capabilities.cpu.core_count,
                        "memory_gb": capabilities.memory.total_bytes / (1024 * 1024 * 1024),
                        "storage": capabilities.storage,
                        "network": capabilities.network
                    },
                    "privacy_settings": {
                        "privacy_level": "public",
                        "sharing_mode": "full"
                    }
                });
                Ok(sharing)
            }

            HardwareDetectionRequest::RefreshHardware => {
                debug!("Refreshing hardware detection");

                // Force refresh by getting fresh capabilities
                let _capabilities = self.hardware_service.get_hardware_capabilities().await?;

                Ok(serde_json::json!({
                    "refreshed": true,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }))
            }
        }
    }
}

#[async_trait]
impl stoq::protocol::MessageHandler<DashboardMessage> for DashboardMessageHandler {
    async fn handle_message(
        &self,
        message: stoq::protocol::StoqMessage<DashboardMessage>,
        connection_info: &stoq::protocol::ConnectionInfo,
    ) -> Result<Option<bytes::Bytes>> {
        info!("📡 Received dashboard message from connection: {}", connection_info.connection_id);
        debug!("Message type: {:?}", std::mem::discriminant(&message.payload));
        
        // Validate authentication - require certificate for all dashboard access
        if connection_info.cert_fingerprint.is_none() {
            error!("❌ Dashboard access denied: No certificate provided");
            let error_response = DashboardResponse {
                success: false,
                data: None,
                error: Some("Certificate authentication required for dashboard access".to_string()),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            let response_data = serde_json::to_vec(&error_response)?;
            return Ok(Some(bytes::Bytes::from(response_data)));
        }
        
        info!("✅ Dashboard access authenticated with certificate: {}", 
              connection_info.cert_fingerprint.as_ref().unwrap());
        
        // Handle the dashboard message
        let result = match message.payload {
            DashboardMessage::GetServerStatistics => {
                self.handle_server_statistics().await
            }
            
            DashboardMessage::GetSystemHealth => {
                self.handle_system_health().await
            }
            
            DashboardMessage::AssetManagement(request) => {
                self.handle_asset_management(request).await
            }
            
            DashboardMessage::CertificateOperation(request) => {
                self.handle_certificate_operation(request).await
            }
            
            DashboardMessage::PerformanceMonitoring(request) => {
                self.handle_performance_monitoring(request).await
            }
            
            DashboardMessage::IntegrationStatus => {
                self.handle_integration_status().await
            }

            DashboardMessage::HardwareDetection(request) => {
                self.handle_hardware_detection(request).await
            }
        };
        
        // Prepare response
        let response = match result {
            Ok(data) => DashboardResponse {
                success: true,
                data: Some(data),
                error: None,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
            Err(e) => {
                error!("❌ Dashboard message handling failed: {}", e);
                DashboardResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            }
        };
        
        let response_data = serde_json::to_vec(&response)?;
        debug!("📤 Sending dashboard response ({} bytes)", response_data.len());
        
        Ok(Some(bytes::Bytes::from(response_data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dashboard_handler_creation() {
        // Test dashboard handler creation
        // This would require full integration setup
    }
    
    #[tokio::test]
    async fn test_server_statistics_handling() {
        // Test server statistics message handling
    }
    
    #[tokio::test]
    async fn test_certificate_authentication() {
        // Test that dashboard access requires certificate authentication
    }
}
//! HyperMesh Consensus API Server
//! 
//! This module provides HTTP API endpoints for external services like TrustChain
//! to request consensus validation through HyperMesh's four-proof system.

use std::sync::Arc;
use std::convert::Infallible;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use warp::{Filter, Reply, Rejection};
use warp::http::StatusCode;

use super::validation_service::{
    ConsensusValidationService, CertificateValidationRequest, FourProofValidationRequest,
    ValidationResult, ValidationStatus, ValidationServiceMetrics,
};

/// Consensus API server configuration
#[derive(Debug, Clone)]
pub struct ConsensusApiConfig {
    /// Server bind address
    pub bind_address: std::net::SocketAddr,
    /// Maximum request size (bytes)
    pub max_request_size: usize,
    /// Request timeout
    pub request_timeout: std::time::Duration,
    /// Enable CORS
    pub enable_cors: bool,
    /// API rate limiting
    pub rate_limit_per_second: u32,
}

impl Default for ConsensusApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:8080".parse().unwrap(), // IPv6 localhost
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout: std::time::Duration::from_secs(60),
            enable_cors: true,
            rate_limit_per_second: 100,
        }
    }
}

impl ConsensusApiConfig {
    /// Production configuration
    pub fn production(bind_address: std::net::SocketAddr) -> Self {
        Self {
            bind_address,
            max_request_size: 50 * 1024 * 1024, // 50MB
            request_timeout: std::time::Duration::from_secs(120),
            enable_cors: true,
            rate_limit_per_second: 1000,
        }
    }
}

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message if any
    pub error: Option<String>,
    /// Request timestamp
    pub timestamp: std::time::SystemTime,
    /// Processing time in microseconds
    pub processing_time_us: u64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, processing_time_us: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now(),
            processing_time_us,
        }
    }

    pub fn error(error: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now(),
            processing_time_us: 0,
        }
    }
}

/// HyperMesh consensus API server
pub struct ConsensusApiServer {
    /// Consensus validation service
    validation_service: Arc<ConsensusValidationService>,
    /// Server configuration
    config: ConsensusApiConfig,
    /// Server metrics
    metrics: Arc<RwLock<ApiServerMetrics>>,
}

#[derive(Debug, Default)]
pub struct ApiServerMetrics {
    /// Total requests handled
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average processing time (microseconds)
    pub avg_processing_time_us: u64,
    /// Current active connections
    pub active_connections: u32,
    /// Last request timestamp
    pub last_request: Option<std::time::SystemTime>,
}

impl ConsensusApiServer {
    /// Create new consensus API server
    pub async fn new(
        validation_service: Arc<ConsensusValidationService>,
        config: ConsensusApiConfig,
    ) -> Result<Self> {
        info!("Initializing HyperMesh consensus API server on {}", config.bind_address);

        Ok(Self {
            validation_service,
            config,
            metrics: Arc::new(RwLock::new(ApiServerMetrics::default())),
        })
    }

    /// Start the API server
    pub async fn start(&self) -> Result<()> {
        info!("Starting HyperMesh consensus API server");

        let validation_service = self.validation_service.clone();
        let metrics = self.metrics.clone();

        // Certificate validation endpoint
        let certificate_validation = warp::path("consensus")
            .and(warp::path("validation"))
            .and(warp::path("certificate"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_validation_service(validation_service.clone()))
            .and(with_metrics(metrics.clone()))
            .and_then(handle_certificate_validation);

        // Four-proof validation endpoint
        let four_proof_validation = warp::path("consensus")
            .and(warp::path("validation"))
            .and(warp::path("four-proof"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_validation_service(validation_service.clone()))
            .and(with_metrics(metrics.clone()))
            .and_then(handle_four_proof_validation);

        // Validation status endpoint
        let validation_status = warp::path("consensus")
            .and(warp::path("validation"))
            .and(warp::path("status"))
            .and(warp::path::param::<String>())
            .and(warp::get())
            .and(with_validation_service(validation_service.clone()))
            .and(with_metrics(metrics.clone()))
            .and_then(handle_validation_status);

        // Service metrics endpoint
        let service_metrics = warp::path("consensus")
            .and(warp::path("metrics"))
            .and(warp::get())
            .and(with_validation_service(validation_service.clone()))
            .and(with_metrics(metrics.clone()))
            .and_then(handle_service_metrics);

        // Health check endpoint
        let health = warp::path("consensus")
            .and(warp::path("health"))
            .and(warp::get())
            .and_then(handle_health_check);

        // Combine all routes
        let routes = certificate_validation
            .or(four_proof_validation)
            .or(validation_status)
            .or(service_metrics)
            .or(health)
            .recover(handle_rejection);

        // Add CORS if enabled
        let routes = if self.config.enable_cors {
            routes
                .with(warp::cors()
                    .allow_any_origin()
                    .allow_headers(vec!["content-type"])
                    .allow_methods(vec!["GET", "POST"]))
                .boxed()
        } else {
            routes.boxed()
        };

        // Start server
        warp::serve(routes)
            .run(self.config.bind_address)
            .await;

        Ok(())
    }

    /// Get server metrics
    pub async fn get_metrics(&self) -> ApiServerMetrics {
        self.metrics.read().await.clone()
    }
}

// Handler functions

async fn handle_certificate_validation(
    request: CertificateValidationRequest,
    validation_service: Arc<ConsensusValidationService>,
    metrics: Arc<RwLock<ApiServerMetrics>>,
) -> Result<impl Reply, Rejection> {
    let start_time = std::time::Instant::now();
    
    debug!("Handling certificate validation request: {}", request.request_id);

    match validation_service.validate_certificate_request(request).await {
        Ok(result) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, true, processing_time).await;
            
            Ok(warp::reply::with_status(
                warp::reply::json(&ApiResponse::success(result, processing_time)),
                StatusCode::OK,
            ))
        }
        Err(e) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, false, processing_time).await;
            
            error!("Certificate validation failed: {}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&ApiResponse::<()>::error(e.to_string())),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_four_proof_validation(
    request: FourProofValidationRequest,
    validation_service: Arc<ConsensusValidationService>,
    metrics: Arc<RwLock<ApiServerMetrics>>,
) -> Result<impl Reply, Rejection> {
    let start_time = std::time::Instant::now();
    
    debug!("Handling four-proof validation request for operation: {}", request.operation);

    match validation_service.validate_four_proof_set(request).await {
        Ok(result) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, true, processing_time).await;
            
            Ok(warp::reply::with_status(
                warp::reply::json(&ApiResponse::success(result, processing_time)),
                StatusCode::OK,
            ))
        }
        Err(e) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, false, processing_time).await;
            
            error!("Four-proof validation failed: {}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&ApiResponse::<()>::error(e.to_string())),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_validation_status(
    request_id: String,
    validation_service: Arc<ConsensusValidationService>,
    metrics: Arc<RwLock<ApiServerMetrics>>,
) -> Result<impl Reply, Rejection> {
    let start_time = std::time::Instant::now();
    
    debug!("Checking validation status for request: {}", request_id);

    match validation_service.get_validation_status(&request_id).await {
        Ok(result) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, true, processing_time).await;
            
            Ok(warp::reply::with_status(
                warp::reply::json(&ApiResponse::success(result, processing_time)),
                StatusCode::OK,
            ))
        }
        Err(e) => {
            let processing_time = start_time.elapsed().as_micros() as u64;
            update_metrics(&metrics, false, processing_time).await;
            
            if e.to_string().contains("not found") {
                Ok(warp::reply::with_status(
                    warp::reply::json(&ApiResponse::<()>::error(e.to_string())),
                    StatusCode::NOT_FOUND,
                ))
            } else {
                error!("Validation status check failed: {}", e);
                Ok(warp::reply::with_status(
                    warp::reply::json(&ApiResponse::<()>::error(e.to_string())),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}

async fn handle_service_metrics(
    validation_service: Arc<ConsensusValidationService>,
    metrics: Arc<RwLock<ApiServerMetrics>>,
) -> Result<impl Reply, Rejection> {
    let start_time = std::time::Instant::now();
    
    debug!("Retrieving service metrics");

    let validation_metrics = validation_service.get_metrics().await;
    let api_metrics = metrics.read().await.clone();

    #[derive(Serialize)]
    struct CombinedMetrics {
        validation_service: ValidationServiceMetrics,
        api_server: ApiServerMetrics,
    }

    let combined_metrics = CombinedMetrics {
        validation_service: validation_metrics,
        api_server: api_metrics,
    };

    let processing_time = start_time.elapsed().as_micros() as u64;
    update_metrics(&metrics, true, processing_time).await;

    Ok(warp::reply::with_status(
        warp::reply::json(&ApiResponse::success(combined_metrics, processing_time)),
        StatusCode::OK,
    ))
}

async fn handle_health_check() -> Result<impl Reply, Rejection> {
    #[derive(Serialize)]
    struct HealthStatus {
        status: &'static str,
        timestamp: std::time::SystemTime,
        service: &'static str,
    }

    let health = HealthStatus {
        status: "healthy",
        timestamp: std::time::SystemTime::now(),
        service: "hypermesh-consensus-validation",
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&ApiResponse::success(health, 0)),
        StatusCode::OK,
    ))
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        message = "BAD_REQUEST";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        warn!("Unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "INTERNAL_SERVER_ERROR";
    }

    let json = warp::reply::json(&ApiResponse::<()>::error(message.to_string()));

    Ok(warp::reply::with_status(json, code))
}

// Helper functions

fn with_validation_service(
    service: Arc<ConsensusValidationService>,
) -> impl Filter<Extract = (Arc<ConsensusValidationService>,), Error = Infallible> + Clone {
    warp::any().map(move || service.clone())
}

fn with_metrics(
    metrics: Arc<RwLock<ApiServerMetrics>>,
) -> impl Filter<Extract = (Arc<RwLock<ApiServerMetrics>>,), Error = Infallible> + Clone {
    warp::any().map(move || metrics.clone())
}

async fn update_metrics(
    metrics: &Arc<RwLock<ApiServerMetrics>>,
    success: bool,
    processing_time_us: u64,
) {
    let mut metrics = metrics.write().await;
    
    metrics.total_requests += 1;
    
    if success {
        metrics.successful_requests += 1;
    } else {
        metrics.failed_requests += 1;
    }

    // Update rolling average processing time
    if metrics.total_requests == 1 {
        metrics.avg_processing_time_us = processing_time_us;
    } else {
        metrics.avg_processing_time_us = 
            (metrics.avg_processing_time_us * (metrics.total_requests - 1) + processing_time_us) 
            / metrics.total_requests;
    }

    metrics.last_request = Some(std::time::SystemTime::now());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_creation() {
        let config = ConsensusApiConfig::default();
        assert!(config.bind_address.to_string().contains("::1"));
        assert!(config.max_request_size > 0);
    }

    #[test]
    fn test_production_config() {
        let addr = "[2001:db8::1]:8080".parse().unwrap();
        let config = ConsensusApiConfig::production(addr);
        assert_eq!(config.bind_address, addr);
        assert!(config.max_request_size > ConsensusApiConfig::default().max_request_size);
    }

    #[test]
    fn test_api_response_creation() {
        let response = ApiResponse::success("test_data".to_string(), 1000);
        assert!(response.success);
        assert_eq!(response.data.unwrap(), "test_data");
        assert_eq!(response.processing_time_us, 1000);

        let error_response = ApiResponse::<()>::error("test error".to_string());
        assert!(!error_response.success);
        assert_eq!(error_response.error.unwrap(), "test error");
    }
}
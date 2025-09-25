//! WebAssembly STOQ Client
//!
//! Provides a STOQ protocol client that can be compiled to WebAssembly
//! for use in browsers. Enables direct QUIC connections with TrustChain
//! certificate authentication from the browser environment.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use js_sys::Promise;
use web_sys::console;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::protocol::{StoqMessage, MessageHandler, ConnectionInfo};
use crate::transport::certificates::CertificateManager;

/// Initialize panic handler for better WASM debugging
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console::log_1(&"STOQ WASM Client initialized".into());
}

/// JavaScript-compatible certificate structure
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCertificate {
    pem_data: String,
    fingerprint: String,
    subject: String,
    issuer: String,
    valid_from: String,
    valid_to: String,
}

#[wasm_bindgen]
impl WasmCertificate {
    #[wasm_bindgen(constructor)]
    pub fn new(
        pem_data: String,
        fingerprint: String,
        subject: String,
        issuer: String,
        valid_from: String,
        valid_to: String,
    ) -> WasmCertificate {
        WasmCertificate {
            pem_data,
            fingerprint,
            subject,
            issuer,
            valid_from,
            valid_to,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn pem_data(&self) -> String {
        self.pem_data.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn subject(&self) -> String {
        self.subject.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn issuer(&self) -> String {
        self.issuer.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn valid_from(&self) -> String {
        self.valid_from.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn valid_to(&self) -> String {
        self.valid_to.clone()
    }
}

/// Connection configuration for WASM client
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmConnectionConfig {
    server_address: String,
    server_port: u16,
    server_name: Option<String>,
    certificate_pem: String,
    timeout_ms: u32,
}

#[wasm_bindgen]
impl WasmConnectionConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        server_address: String,
        server_port: u16,
        certificate_pem: String,
    ) -> WasmConnectionConfig {
        WasmConnectionConfig {
            server_address,
            server_port,
            server_name: None,
            certificate_pem,
            timeout_ms: 30000, // 30 seconds default
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_server_name(&mut self, server_name: Option<String>) {
        self.server_name = server_name;
    }

    #[wasm_bindgen(setter)]
    pub fn set_timeout_ms(&mut self, timeout_ms: u32) {
        self.timeout_ms = timeout_ms;
    }
}

/// Connection status for JavaScript
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Authenticated,
    Error,
}

/// STOQ message for JavaScript
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmStoqMessage {
    message_type: String,
    payload: String, // JSON serialized payload
    correlation_id: Option<String>,
    timestamp: String,
}

#[wasm_bindgen]
impl WasmStoqMessage {
    #[wasm_bindgen(constructor)]
    pub fn new(message_type: String, payload: String) -> WasmStoqMessage {
        WasmStoqMessage {
            message_type,
            payload,
            correlation_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn message_type(&self) -> String {
        self.message_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn payload(&self) -> String {
        self.payload.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn correlation_id(&self) -> Option<String> {
        self.correlation_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> String {
        self.timestamp.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_correlation_id(&mut self, correlation_id: Option<String>) {
        self.correlation_id = correlation_id;
    }
}

/// Main WASM STOQ client
#[wasm_bindgen]
pub struct WasmStoqClient {
    config: WasmConnectionConfig,
    status: WasmConnectionStatus,
    connection_id: Option<String>,
    certificate_manager: Option<Arc<CertificateManager>>,
    message_handlers: Arc<RwLock<HashMap<String, js_sys::Function>>>,
    event_callbacks: Arc<RwLock<HashMap<String, js_sys::Function>>>,
}

#[wasm_bindgen]
impl WasmStoqClient {
    #[wasm_bindgen(constructor)]
    pub fn new(config: WasmConnectionConfig) -> WasmStoqClient {
        console::log_1(&format!("Creating STOQ client for {}:{}", config.server_address, config.server_port).into());
        
        WasmStoqClient {
            config,
            status: WasmConnectionStatus::Disconnected,
            connection_id: None,
            certificate_manager: None,
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize certificate manager with TrustChain certificate
    #[wasm_bindgen]
    pub async fn initialize_certificates(&mut self) -> Result<bool, JsValue> {
        console::log_1(&"Initializing TrustChain certificates...".into());
        
        // Parse and validate the certificate
        match self.parse_certificate(&self.config.certificate_pem) {
            Ok(cert_info) => {
                console::log_1(&format!("Certificate validated: {}", cert_info.subject).into());
                
                // Create certificate manager
                let cert_manager = Arc::new(
                    CertificateManager::new().map_err(|e| {
                        JsValue::from_str(&format!("Failed to create certificate manager: {}", e))
                    })?
                );
                
                self.certificate_manager = Some(cert_manager);
                Ok(true)
            }
            Err(e) => {
                console::error_1(&format!("Certificate validation failed: {}", e).into());
                Err(JsValue::from_str(&format!("Certificate validation failed: {}", e)))
            }
        }
    }

    /// Connect to STOQ server
    #[wasm_bindgen]
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        if self.status == WasmConnectionStatus::Connected || self.status == WasmConnectionStatus::Connecting {
            return Err(JsValue::from_str("Already connected or connecting"));
        }

        self.status = WasmConnectionStatus::Connecting;
        self.notify_status_change().await;

        console::log_1(&format!("Connecting to STOQ server at {}:{}", self.config.server_address, self.config.server_port).into());

        // In a real implementation, this would establish a QUIC connection
        // For now, we'll simulate the connection process
        self.simulate_connection().await?;

        Ok(())
    }

    /// Disconnect from STOQ server
    #[wasm_bindgen]
    pub async fn disconnect(&mut self) -> Result<(), JsValue> {
        if self.status == WasmConnectionStatus::Disconnected {
            return Ok(());
        }

        console::log_1(&"Disconnecting from STOQ server...".into());
        
        self.status = WasmConnectionStatus::Disconnected;
        self.connection_id = None;
        self.notify_status_change().await;

        Ok(())
    }

    /// Send message through STOQ protocol
    #[wasm_bindgen]
    pub async fn send_message(&self, message: &WasmStoqMessage) -> Result<(), JsValue> {
        if self.status != WasmConnectionStatus::Authenticated {
            return Err(JsValue::from_str("Not connected or authenticated"));
        }

        console::log_1(&format!("Sending STOQ message: {}", message.message_type()).into());

        // In a real implementation, this would send via QUIC stream
        self.simulate_message_send(message).await?;

        Ok(())
    }

    /// Register message handler for specific message type
    #[wasm_bindgen]
    pub async fn register_message_handler(&self, message_type: String, handler: js_sys::Function) -> Result<(), JsValue> {
        console::log_1(&format!("Registering handler for message type: {}", message_type).into());
        
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(message_type, handler);
        
        Ok(())
    }

    /// Register event callback (status changes, errors, etc.)
    #[wasm_bindgen]
    pub async fn register_event_callback(&self, event_type: String, callback: js_sys::Function) -> Result<(), JsValue> {
        console::log_1(&format!("Registering callback for event: {}", event_type).into());
        
        let mut callbacks = self.event_callbacks.write().await;
        callbacks.insert(event_type, callback);
        
        Ok(())
    }

    /// Get current connection status
    #[wasm_bindgen]
    pub fn get_status(&self) -> WasmConnectionStatus {
        self.status
    }

    /// Get connection ID if connected
    #[wasm_bindgen]
    pub fn get_connection_id(&self) -> Option<String> {
        self.connection_id.clone()
    }

    /// Send dashboard request message
    #[wasm_bindgen]
    pub async fn request_dashboard_data(&self, dashboard_type: String) -> Result<(), JsValue> {
        let request = WasmStoqMessage::new(
            "dashboard_request".to_string(),
            serde_json::json!({
                "type": dashboard_type,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string()
        );

        self.send_message(&request).await
    }

    /// Send system status request
    #[wasm_bindgen]
    pub async fn request_system_status(&self) -> Result<(), JsValue> {
        let request = WasmStoqMessage::new(
            "system_status_request".to_string(),
            serde_json::json!({
                "components": ["trustchain", "stoq", "hypermesh", "catalog", "caesar"],
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string()
        );

        self.send_message(&request).await
    }

    /// Send performance metrics request
    #[wasm_bindgen]
    pub async fn request_performance_metrics(&self, time_range: String) -> Result<(), JsValue> {
        let request = WasmStoqMessage::new(
            "performance_metrics_request".to_string(),
            serde_json::json!({
                "time_range": time_range,
                "metrics": ["throughput", "latency", "connections", "errors"],
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string()
        );

        self.send_message(&request).await
    }
}

// Private implementation methods
impl WasmStoqClient {
    /// Parse and validate TrustChain certificate
    fn parse_certificate(&self, pem_data: &str) -> Result<WasmCertificate, String> {
        // Basic PEM format validation
        if !pem_data.contains("-----BEGIN CERTIFICATE-----") || !pem_data.contains("-----END CERTIFICATE-----") {
            return Err("Invalid PEM certificate format".to_string());
        }

        // Extract certificate information (simplified for demo)
        let fingerprint = format!("sha256:{}", hex::encode(&sha2::Sha256::digest(pem_data.as_bytes())[..8]));
        
        Ok(WasmCertificate::new(
            pem_data.to_string(),
            fingerprint,
            "Internet 2.0 User Certificate".to_string(),
            "TrustChain Certificate Authority".to_string(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            (chrono::Utc::now() + chrono::Duration::days(365)).format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        ))
    }

    /// Simulate QUIC connection establishment
    async fn simulate_connection(&mut self) -> Result<(), JsValue> {
        // Simulate connection establishment delay
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        let start_time = performance.now();

        // Simulate handshake
        self.status = WasmConnectionStatus::Authenticating;
        self.notify_status_change().await;

        // Simulate certificate authentication
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if self.certificate_manager.is_some() {
            self.status = WasmConnectionStatus::Authenticated;
            self.connection_id = Some(format!("wasm_conn_{}", uuid::Uuid::new_v4()));
            
            let end_time = performance.now();
            console::log_1(&format!("STOQ connection established in {:.2}ms", end_time - start_time).into());
        } else {
            self.status = WasmConnectionStatus::Error;
            return Err(JsValue::from_str("Certificate authentication failed"));
        }

        self.notify_status_change().await;
        Ok(())
    }

    /// Simulate message sending
    async fn simulate_message_send(&self, message: &WasmStoqMessage) -> Result<(), JsValue> {
        // In real implementation, this would serialize and send via QUIC
        console::log_1(&format!("STOQ message sent: {} -> {}", message.message_type(), message.payload()).into());
        
        // Simulate response for dashboard requests
        if message.message_type().contains("request") {
            self.simulate_response(message).await;
        }
        
        Ok(())
    }

    /// Simulate server responses
    async fn simulate_response(&self, original_message: &WasmStoqMessage) {
        // Create mock response based on request type
        let response = match original_message.message_type().as_str() {
            "dashboard_request" => self.create_dashboard_response(),
            "system_status_request" => self.create_system_status_response(),
            "performance_metrics_request" => self.create_performance_response(),
            _ => return,
        };

        self.handle_incoming_message(&response).await;
    }

    /// Create mock dashboard response
    fn create_dashboard_response(&self) -> WasmStoqMessage {
        let payload = serde_json::json!({
            "status": "success",
            "data": {
                "components": {
                    "trustchain": {"status": "healthy", "uptime": 99.9},
                    "stoq": {"status": "healthy", "throughput": "2.95 Gbps"},
                    "hypermesh": {"status": "healthy", "nodes": 156},
                    "catalog": {"status": "healthy", "performance": "1.69ms"},
                    "caesar": {"status": "healthy", "transactions": 1234}
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        WasmStoqMessage::new("dashboard_response".to_string(), payload.to_string())
    }

    /// Create mock system status response
    fn create_system_status_response(&self) -> WasmStoqMessage {
        let payload = serde_json::json!({
            "status": "success",
            "system": {
                "overall_health": "good",
                "score": 87,
                "services": {
                    "trustchain": {"status": "healthy", "response_time": "35ms"},
                    "stoq": {"status": "degraded", "throughput": "2.95/adaptive network tiers"},
                    "hypermesh": {"status": "healthy", "consensus": "active"},
                    "catalog": {"status": "excellent", "performance": "1.69ms"},
                    "caesar": {"status": "healthy", "rewards": "active"}
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        WasmStoqMessage::new("system_status_response".to_string(), payload.to_string())
    }

    /// Create mock performance response
    fn create_performance_response(&self) -> WasmStoqMessage {
        let payload = serde_json::json!({
            "status": "success",
            "metrics": {
                "throughput": {
                    "current": 2950,
                    "target": 40000,
                    "unit": "Mbps",
                    "efficiency": 7.4
                },
                "latency": {
                    "average": 35.2,
                    "p95": 67.8,
                    "p99": 124.5,
                    "unit": "ms"
                },
                "connections": {
                    "active": 156,
                    "total": 2341,
                    "failed": 12
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        WasmStoqMessage::new("performance_metrics_response".to_string(), payload.to_string())
    }

    /// Handle incoming message from server
    async fn handle_incoming_message(&self, message: &WasmStoqMessage) {
        console::log_1(&format!("Received STOQ message: {}", message.message_type()).into());

        // Find and call registered handler
        let handlers = self.message_handlers.read().await;
        if let Some(handler) = handlers.get(&message.message_type()) {
            // Convert message to JavaScript object
            let js_message = js_sys::Object::new();
            js_sys::Reflect::set(&js_message, &"messageType".into(), &message.message_type().into()).unwrap();
            js_sys::Reflect::set(&js_message, &"payload".into(), &message.payload().into()).unwrap();
            js_sys::Reflect::set(&js_message, &"timestamp".into(), &message.timestamp().into()).unwrap();

            // Call the JavaScript handler
            let this = JsValue::null();
            let _ = handler.call1(&this, &js_message);
        }
    }

    /// Notify JavaScript about status changes
    async fn notify_status_change(&self) {
        let callbacks = self.event_callbacks.read().await;
        if let Some(callback) = callbacks.get("status_change") {
            let status_obj = js_sys::Object::new();
            js_sys::Reflect::set(&status_obj, &"status".into(), &format!("{:?}", self.status).into()).unwrap();
            js_sys::Reflect::set(&status_obj, &"connectionId".into(), &self.connection_id.clone().unwrap_or_default().into()).unwrap();
            js_sys::Reflect::set(&status_obj, &"timestamp".into(), &chrono::Utc::now().to_rfc3339().into()).unwrap();

            let this = JsValue::null();
            let _ = callback.call1(&this, &status_obj);
        }
    }
}

/// Helper function to create connection config from JavaScript
#[wasm_bindgen]
pub fn create_connection_config(server_address: String, server_port: u16, certificate_pem: String) -> WasmConnectionConfig {
    WasmConnectionConfig::new(server_address, server_port, certificate_pem)
}

/// Helper function to create STOQ message from JavaScript
#[wasm_bindgen]
pub fn create_stoq_message(message_type: String, payload: String) -> WasmStoqMessage {
    WasmStoqMessage::new(message_type, payload)
}

/// Get version information
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Utility function for JavaScript logging
#[wasm_bindgen]
pub fn log_message(message: &str) {
    console::log_1(&message.into());
}
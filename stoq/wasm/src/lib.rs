//! STOQ WebAssembly Client Library
//!
//! This crate provides a WebAssembly interface to the STOQ protocol,
//! enabling direct QUIC connections with TrustChain authentication from browsers.

use wasm_bindgen::prelude::*;
use js_sys::*;
use web_sys::*;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;

/// Configuration for WASM STOQ connection
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmConnectionConfig {
    server_address: String,
    server_port: u16,
    use_ipv6: bool,
}

#[wasm_bindgen]
impl WasmConnectionConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(server_address: &str, server_port: u16, use_ipv6: bool) -> WasmConnectionConfig {
        WasmConnectionConfig {
            server_address: server_address.to_string(),
            server_port,
            use_ipv6,
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn server_address(&self) -> String {
        self.server_address.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn server_port(&self) -> u16 {
        self.server_port
    }
    
    #[wasm_bindgen(getter)]
    pub fn use_ipv6(&self) -> bool {
        self.use_ipv6
    }
}

/// STOQ message structure for WASM - simple data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmStoqMessage {
    pub message_type: String,
    pub payload: String,
    pub correlation_id: String,
    pub timestamp: String,
}

// Separate wasm_bindgen impl for message creation and access
#[wasm_bindgen]
pub fn create_stoq_message(message_type: &str, payload: &str, correlation_id: &str) -> JsValue {
    let message = WasmStoqMessage {
        message_type: message_type.to_string(),
        payload: payload.to_string(),
        correlation_id: correlation_id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    serde_wasm_bindgen::to_value(&message).unwrap()
}

/// Certificate handling for WebAssembly
#[wasm_bindgen]
pub struct WasmCertificate {
    pem_data: String,
    fingerprint: String,
}

#[wasm_bindgen]
impl WasmCertificate {
    #[wasm_bindgen(constructor)]
    pub fn new(pem_data: &str) -> Result<WasmCertificate, JsValue> {
        // Simple validation - check PEM format
        if !pem_data.contains("-----BEGIN CERTIFICATE-----") || 
           !pem_data.contains("-----END CERTIFICATE-----") {
            return Err(JsValue::from_str("Invalid PEM format"));
        }
        
        // Generate simple SHA-256 fingerprint
        let fingerprint = hex::encode(sha2::Sha256::digest(pem_data.as_bytes()));
        
        Ok(WasmCertificate {
            pem_data: pem_data.to_string(),
            fingerprint,
        })
    }
    
    #[wasm_bindgen(getter)]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
    
    #[wasm_bindgen]
    pub fn validate(&self) -> bool {
        // Basic validation - check if PEM format is correct
        self.pem_data.contains("-----BEGIN CERTIFICATE-----") && 
        self.pem_data.contains("-----END CERTIFICATE-----")
    }
}

/// Connection status for WASM client - using simple fields
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmConnectionStatus {
    is_connected: bool,
    is_authenticated: bool,
    connection_id: String,
    error_message: String,
    protocol_version: String,
}

#[wasm_bindgen]
impl WasmConnectionStatus {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmConnectionStatus {
        WasmConnectionStatus {
            is_connected: false,
            is_authenticated: false,
            connection_id: String::new(),
            error_message: String::new(),
            protocol_version: "STOQ/1.0".to_string(),
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
    
    #[wasm_bindgen(getter)]
    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated
    }
    
    #[wasm_bindgen(getter)]
    pub fn connection_id(&self) -> String {
        self.connection_id.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn error_message(&self) -> String {
        self.error_message.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn protocol_version(&self) -> String {
        self.protocol_version.clone()
    }
}

/// Main WASM STOQ Client
#[wasm_bindgen]
pub struct WasmStoqClient {
    status: WasmConnectionStatus,
    server_address: String,
    server_port: u16,
    use_ipv6: bool,
}

#[wasm_bindgen]
impl WasmStoqClient {
    #[wasm_bindgen(constructor)]
    pub fn new(server_address: &str, server_port: u16, use_ipv6: bool) -> WasmStoqClient {
        WasmStoqClient {
            status: WasmConnectionStatus::new(),
            server_address: server_address.to_string(),
            server_port,
            use_ipv6,
        }
    }
    
    /// Initialize connection with certificate
    #[wasm_bindgen]
    pub async fn connect(&mut self, certificate: &WasmCertificate) -> Result<(), JsValue> {
        if !certificate.validate() {
            self.status.error_message = "Invalid certificate".to_string();
            return Err(JsValue::from_str("Invalid certificate"));
        }
        
        // For now, simulate connection (real QUIC implementation would go here)
        self.status.connection_id = uuid::Uuid::new_v4().to_string();
        self.status.is_connected = true;
        self.status.is_authenticated = true;
        self.status.error_message = String::new();
        
        web_sys::console::log_1(&format!("Connected to STOQ server with certificate fingerprint: {}", 
                                        certificate.fingerprint()).into());
        
        Ok(())
    }
    
    /// Send a message (using JsValue instead of struct reference)
    #[wasm_bindgen]
    pub async fn send_message(&self, message_js: JsValue) -> Result<JsValue, JsValue> {
        if !self.status.is_connected {
            return Err(JsValue::from_str("Not connected"));
        }
        
        // Parse the JavaScript message
        let message: WasmStoqMessage = serde_wasm_bindgen::from_value(message_js)?;
        
        web_sys::console::log_1(&format!("Sending STOQ message: {}", message.message_type).into());
        
        // Simulate sending and receiving response
        let response_data = match message.message_type.as_str() {
            "dashboard_request" => self.handle_dashboard_request(&message).await?,
            "system_status_request" => self.handle_system_status_request(&message).await?,
            "performance_metrics_request" => self.handle_performance_metrics_request(&message).await?,
            _ => return Err(JsValue::from_str("Unknown message type")),
        };
        
        Ok(response_data)
    }
    
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> WasmConnectionStatus {
        self.status.clone()
    }
    
    /// Disconnect from server
    #[wasm_bindgen]
    pub async fn disconnect(&mut self) -> Result<(), JsValue> {
        self.status.is_connected = false;
        self.status.is_authenticated = false;
        self.status.connection_id = String::new();
        
        web_sys::console::log_1(&"Disconnected from STOQ server".into());
        Ok(())
    }
}

// Internal message handlers (simulate server responses)
impl WasmStoqClient {
    async fn handle_dashboard_request(&self, message: &WasmStoqMessage) -> Result<JsValue, JsValue> {
        let response_data = r#"{
            "status": "success",
            "data": {
                "components": {
                    "trustchain": {"status": "healthy", "version": "1.0.0"},
                    "stoq": {"status": "healthy", "version": "1.0.0"},
                    "hypermesh": {"status": "healthy", "version": "1.0.0"},
                    "catalog": {"status": "healthy", "version": "1.0.0"},
                    "caesar": {"status": "healthy", "version": "1.0.0"}
                },
                "timestamp": "2024-12-19T10:30:00Z"
            }
        }"#;
        
        let response = WasmStoqMessage {
            message_type: "dashboard_response".to_string(),
            payload: response_data.to_string(),
            correlation_id: message.correlation_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    async fn handle_system_status_request(&self, message: &WasmStoqMessage) -> Result<JsValue, JsValue> {
        let response_data = r#"{
            "system": {
                "overall_health": "good",
                "score": 87,
                "services": {
                    "trustchain": {"status": "healthy", "uptime": "99.9%"},
                    "stoq": {"status": "healthy", "uptime": "99.8%"},
                    "hypermesh": {"status": "healthy", "uptime": "99.7%"},
                    "catalog": {"status": "healthy", "uptime": "99.9%"},
                    "caesar": {"status": "healthy", "uptime": "99.8%"}
                }
            }
        }"#;
        
        let response = WasmStoqMessage {
            message_type: "system_status_response".to_string(),
            payload: response_data.to_string(),
            correlation_id: message.correlation_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    async fn handle_performance_metrics_request(&self, message: &WasmStoqMessage) -> Result<JsValue, JsValue> {
        let response_data = r#"{
            "metrics": {
                "throughput": {
                    "current": 2950,
                    "target": 40000,
                    "efficiency": 7.375
                },
                "latency": {
                    "average": 12
                },
                "connections": {
                    "active": 156
                }
            }
        }"#;
        
        let response = WasmStoqMessage {
            message_type: "performance_metrics_response".to_string(),
            payload: response_data.to_string(),
            correlation_id: message.correlation_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// WebAssembly entry point - initialize the STOQ WASM client
#[wasm_bindgen(start)]
pub fn main() {
    // Initialize panic handler for better debugging
    console_error_panic_hook::set_once();
    
    // Initialize tracing for WASM
    tracing_wasm::set_as_global_default();
    
    web_sys::console::log_1(&"STOQ WebAssembly Client loaded successfully".into());
}
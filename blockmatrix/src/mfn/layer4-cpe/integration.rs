//! Layer Integration System
//!
//! This module implements integration interfaces with other MFN layers and
//! the HyperMesh transport layer for seamless data flow and coordination.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc, broadcast};
use tracing::{debug, info, warn, error};

use crate::{ContextVector, FlowKey};
use crate::prediction::PredictionResult;

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub enable_layer2_feedback: bool,
    pub enable_layer3_routing: bool,
    pub enable_hypermesh_metrics: bool,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enable_layer2_feedback: true,
            enable_layer3_routing: true,
            enable_hypermesh_metrics: true,
        }
    }
}

/// Messages from Layer 2 (DSR) neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Layer2Message {
    /// Similarity detection result for context learning
    SimilarityResult {
        flow_key: FlowKey,
        similarity_score: f32,
        pattern_id: String,
        confidence: f32,
    },
    /// Neural network adaptation feedback
    AdaptationFeedback {
        network_id: String,
        performance_delta: f32,
        suggested_learning_rate: f32,
    },
    /// Spiking neural network state
    SpikingState {
        neuron_activations: Vec<f32>,
        spike_events: Vec<u64>,
        pattern_matches: Vec<String>,
    },
}

/// Messages to Layer 3 (ALM) for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Layer3Message {
    /// Context prediction for routing optimization
    RoutingPrediction {
        flow_key: FlowKey,
        predicted_context: Vec<f32>,
        confidence: f32,
        time_horizon_ms: u64,
        routing_suggestions: Vec<RoutingSuggestion>,
    },
    /// Load balancing recommendations
    LoadBalancingUpdate {
        service_weights: HashMap<String, f32>,
        predicted_loads: HashMap<String, f32>,
        adaptation_confidence: f32,
    },
    /// Circuit breaker state predictions
    CircuitBreakerPrediction {
        service_id: String,
        failure_probability: f32,
        recommended_threshold: f32,
        time_to_recovery_ms: u64,
    },
}

/// Routing suggestion for Layer 3 ALM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSuggestion {
    pub route_id: String,
    pub expected_latency_ms: f32,
    pub expected_throughput: f32,
    pub reliability_score: f32,
    pub cost_factor: f32,
}

/// Messages from HyperMesh transport layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperMeshMessage {
    /// Transport performance metrics
    TransportMetrics {
        connection_count: u64,
        average_latency_ms: f32,
        throughput_mbps: f32,
        error_rate: f32,
    },
    /// QUIC connection events
    ConnectionEvent {
        event_type: String,
        connection_id: String,
        timestamp: u64,
        metadata: HashMap<String, f32>,
    },
    /// IPv6 network topology changes
    TopologyUpdate {
        node_additions: Vec<String>,
        node_removals: Vec<String>,
        link_changes: HashMap<String, f32>,
    },
}

/// Integration events for internal coordination
#[derive(Debug, Clone)]
pub enum IntegrationEvent {
    Layer2Updated(Layer2Message),
    Layer3RequestReceived(String), // Request ID
    HyperMeshEvent(HyperMeshMessage),
    PredictionCompleted { flow_key: FlowKey, result: PredictionResult },
    IntegrationError { source: String, error: String },
}

/// Statistics for integration performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStats {
    pub layer2_messages_received: u64,
    pub layer3_messages_sent: u64,
    pub hypermesh_events_processed: u64,
    pub integration_errors: u64,
    pub average_processing_time_ms: f64,
    pub message_queue_size: usize,
}

/// Main layer connector for coordinating with other components
pub struct LayerConnector {
    config: IntegrationConfig,
    
    // Communication channels
    layer2_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<Layer2Message>>>>,
    layer3_sender: Arc<RwLock<Option<mpsc::UnboundedSender<Layer3Message>>>>,
    hypermesh_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<HyperMeshMessage>>>>,
    
    // Event broadcasting
    event_broadcaster: broadcast::Sender<IntegrationEvent>,
    event_receiver: Arc<RwLock<Option<broadcast::Receiver<IntegrationEvent>>>>,
    
    // Integration state
    active_connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    message_buffer: Arc<RwLock<MessageBuffer>>,
    
    // Statistics
    stats: Arc<RwLock<IntegrationStats>>,
    processing_times: Arc<RwLock<Vec<Duration>>>,
}

/// Connection state tracking
#[derive(Debug, Clone)]
struct ConnectionState {
    connection_id: String,
    layer_type: String,
    established_at: Instant,
    last_message_at: Instant,
    message_count: u64,
    health_status: ConnectionHealth,
}

/// Connection health status
#[derive(Debug, Clone, PartialEq)]
enum ConnectionHealth {
    Healthy,
    Degraded,
    Failed,
}

/// Message buffer for handling bursts and ordering
struct MessageBuffer {
    layer2_buffer: Vec<Layer2Message>,
    hypermesh_buffer: Vec<HyperMeshMessage>,
    max_buffer_size: usize,
    processing_batch_size: usize,
}

impl LayerConnector {
    /// Create a new layer connector
    pub async fn new(config: IntegrationConfig) -> Result<Self> {
        info!("Initializing LayerConnector with config: {:?}", config);
        
        let (event_tx, event_rx) = broadcast::channel(1000);
        
        let connector = Self {
            config,
            layer2_receiver: Arc::new(RwLock::new(None)),
            layer3_sender: Arc::new(RwLock::new(None)),
            hypermesh_receiver: Arc::new(RwLock::new(None)),
            event_broadcaster: event_tx,
            event_receiver: Arc::new(RwLock::new(Some(event_rx))),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            message_buffer: Arc::new(RwLock::new(MessageBuffer {
                layer2_buffer: Vec::new(),
                hypermesh_buffer: Vec::new(),
                max_buffer_size: 10000,
                processing_batch_size: 100,
            })),
            stats: Arc::new(RwLock::new(IntegrationStats {
                layer2_messages_received: 0,
                layer3_messages_sent: 0,
                hypermesh_events_processed: 0,
                integration_errors: 0,
                average_processing_time_ms: 0.0,
                message_queue_size: 0,
            })),
            processing_times: Arc::new(RwLock::new(Vec::new())),
        };
        
        Ok(connector)
    }
    
    /// Initialize connections to other layers
    pub async fn initialize_connections(&mut self) -> Result<()> {
        info!("Initializing layer connections");
        
        if self.config.enable_layer2_feedback {
            self.setup_layer2_connection().await?;
        }
        
        if self.config.enable_layer3_routing {
            self.setup_layer3_connection().await?;
        }
        
        if self.config.enable_hypermesh_metrics {
            self.setup_hypermesh_connection().await?;
        }
        
        // Start background processing task
        self.start_background_processing().await;
        
        info!("Layer connections initialized successfully");
        Ok(())
    }
    
    /// Setup Layer 2 DSR connection
    async fn setup_layer2_connection(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.layer2_receiver.write().await = Some(rx);
        
        // In a real implementation, this would establish actual IPC/network connection
        // For now, we simulate the connection setup
        
        let connection_state = ConnectionState {
            connection_id: "layer2_dsr".to_string(),
            layer_type: "DSR".to_string(),
            established_at: Instant::now(),
            last_message_at: Instant::now(),
            message_count: 0,
            health_status: ConnectionHealth::Healthy,
        };
        
        self.active_connections.write().await.insert(
            "layer2_dsr".to_string(),
            connection_state,
        );
        
        debug!("Layer 2 DSR connection established");
        Ok(())
    }
    
    /// Setup Layer 3 ALM connection
    async fn setup_layer3_connection(&mut self) -> Result<()> {
        let (tx, _rx) = mpsc::unbounded_channel();
        *self.layer3_sender.write().await = Some(tx);
        
        let connection_state = ConnectionState {
            connection_id: "layer3_alm".to_string(),
            layer_type: "ALM".to_string(),
            established_at: Instant::now(),
            last_message_at: Instant::now(),
            message_count: 0,
            health_status: ConnectionHealth::Healthy,
        };
        
        self.active_connections.write().await.insert(
            "layer3_alm".to_string(),
            connection_state,
        );
        
        debug!("Layer 3 ALM connection established");
        Ok(())
    }
    
    /// Setup HyperMesh transport connection
    async fn setup_hypermesh_connection(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.hypermesh_receiver.write().await = Some(rx);
        
        let connection_state = ConnectionState {
            connection_id: "hypermesh_transport".to_string(),
            layer_type: "TRANSPORT".to_string(),
            established_at: Instant::now(),
            last_message_at: Instant::now(),
            message_count: 0,
            health_status: ConnectionHealth::Healthy,
        };
        
        self.active_connections.write().await.insert(
            "hypermesh_transport".to_string(),
            connection_state,
        );
        
        debug!("HyperMesh transport connection established");
        Ok(())
    }
    
    /// Send routing prediction to Layer 3 ALM
    pub async fn send_routing_prediction(
        &self,
        flow_key: FlowKey,
        prediction: &PredictionResult,
        routing_suggestions: Vec<RoutingSuggestion>,
    ) -> Result<()> {
        if !self.config.enable_layer3_routing {
            return Ok(());
        }
        
        let message = Layer3Message::RoutingPrediction {
            flow_key,
            predicted_context: prediction.predicted_context.clone(),
            confidence: prediction.confidence,
            time_horizon_ms: prediction.prediction_horizon as u64 * 1000,
            routing_suggestions,
        };
        
        let sender = self.layer3_sender.read().await;
        if let Some(ref tx) = *sender {
            if let Err(e) = tx.send(message) {
                error!("Failed to send routing prediction to Layer 3: {}", e);
                return Err(anyhow::anyhow!("Layer 3 send error: {}", e));
            }
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.layer3_messages_sent += 1;
            
            debug!("Routing prediction sent to Layer 3 ALM");
        } else {
            warn!("Layer 3 sender not available");
        }
        
        Ok(())
    }
    
    /// Send load balancing update to Layer 3
    pub async fn send_load_balancing_update(
        &self,
        service_weights: HashMap<String, f32>,
        predicted_loads: HashMap<String, f32>,
        confidence: f32,
    ) -> Result<()> {
        if !self.config.enable_layer3_routing {
            return Ok(());
        }
        
        let message = Layer3Message::LoadBalancingUpdate {
            service_weights,
            predicted_loads,
            adaptation_confidence: confidence,
        };
        
        let sender = self.layer3_sender.read().await;
        if let Some(ref tx) = *sender {
            tx.send(message).map_err(|e| anyhow::anyhow!("Send error: {}", e))?;
            
            debug!("Load balancing update sent to Layer 3 ALM");
        }
        
        Ok(())
    }
    
    /// Process messages from Layer 2 DSR
    pub async fn process_layer2_messages(&self) -> Result<Vec<Layer2Message>> {
        let mut messages = Vec::new();
        
        let mut receiver_guard = self.layer2_receiver.write().await;
        if let Some(ref mut rx) = *receiver_guard {
            // Process available messages
            while let Ok(message) = rx.try_recv() {
                messages.push(message);
                
                if messages.len() >= 100 { // Batch limit
                    break;
                }
            }
        }
        
        if !messages.is_empty() {
            // Update connection state
            self.update_connection_activity("layer2_dsr", messages.len() as u64).await;
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.layer2_messages_received += messages.len() as u64;
            
            debug!("Processed {} messages from Layer 2 DSR", messages.len());
        }
        
        Ok(messages)
    }
    
    /// Process messages from HyperMesh transport
    pub async fn process_hypermesh_messages(&self) -> Result<Vec<HyperMeshMessage>> {
        let mut messages = Vec::new();
        
        let mut receiver_guard = self.hypermesh_receiver.write().await;
        if let Some(ref mut rx) = *receiver_guard {
            while let Ok(message) = rx.try_recv() {
                messages.push(message);
                
                if messages.len() >= 100 {
                    break;
                }
            }
        }
        
        if !messages.is_empty() {
            self.update_connection_activity("hypermesh_transport", messages.len() as u64).await;
            
            let mut stats = self.stats.write().await;
            stats.hypermesh_events_processed += messages.len() as u64;
            
            debug!("Processed {} messages from HyperMesh transport", messages.len());
        }
        
        Ok(messages)
    }
    
    /// Update connection activity timestamp
    async fn update_connection_activity(&self, connection_id: &str, message_count: u64) {
        let mut connections = self.active_connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.last_message_at = Instant::now();
            connection.message_count += message_count;
            
            // Update health status based on activity
            let idle_time = connection.last_message_at.elapsed();
            connection.health_status = if idle_time > Duration::from_secs(300) {
                ConnectionHealth::Failed
            } else if idle_time > Duration::from_secs(60) {
                ConnectionHealth::Degraded
            } else {
                ConnectionHealth::Healthy
            };
        }
    }
    
    /// Start background processing task
    async fn start_background_processing(&self) {
        let layer2_receiver = self.layer2_receiver.clone();
        let hypermesh_receiver = self.hypermesh_receiver.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let stats = self.stats.clone();
        let processing_times = self.processing_times.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                let start_time = Instant::now();
                
                // Process Layer 2 messages
                {
                    let mut receiver_guard = layer2_receiver.write().await;
                    if let Some(ref mut rx) = *receiver_guard {
                        while let Ok(message) = rx.try_recv() {
                            let event = IntegrationEvent::Layer2Updated(message);
                            let _ = event_broadcaster.send(event);
                        }
                    }
                }
                
                // Process HyperMesh messages
                {
                    let mut receiver_guard = hypermesh_receiver.write().await;
                    if let Some(ref mut rx) = *receiver_guard {
                        while let Ok(message) = rx.try_recv() {
                            let event = IntegrationEvent::HyperMeshEvent(message);
                            let _ = event_broadcaster.send(event);
                        }
                    }
                }
                
                // Update processing time statistics
                let processing_time = start_time.elapsed();
                {
                    let mut times = processing_times.write().await;
                    times.push(processing_time);
                    
                    if times.len() > 1000 {
                        times.remove(0);
                    }
                    
                    // Update average processing time
                    let avg_time = times.iter().sum::<Duration>().as_secs_f64() / times.len() as f64;
                    stats.write().await.average_processing_time_ms = avg_time * 1000.0;
                }
            }
        });
        
        debug!("Background processing task started");
    }
    
    /// Subscribe to integration events
    pub async fn subscribe_to_events(&self) -> broadcast::Receiver<IntegrationEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Get connection health status
    pub async fn get_connection_health(&self) -> HashMap<String, ConnectionHealth> {
        let connections = self.active_connections.read().await;
        connections.iter()
            .map(|(id, state)| (id.clone(), state.health_status.clone()))
            .collect()
    }
    
    /// Get integration statistics
    pub async fn get_statistics(&self) -> IntegrationStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Handle integration errors
    pub async fn handle_integration_error(&self, source: &str, error: &str) {
        error!("Integration error from {}: {}", source, error);
        
        let event = IntegrationEvent::IntegrationError {
            source: source.to_string(),
            error: error.to_string(),
        };
        
        let _ = self.event_broadcaster.send(event);
        
        let mut stats = self.stats.write().await;
        stats.integration_errors += 1;
    }
    
    /// Create routing suggestions based on prediction
    pub fn create_routing_suggestions(
        &self,
        prediction: &PredictionResult,
        context: &ContextVector,
    ) -> Vec<RoutingSuggestion> {
        let mut suggestions = Vec::new();
        
        // Analyze predicted context to generate routing suggestions
        let predicted_load = prediction.predicted_context.iter().sum::<f32>() / prediction.predicted_context.len() as f32;
        
        // Generate different routing options based on predicted characteristics
        if predicted_load < 0.3 {
            suggestions.push(RoutingSuggestion {
                route_id: "fast_path".to_string(),
                expected_latency_ms: 1.0,
                expected_throughput: 10000.0,
                reliability_score: 0.99,
                cost_factor: 1.2,
            });
        }
        
        if predicted_load < 0.7 {
            suggestions.push(RoutingSuggestion {
                route_id: "balanced_path".to_string(),
                expected_latency_ms: 2.5,
                expected_throughput: 7000.0,
                reliability_score: 0.95,
                cost_factor: 1.0,
            });
        }
        
        suggestions.push(RoutingSuggestion {
            route_id: "high_capacity_path".to_string(),
            expected_latency_ms: 5.0,
            expected_throughput: 15000.0,
            reliability_score: 0.98,
            cost_factor: 0.8,
        });
        
        // Add fault-tolerant option if context suggests reliability concerns
        if let Some(&reliability_concern) = context.metadata.get("reliability_concern") {
            if reliability_concern > 0.5 {
                suggestions.push(RoutingSuggestion {
                    route_id: "fault_tolerant_path".to_string(),
                    expected_latency_ms: 8.0,
                    expected_throughput: 5000.0,
                    reliability_score: 0.999,
                    cost_factor: 1.5,
                });
            }
        }
        
        suggestions
    }
    
    /// Shutdown all connections gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down layer connections");
        
        // Close all channels
        *self.layer2_receiver.write().await = None;
        *self.layer3_sender.write().await = None;
        *self.hypermesh_receiver.write().await = None;
        
        // Clear connection states
        self.active_connections.write().await.clear();
        
        info!("Layer connections shut down successfully");
        Ok(())
    }
}

/// Utility functions for message serialization and transport
pub mod transport_utils {
    use super::*;
    use serde_json;
    
    /// Serialize message for transport
    pub fn serialize_layer3_message(message: &Layer3Message) -> Result<Vec<u8>> {
        serde_json::to_vec(message)
            .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))
    }
    
    /// Deserialize Layer 2 message from transport
    pub fn deserialize_layer2_message(data: &[u8]) -> Result<Layer2Message> {
        serde_json::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Deserialization error: {}", e))
    }
    
    /// Deserialize HyperMesh message from transport
    pub fn deserialize_hypermesh_message(data: &[u8]) -> Result<HyperMeshMessage> {
        serde_json::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Deserialization error: {}", e))
    }
    
    /// Create message header with metadata
    pub fn create_message_header(
        message_type: &str,
        source: &str,
        timestamp: u64,
    ) -> HashMap<String, String> {
        let mut header = HashMap::new();
        header.insert("message_type".to_string(), message_type.to_string());
        header.insert("source".to_string(), source.to_string());
        header.insert("timestamp".to_string(), timestamp.to_string());
        header.insert("version".to_string(), "1.0".to_string());
        header
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_layer_connector_creation() {
        let config = IntegrationConfig::default();
        let connector = LayerConnector::new(config).await;
        assert!(connector.is_ok());
    }
    
    #[tokio::test]
    async fn test_routing_suggestions() {
        let config = IntegrationConfig::default();
        let connector = LayerConnector::new(config).await.unwrap();
        
        let prediction = PredictionResult::new(vec![0.2, 0.3, 0.4], 0.8);
        let context = ContextVector::new([1u8; 32], vec![0.1, 0.2, 0.3]);
        
        let suggestions = connector.create_routing_suggestions(&prediction, &context);
        assert!(!suggestions.is_empty());
        
        // Should include fast path for low predicted load
        assert!(suggestions.iter().any(|s| s.route_id == "fast_path"));
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let config = IntegrationConfig::default();
        let connector = LayerConnector::new(config).await.unwrap();
        
        let mut event_receiver = connector.subscribe_to_events().await;
        
        // Send test event
        let test_event = IntegrationEvent::IntegrationError {
            source: "test".to_string(),
            error: "test error".to_string(),
        };
        
        let _ = connector.event_broadcaster.send(test_event);
        
        // Receive event
        if let Ok(received_event) = event_receiver.try_recv() {
            match received_event {
                IntegrationEvent::IntegrationError { source, error } => {
                    assert_eq!(source, "test");
                    assert_eq!(error, "test error");
                }
                _ => panic!("Unexpected event type"),
            }
        }
    }
    
    #[tokio::test]
    async fn test_message_serialization() {
        let message = Layer3Message::RoutingPrediction {
            flow_key: [1u8; 32],
            predicted_context: vec![0.1, 0.2, 0.3],
            confidence: 0.8,
            time_horizon_ms: 5000,
            routing_suggestions: vec![],
        };
        
        let serialized = transport_utils::serialize_layer3_message(&message);
        assert!(serialized.is_ok());
        
        let data = serialized.unwrap();
        assert!(!data.is_empty());
    }
    
    #[test]
    fn test_connection_health_transitions() {
        let mut connection = ConnectionState {
            connection_id: "test".to_string(),
            layer_type: "TEST".to_string(),
            established_at: Instant::now(),
            last_message_at: Instant::now() - Duration::from_secs(400),
            message_count: 0,
            health_status: ConnectionHealth::Healthy,
        };
        
        // Simulate health check logic
        let idle_time = connection.last_message_at.elapsed();
        if idle_time > Duration::from_secs(300) {
            connection.health_status = ConnectionHealth::Failed;
        }
        
        assert_eq!(connection.health_status, ConnectionHealth::Failed);
    }
}
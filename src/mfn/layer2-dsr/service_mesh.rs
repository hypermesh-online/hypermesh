//! Service Mesh Intelligence with Neural Predictions
//!
//! Provides intelligent load balancing, circuit breaking, and traffic analysis
//! using the spiking neural network for enhanced service mesh operations.

use crate::network::NeuralNetwork;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

/// Load balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round robin distribution
    RoundRobin,
    /// Weighted round robin based on capacity
    WeightedRoundRobin,
    /// Least connections strategy
    LeastConnections,
    /// Neural network predicted optimal distribution
    NeuralOptimal,
    /// Consistent hashing for sticky sessions
    ConsistentHashing,
    /// Custom weighted distribution
    Custom { weights: HashMap<String, f64> },
}

/// Circuit breaker state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitBreakerState {
    Closed,    // Normal operation
    Open,      // Failing, blocking requests
    HalfOpen,  // Testing recovery
}

/// Service mesh recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub service: String,
    pub recommendation_type: RecommendationType,
    pub confidence: f64,
    pub expected_improvement: f64,
    pub description: String,
    pub priority: u8, // 1 = highest, 10 = lowest
}

/// Type of service mesh recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    ScaleUp { target_instances: usize },
    ScaleDown { target_instances: usize },
    LoadBalanceAdjust { new_strategy: LoadBalancingStrategy },
    CircuitBreakerTune { failure_threshold: f64, recovery_timeout: Duration },
    TrafficShift { percentage: f64, target_service: String },
    RetryPolicyUpdate { max_retries: usize, backoff_strategy: BackoffStrategy },
    TimeoutAdjust { new_timeout_ms: u64 },
    RateLimitUpdate { requests_per_second: f64 },
}

/// Retry backoff strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Linear { increment_ms: u64 },
    Exponential { base_ms: u64, multiplier: f64 },
    Fixed { delay_ms: u64 },
}

/// Service performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub request_rate: f64,          // requests per second
    pub response_time_p50: f64,     // median response time (ms)
    pub response_time_p95: f64,     // 95th percentile response time (ms)
    pub response_time_p99: f64,     // 99th percentile response time (ms)
    pub error_rate: f64,            // error percentage
    pub active_connections: usize,  // concurrent connections
    pub cpu_utilization: f64,       // CPU usage percentage
    pub memory_utilization: f64,    // Memory usage percentage
    pub throughput_mbps: f64,       // Network throughput
    pub success_rate: f64,          // Success percentage
    pub timestamp: SystemTime,
}

/// Circuit breaker configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub service_name: String,
    pub state: CircuitBreakerState,
    pub failure_threshold: f64,     // Error rate threshold (0.0-1.0)
    pub recovery_timeout: Duration, // Time to wait before testing recovery
    pub failure_count: usize,       // Recent failure count
    pub success_count: usize,       // Recent success count
    pub last_failure_time: Option<SystemTime>,
    pub last_state_change: SystemTime,
    /// Neural prediction for recovery success
    pub recovery_probability: f64,
}

/// Load balancer with neural intelligence
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    pub service_name: String,
    pub strategy: LoadBalancingStrategy,
    pub endpoints: Vec<ServiceEndpoint>,
    pub current_weights: HashMap<String, f64>,
    pub request_count: u64,
    /// Neural predictions for endpoint performance
    pub endpoint_predictions: HashMap<String, EndpointPrediction>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub weight: f64,
    pub health_status: HealthStatus,
    pub current_connections: usize,
    pub recent_response_time: f64,
    pub recent_error_rate: f64,
}

/// Health status of service endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Neural prediction for endpoint performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointPrediction {
    pub expected_response_time: f64,
    pub expected_success_rate: f64,
    pub expected_throughput: f64,
    pub confidence: f64,
    pub last_updated: SystemTime,
}

/// Service mesh intelligence engine
pub struct ServiceMeshIntelligence {
    neural_network: Arc<RwLock<NeuralNetwork>>,
    /// Circuit breakers for services
    circuit_breakers: HashMap<String, CircuitBreaker>,
    /// Load balancers for services  
    load_balancers: HashMap<String, LoadBalancer>,
    /// Historical metrics for learning
    metrics_history: HashMap<String, VecDeque<ServiceMetrics>>,
    /// Traffic patterns for prediction
    traffic_patterns: HashMap<String, TrafficPattern>,
    /// Service dependencies graph
    dependency_graph: HashMap<String, Vec<String>>,
    /// Recommendation cache
    recommendation_cache: HashMap<String, (Vec<Recommendation>, Instant)>,
    /// Statistics
    total_requests: u64,
    circuit_breaker_activations: u64,
    load_balance_decisions: u64,
    neural_predictions: u64,
}

/// Traffic pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrafficPattern {
    pub service_name: String,
    pub hourly_patterns: Vec<f64>,      // 24-hour pattern
    pub weekly_patterns: Vec<f64>,      // 7-day pattern
    pub seasonal_multiplier: f64,       // Seasonal adjustment
    pub trend_coefficient: f64,         // Long-term trend
    pub last_updated: SystemTime,
}

impl ServiceMeshIntelligence {
    pub async fn new(neural_network: Arc<RwLock<NeuralNetwork>>) -> Result<Self> {
        Ok(Self {
            neural_network,
            circuit_breakers: HashMap::new(),
            load_balancers: HashMap::new(),
            metrics_history: HashMap::new(),
            traffic_patterns: HashMap::new(),
            dependency_graph: HashMap::new(),
            recommendation_cache: HashMap::new(),
            total_requests: 0,
            circuit_breaker_activations: 0,
            load_balance_decisions: 0,
            neural_predictions: 0,
        })
    }
    
    /// Analyze service metrics and generate recommendations
    pub async fn analyze_and_recommend(&self,
        service_metrics: &HashMap<String, f64>
    ) -> Result<Vec<Recommendation>> {
        info!("Analyzing {} services for recommendations", service_metrics.len());
        
        let mut recommendations = Vec::new();
        
        // Group metrics by service
        let services = self.extract_services_from_metrics(service_metrics);
        
        for (service_name, metrics) in &services {
            // Check cache first
            let cache_key = format!("{}_{}", service_name, self.compute_metrics_hash(metrics));
            if let Some((cached_recs, cache_time)) = self.recommendation_cache.get(&cache_key) {
                if cache_time.elapsed() < Duration::from_secs(60) { // 1 minute cache
                    recommendations.extend(cached_recs.clone());
                    continue;
                }
            }
            
            // Get neural predictions for service
            let neural_predictions = self.predict_service_performance(service_name, metrics).await?;
            
            // Generate recommendations based on metrics and predictions
            let service_recs = self.generate_service_recommendations(
                service_name,
                metrics,
                &neural_predictions
            ).await?;
            
            recommendations.extend(service_recs);
        }
        
        // Sort by priority and confidence
        recommendations.sort_by(|a, b| {
            a.priority.cmp(&b.priority)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap())
        });
        
        // Limit to top recommendations
        recommendations.truncate(20);
        
        info!("Generated {} recommendations", recommendations.len());
        Ok(recommendations)
    }
    
    /// Extract service metrics grouped by service name
    fn extract_services_from_metrics(&self,
        metrics: &HashMap<String, f64>
    ) -> HashMap<String, HashMap<String, f64>> {
        let mut services = HashMap::new();
        
        for (key, value) in metrics {
            if let Some(underscore_pos) = key.rfind('_') {
                let metric_name = &key[..underscore_pos];
                let service_name = &key[underscore_pos + 1..];
                
                services.entry(service_name.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(metric_name.to_string(), *value);
            }
        }
        
        services
    }
    
    /// Predict service performance using neural network
    async fn predict_service_performance(&self,
        service_name: &str,
        metrics: &HashMap<String, f64>
    ) -> Result<ServicePrediction> {
        // Encode service metrics as neural network input
        let input = self.encode_service_metrics(metrics);
        
        // Get neural network prediction
        let output = {
            let mut network = self.neural_network.write().await;
            network.process_input(&input, None).await?
        };
        
        // Decode prediction
        let prediction = self.decode_service_prediction(&output);
        
        trace!("Neural prediction for {}: response_time={:.2}ms, success_rate={:.3}",
               service_name, prediction.predicted_response_time, prediction.predicted_success_rate);
        
        Ok(prediction)
    }
    
    /// Encode service metrics for neural network
    fn encode_service_metrics(&self, metrics: &HashMap<String, f64>) -> Vec<f64> {
        let mut input = Vec::with_capacity(64);
        
        // Standard service metrics
        let metric_keys = [
            "request_rate", "response_time_p50", "response_time_p95", "response_time_p99",
            "error_rate", "active_connections", "cpu_utilization", "memory_utilization",
            "throughput_mbps", "success_rate", "queue_depth", "cache_hit_rate"
        ];
        
        for key in &metric_keys {
            let value = metrics.get(*key).cloned().unwrap_or(0.0);
            let normalized = self.normalize_service_metric(key, value);
            input.push(normalized);
        }
        
        // Add derived metrics
        let load_factor = metrics.get("cpu_utilization").unwrap_or(&0.0) * 0.6 + 
                         metrics.get("memory_utilization").unwrap_or(&0.0) * 0.4;
        input.push(load_factor / 100.0);
        
        let response_time_variance = metrics.get("response_time_p99").unwrap_or(&0.0) - 
                                   metrics.get("response_time_p50").unwrap_or(&0.0);
        input.push((response_time_variance / 1000.0).min(1.0).max(0.0));
        
        // Pad to fixed size
        while input.len() < 64 {
            input.push(0.0);
        }
        
        input.truncate(64);
        input
    }
    
    /// Normalize service metric for neural network
    fn normalize_service_metric(&self, metric_name: &str, value: f64) -> f64 {
        match metric_name {
            "request_rate" => (value / 1000.0).min(1.0).max(0.0), // Cap at 1000 RPS
            "response_time_p50" | "response_time_p95" | "response_time_p99" => {
                (value / 1000.0).min(1.0).max(0.0) // Cap at 1 second
            },
            "error_rate" => (value / 100.0).min(1.0).max(0.0), // Percentage
            "active_connections" => (value / 10000.0).min(1.0).max(0.0), // Cap at 10k connections
            "cpu_utilization" | "memory_utilization" => (value / 100.0).min(1.0).max(0.0),
            "throughput_mbps" => (value / 10000.0).min(1.0).max(0.0), // Cap at 10 Gbps
            "success_rate" | "cache_hit_rate" => (value / 100.0).min(1.0).max(0.0),
            _ => value.min(1.0).max(0.0),
        }
    }
    
    /// Decode neural network output to service prediction
    fn decode_service_prediction(&self, output: &[f64]) -> ServicePrediction {
        let predicted_response_time = output.get(0).cloned().unwrap_or(0.0) * 1000.0; // Convert to ms
        let predicted_success_rate = output.get(1).cloned().unwrap_or(0.95);
        let predicted_throughput = output.get(2).cloned().unwrap_or(0.5) * 1000.0; // Convert to Mbps
        let predicted_cpu_usage = output.get(3).cloned().unwrap_or(0.5) * 100.0;
        
        // Calculate confidence from output consistency
        let confidence = if output.len() >= 4 {
            let variance = output[0..4].iter()
                .map(|&x| (x - 0.5).powi(2))
                .sum::<f64>() / 4.0;
            1.0 - variance.sqrt()
        } else {
            0.5
        }.max(0.0).min(1.0);
        
        ServicePrediction {
            predicted_response_time,
            predicted_success_rate,
            predicted_throughput,
            predicted_cpu_usage,
            confidence,
        }
    }
    
    /// Generate recommendations for a specific service
    async fn generate_service_recommendations(&self,
        service_name: &str,
        metrics: &HashMap<String, f64>,
        prediction: &ServicePrediction
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();
        
        let response_time = metrics.get("response_time_p95").cloned().unwrap_or(0.0);
        let error_rate = metrics.get("error_rate").cloned().unwrap_or(0.0);
        let cpu_util = metrics.get("cpu_utilization").cloned().unwrap_or(0.0);
        let memory_util = metrics.get("memory_utilization").cloned().unwrap_or(0.0);
        let success_rate = metrics.get("success_rate").cloned().unwrap_or(100.0);
        
        // Scaling recommendations
        if cpu_util > 80.0 || memory_util > 85.0 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::ScaleUp { target_instances: 2 },
                confidence: 0.85,
                expected_improvement: 0.3,
                description: format!("High resource utilization detected (CPU: {:.1}%, Memory: {:.1}%)", 
                                   cpu_util, memory_util),
                priority: 2,
            });
        } else if cpu_util < 20.0 && memory_util < 30.0 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::ScaleDown { target_instances: 1 },
                confidence: 0.70,
                expected_improvement: 0.2,
                description: "Low resource utilization - consider scaling down".to_string(),
                priority: 6,
            });
        }
        
        // Response time recommendations
        if response_time > 500.0 {
            if prediction.predicted_response_time < response_time * 0.7 {
                recommendations.push(Recommendation {
                    service: service_name.to_string(),
                    recommendation_type: RecommendationType::TimeoutAdjust { 
                        new_timeout_ms: (response_time * 1.5) as u64 
                    },
                    confidence: prediction.confidence,
                    expected_improvement: 0.25,
                    description: format!("High response time detected ({:.0}ms) - adjust timeout", response_time),
                    priority: 3,
                });
            }
        }
        
        // Error rate recommendations
        if error_rate > 5.0 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::CircuitBreakerTune {
                    failure_threshold: 0.03,
                    recovery_timeout: Duration::from_secs(30),
                },
                confidence: 0.80,
                expected_improvement: 0.4,
                description: format!("High error rate detected ({:.1}%) - tune circuit breaker", error_rate),
                priority: 1,
            });
        }
        
        // Load balancing recommendations
        if success_rate < 95.0 && prediction.predicted_success_rate > 0.97 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::LoadBalanceAdjust {
                    new_strategy: LoadBalancingStrategy::NeuralOptimal,
                },
                confidence: prediction.confidence,
                expected_improvement: 0.35,
                description: "Neural network suggests load balancing optimization".to_string(),
                priority: 2,
            });
        }
        
        // Rate limiting recommendations
        let request_rate = metrics.get("request_rate").cloned().unwrap_or(0.0);
        if request_rate > prediction.predicted_throughput * 0.9 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::RateLimitUpdate {
                    requests_per_second: prediction.predicted_throughput * 0.8,
                },
                confidence: prediction.confidence * 0.9,
                expected_improvement: 0.2,
                description: "Request rate approaching capacity - implement rate limiting".to_string(),
                priority: 4,
            });
        }
        
        // Retry policy recommendations
        if error_rate > 2.0 && error_rate < 10.0 {
            recommendations.push(Recommendation {
                service: service_name.to_string(),
                recommendation_type: RecommendationType::RetryPolicyUpdate {
                    max_retries: 3,
                    backoff_strategy: BackoffStrategy::Exponential { 
                        base_ms: 100,
                        multiplier: 2.0,
                    },
                },
                confidence: 0.75,
                expected_improvement: 0.15,
                description: "Moderate error rate - optimize retry policy".to_string(),
                priority: 5,
            });
        }
        
        Ok(recommendations)
    }
    
    /// Update circuit breaker based on service metrics
    pub async fn update_circuit_breaker(&mut self,
        service_name: &str,
        metrics: &ServiceMetrics
    ) -> Result<bool> {
        let breaker = self.circuit_breakers
            .entry(service_name.to_string())
            .or_insert_with(|| CircuitBreaker {
                service_name: service_name.to_string(),
                state: CircuitBreakerState::Closed,
                failure_threshold: 0.05, // 5% error rate threshold
                recovery_timeout: Duration::from_secs(30),
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_state_change: SystemTime::now(),
                recovery_probability: 0.5,
            });
        
        let current_error_rate = metrics.error_rate / 100.0; // Convert percentage to ratio
        let mut state_changed = false;
        
        match breaker.state {
            CircuitBreakerState::Closed => {
                if current_error_rate > breaker.failure_threshold {
                    breaker.failure_count += 1;
                } else {
                    breaker.success_count += 1;
                    breaker.failure_count = 0; // Reset failure count on success
                }
                
                // Open circuit if failure threshold exceeded
                if breaker.failure_count >= 5 { // 5 consecutive failures
                    breaker.state = CircuitBreakerState::Open;
                    breaker.last_state_change = SystemTime::now();
                    breaker.last_failure_time = Some(SystemTime::now());
                    self.circuit_breaker_activations += 1;
                    state_changed = true;
                    
                    warn!("Circuit breaker opened for service: {} (error rate: {:.1}%)",
                          service_name, metrics.error_rate);
                }
            },
            
            CircuitBreakerState::Open => {
                if let Some(last_failure) = breaker.last_failure_time {
                    if last_failure.elapsed().unwrap_or(Duration::ZERO) >= breaker.recovery_timeout {
                        // Predict recovery probability using neural network
                        breaker.recovery_probability = self.predict_recovery_probability(service_name, metrics).await?;
                        
                        if breaker.recovery_probability > 0.6 {
                            breaker.state = CircuitBreakerState::HalfOpen;
                            breaker.last_state_change = SystemTime::now();
                            state_changed = true;
                            
                            info!("Circuit breaker half-opened for service: {} (recovery probability: {:.1}%)",
                                  service_name, breaker.recovery_probability * 100.0);
                        }
                    }
                }
            },
            
            CircuitBreakerState::HalfOpen => {
                if current_error_rate <= breaker.failure_threshold * 0.5 {
                    breaker.success_count += 1;
                    
                    if breaker.success_count >= 3 { // 3 successful requests to close
                        breaker.state = CircuitBreakerState::Closed;
                        breaker.last_state_change = SystemTime::now();
                        breaker.failure_count = 0;
                        breaker.success_count = 0;
                        state_changed = true;
                        
                        info!("Circuit breaker closed for service: {}", service_name);
                    }
                } else {
                    breaker.state = CircuitBreakerState::Open;
                    breaker.last_failure_time = Some(SystemTime::now());
                    breaker.last_state_change = SystemTime::now();
                    breaker.failure_count += 1;
                    state_changed = true;
                    
                    warn!("Circuit breaker reopened for service: {} (failed recovery test)", service_name);
                }
            },
        }
        
        Ok(state_changed)
    }
    
    /// Predict recovery probability for circuit breaker
    async fn predict_recovery_probability(&self,
        service_name: &str,
        metrics: &ServiceMetrics
    ) -> Result<f64> {
        let input = vec![
            (metrics.error_rate / 100.0).min(1.0),              // Current error rate
            (metrics.response_time_p95 / 1000.0).min(1.0),      // Response time
            (metrics.cpu_utilization / 100.0).min(1.0),         // CPU utilization
            (metrics.memory_utilization / 100.0).min(1.0),      // Memory utilization
            (metrics.active_connections as f64 / 1000.0).min(1.0), // Connection load
        ];
        
        let output = {
            let mut network = self.neural_network.write().await;
            network.process_input(&input, None).await?
        };
        
        self.neural_predictions += 1;
        
        // Extract recovery probability from neural output
        let recovery_prob = output.get(0).cloned().unwrap_or(0.5).max(0.0).min(1.0);
        
        Ok(recovery_prob)
    }
    
    /// Perform neural load balancing decision
    pub async fn neural_load_balance(&mut self,
        service_name: &str,
        endpoints: &[ServiceEndpoint]
    ) -> Result<String> {
        if endpoints.is_empty() {
            return Err(anyhow::anyhow!("No endpoints available for load balancing"));
        }
        
        // Get current load balancer
        let balancer = self.load_balancers.get_mut(service_name);
        
        if let Some(lb) = balancer {
            // Update endpoint predictions
            for endpoint in endpoints {
                let prediction = self.predict_endpoint_performance(endpoint).await?;
                lb.endpoint_predictions.insert(endpoint.id.clone(), prediction);
            }
            
            // Select best endpoint based on neural predictions
            let selected_endpoint = self.select_optimal_endpoint(endpoints, &lb.endpoint_predictions)?;
            lb.request_count += 1;
            self.load_balance_decisions += 1;
            
            trace!("Neural load balancing selected endpoint: {} for service: {}", 
                   selected_endpoint, service_name);
            
            Ok(selected_endpoint)
        } else {
            // Fallback to round robin
            let index = self.load_balance_decisions as usize % endpoints.len();
            Ok(endpoints[index].id.clone())
        }
    }
    
    /// Predict endpoint performance
    async fn predict_endpoint_performance(&self, endpoint: &ServiceEndpoint) -> Result<EndpointPrediction> {
        let input = vec![
            (endpoint.current_connections as f64 / 100.0).min(1.0),
            (endpoint.recent_response_time / 1000.0).min(1.0),
            (endpoint.recent_error_rate / 100.0).min(1.0),
            endpoint.weight.min(1.0),
            match endpoint.health_status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.5,
                HealthStatus::Unhealthy => 0.0,
                HealthStatus::Unknown => 0.3,
            },
        ];
        
        let output = {
            let mut network = self.neural_network.write().await;
            network.process_input(&input, None).await?
        };
        
        Ok(EndpointPrediction {
            expected_response_time: output.get(0).cloned().unwrap_or(0.1) * 1000.0,
            expected_success_rate: output.get(1).cloned().unwrap_or(0.95),
            expected_throughput: output.get(2).cloned().unwrap_or(0.5) * 1000.0,
            confidence: output.get(3).cloned().unwrap_or(0.7),
            last_updated: SystemTime::now(),
        })
    }
    
    /// Select optimal endpoint based on predictions
    fn select_optimal_endpoint(&self,
        endpoints: &[ServiceEndpoint],
        predictions: &HashMap<String, EndpointPrediction>
    ) -> Result<String> {
        let mut best_endpoint = &endpoints[0];
        let mut best_score = 0.0;
        
        for endpoint in endpoints {
            if endpoint.health_status == HealthStatus::Unhealthy {
                continue;
            }
            
            let prediction = predictions.get(&endpoint.id);
            
            let score = if let Some(pred) = prediction {
                // Weighted score based on prediction
                let response_time_score = 1.0 - (pred.expected_response_time / 1000.0).min(1.0);
                let success_rate_score = pred.expected_success_rate;
                let throughput_score = (pred.expected_throughput / 1000.0).min(1.0);
                let confidence_weight = pred.confidence;
                
                (response_time_score * 0.3 + success_rate_score * 0.4 + throughput_score * 0.3) * confidence_weight
            } else {
                // Fallback scoring based on current metrics
                let load_score = 1.0 - (endpoint.current_connections as f64 / 100.0).min(1.0);
                let response_score = 1.0 - (endpoint.recent_response_time / 1000.0).min(1.0);
                let error_score = 1.0 - (endpoint.recent_error_rate / 100.0).min(1.0);
                
                (load_score + response_score + error_score) / 3.0 * endpoint.weight
            };
            
            if score > best_score {
                best_score = score;
                best_endpoint = endpoint;
            }
        }
        
        Ok(best_endpoint.id.clone())
    }
    
    /// Get service mesh statistics
    pub fn get_service_mesh_stats(&self) -> ServiceMeshStats {
        let active_circuit_breakers = self.circuit_breakers.values()
            .filter(|cb| cb.state != CircuitBreakerState::Closed)
            .count();
        
        let avg_endpoint_confidence = if !self.load_balancers.is_empty() {
            let total_confidence: f64 = self.load_balancers.values()
                .flat_map(|lb| lb.endpoint_predictions.values())
                .map(|pred| pred.confidence)
                .sum();
            let total_predictions = self.load_balancers.values()
                .map(|lb| lb.endpoint_predictions.len())
                .sum::<usize>();
            
            if total_predictions > 0 {
                total_confidence / total_predictions as f64
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        ServiceMeshStats {
            total_services: self.circuit_breakers.len().max(self.load_balancers.len()),
            active_circuit_breakers,
            total_requests: self.total_requests,
            circuit_breaker_activations: self.circuit_breaker_activations,
            load_balance_decisions: self.load_balance_decisions,
            neural_predictions: self.neural_predictions,
            average_endpoint_confidence: avg_endpoint_confidence,
            cached_recommendations: self.recommendation_cache.len(),
        }
    }
    
    /// Compute hash for metrics caching
    fn compute_metrics_hash(&self, metrics: &HashMap<String, f64>) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        let mut keys: Vec<_> = metrics.keys().collect();
        keys.sort();
        
        for key in keys {
            if let Some(&value) = metrics.get(key) {
                key.hash(&mut hasher);
                ((value * 100.0).round() as i64).hash(&mut hasher);
            }
        }
        
        format!("{:x}", hasher.finish())
    }
}

/// Service performance prediction from neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServicePrediction {
    pub predicted_response_time: f64,
    pub predicted_success_rate: f64,
    pub predicted_throughput: f64,
    pub predicted_cpu_usage: f64,
    pub confidence: f64,
}

/// Service mesh statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshStats {
    pub total_services: usize,
    pub active_circuit_breakers: usize,
    pub total_requests: u64,
    pub circuit_breaker_activations: u64,
    pub load_balance_decisions: u64,
    pub neural_predictions: u64,
    pub average_endpoint_confidence: f64,
    pub cached_recommendations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::NeuralNetwork;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    #[tokio::test]
    async fn test_service_mesh_creation() {
        let network = Arc::new(RwLock::new(
            NeuralNetwork::new(50, 5).await.unwrap()
        ));
        
        let mesh = ServiceMeshIntelligence::new(network).await;
        assert!(mesh.is_ok());
    }
    
    #[tokio::test]
    async fn test_recommendation_generation() {
        let network = Arc::new(RwLock::new(
            NeuralNetwork::new(64, 8).await.unwrap()
        ));
        
        let mesh = ServiceMeshIntelligence::new(network).await.unwrap();
        
        let mut metrics = HashMap::new();
        metrics.insert("cpu_utilization_service1".to_string(), 85.0);
        metrics.insert("response_time_p95_service1".to_string(), 600.0);
        metrics.insert("error_rate_service1".to_string(), 8.0);
        
        let recommendations = mesh.analyze_and_recommend(&metrics).await;
        assert!(recommendations.is_ok());
        
        let recs = recommendations.unwrap();
        assert!(!recs.is_empty());
        
        // Should recommend scaling up due to high CPU
        let has_scale_up = recs.iter().any(|r| {
            matches!(r.recommendation_type, RecommendationType::ScaleUp { .. })
        });
        assert!(has_scale_up);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_functionality() {
        let network = Arc::new(RwLock::new(
            NeuralNetwork::new(32, 4).await.unwrap()
        ));
        
        let mut mesh = ServiceMeshIntelligence::new(network).await.unwrap();
        
        let metrics = ServiceMetrics {
            service_name: "test_service".to_string(),
            request_rate: 100.0,
            response_time_p50: 150.0,
            response_time_p95: 300.0,
            response_time_p99: 500.0,
            error_rate: 10.0, // High error rate
            active_connections: 50,
            cpu_utilization: 60.0,
            memory_utilization: 55.0,
            throughput_mbps: 100.0,
            success_rate: 90.0,
            timestamp: SystemTime::now(),
        };
        
        // First update should open circuit breaker due to high error rate
        let mut state_changed = false;
        for _ in 0..6 { // Need multiple failures to trigger
            let changed = mesh.update_circuit_breaker("test_service", &metrics).await.unwrap();
            state_changed = state_changed || changed;
        }
        
        assert!(state_changed);
        
        let breaker = mesh.circuit_breakers.get("test_service").unwrap();
        assert_eq!(breaker.state, CircuitBreakerState::Open);
    }
    
    #[tokio::test]
    async fn test_neural_load_balancing() {
        let network = Arc::new(RwLock::new(
            NeuralNetwork::new(32, 4).await.unwrap()
        ));
        
        let mut mesh = ServiceMeshIntelligence::new(network).await.unwrap();
        
        let endpoints = vec![
            ServiceEndpoint {
                id: "endpoint1".to_string(),
                address: "10.0.0.1".to_string(),
                port: 8080,
                weight: 1.0,
                health_status: HealthStatus::Healthy,
                current_connections: 10,
                recent_response_time: 100.0,
                recent_error_rate: 1.0,
            },
            ServiceEndpoint {
                id: "endpoint2".to_string(),
                address: "10.0.0.2".to_string(),
                port: 8080,
                weight: 1.0,
                health_status: HealthStatus::Healthy,
                current_connections: 50,
                recent_response_time: 200.0,
                recent_error_rate: 5.0,
            },
        ];
        
        let selected = mesh.neural_load_balance("test_service", &endpoints).await;
        assert!(selected.is_ok());
        
        let endpoint_id = selected.unwrap();
        assert!(endpoint_id == "endpoint1" || endpoint_id == "endpoint2");
    }
    
    #[test]
    fn test_service_metrics_encoding() {
        let mesh = tokio_test::block_on(async {
            let network = Arc::new(RwLock::new(
                NeuralNetwork::new(32, 4).await.unwrap()
            ));
            ServiceMeshIntelligence::new(network).await.unwrap()
        });
        
        let mut metrics = HashMap::new();
        metrics.insert("cpu_utilization".to_string(), 75.0);
        metrics.insert("response_time_p95".to_string(), 250.0);
        metrics.insert("error_rate".to_string(), 3.0);
        
        let encoded = mesh.encode_service_metrics(&metrics);
        assert_eq!(encoded.len(), 64);
        assert!(encoded.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }
    
    #[test]
    fn test_metric_normalization() {
        let mesh = tokio_test::block_on(async {
            let network = Arc::new(RwLock::new(
                NeuralNetwork::new(10, 2).await.unwrap()
            ));
            ServiceMeshIntelligence::new(network).await.unwrap()
        });
        
        assert_eq!(mesh.normalize_service_metric("cpu_utilization", 50.0), 0.5);
        assert_eq!(mesh.normalize_service_metric("response_time_p95", 500.0), 0.5);
        assert_eq!(mesh.normalize_service_metric("error_rate", 10.0), 0.1);
    }
}
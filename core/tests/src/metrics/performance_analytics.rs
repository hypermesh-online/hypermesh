//! Performance analytics engine for system health and optimization

use super::{MetricsStorage, Metric, TimeRange};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub struct PerformanceAnalytics {
    storage: Arc<RwLock<MetricsStorage>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    reports: Arc<RwLock<Vec<PerformanceReport>>>,
}

impl PerformanceAnalytics {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let storage = Arc::new(RwLock::new(MetricsStorage::new()));
        
        // Seed with some initial data for testing
        Self::seed_test_data(&storage).await;
        
        Ok(Self {
            storage,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            reports: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    async fn seed_test_data(storage: &Arc<RwLock<MetricsStorage>>) {
        let mut storage_guard = storage.write().await;
        let now = SystemTime::now();
        
        // Generate some test metrics for different components
        for i in 0..100 {
            let timestamp = now - Duration::from_secs(i * 60); // Every minute for last 100 minutes
            
            // Runtime metrics
            storage_guard.store_metric("runtime", create_test_runtime_metric(timestamp)).await;
            
            // Consensus metrics
            storage_guard.store_metric("consensus", create_test_consensus_metric(timestamp)).await;
            
            // Network metrics
            storage_guard.store_metric("network", create_test_network_metric(timestamp)).await;
            
            // eBPF metrics
            storage_guard.store_metric("ebpf", create_test_ebpf_metric(timestamp)).await;
        }
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Start performance analysis loop
        let storage = self.storage.clone();
        let reports = self.reports.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                match Self::analyze_performance(&storage).await {
                    Ok(report) => {
                        let mut reports_guard = reports.write().await;
                        reports_guard.push(report);
                        
                        // Keep only last 24 reports (hours)
                        if reports_guard.len() > 24 {
                            reports_guard.remove(0);
                        }
                    },
                    Err(e) => {
                        tracing::error!("Performance analysis failed: {}", e);
                    }
                }
                
                tokio::time::sleep(Duration::from_secs(3600)).await; // Analyze every hour
            }
        });
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn analyze_performance(storage: &Arc<RwLock<MetricsStorage>>) -> Result<PerformanceReport, Box<dyn std::error::Error>> {
        let time_range = TimeRange {
            start: SystemTime::now() - Duration::from_secs(3600), // Last hour
            end: SystemTime::now(),
        };
        
        let storage_guard = storage.read().await;
        
        // Analyze different components
        let runtime_analysis = Self::analyze_runtime_performance(&storage_guard, &time_range).await?;
        let consensus_analysis = Self::analyze_consensus_performance(&storage_guard, &time_range).await?;
        let network_analysis = Self::analyze_network_performance(&storage_guard, &time_range).await?;
        let ebpf_analysis = Self::analyze_ebpf_performance(&storage_guard, &time_range).await?;
        
        // Calculate overall health score
        let overall_health_score = Self::calculate_overall_health_score(
            &runtime_analysis,
            &consensus_analysis,
            &network_analysis,
            &ebpf_analysis,
        );
        
        // Identify performance bottlenecks
        let bottlenecks = Self::identify_bottlenecks(
            &runtime_analysis,
            &consensus_analysis,
            &network_analysis,
            &ebpf_analysis,
        );
        
        // Generate recommendations
        let recommendations = Self::generate_recommendations(&bottlenecks);
        
        let mut system_metrics = HashMap::new();
        system_metrics.insert("runtime".to_string(), runtime_analysis);
        system_metrics.insert("consensus".to_string(), consensus_analysis);
        system_metrics.insert("network".to_string(), network_analysis);
        system_metrics.insert("ebpf".to_string(), ebpf_analysis);
        
        Ok(PerformanceReport {
            timestamp: SystemTime::now(),
            time_range,
            overall_health_score,
            system_metrics,
            bottlenecks,
            recommendations,
        })
    }
    
    async fn analyze_runtime_performance(
        storage: &MetricsStorage,
        time_range: &TimeRange,
    ) -> Result<ComponentAnalysis, Box<dyn std::error::Error>> {
        let metrics = storage.query_metrics("runtime", time_range.clone()).await?;
        
        if metrics.is_empty() {
            return Ok(ComponentAnalysis::default());
        }
        
        let mut cpu_values = Vec::new();
        let mut memory_values = Vec::new();
        
        for metric in &metrics {
            match metric.name.as_str() {
                "cpu_usage_percent" => {
                    if let super::MetricValue::Gauge(value) = metric.value {
                        cpu_values.push(value);
                    }
                },
                "memory_utilization_percent" => {
                    if let super::MetricValue::Gauge(value) = metric.value {
                        memory_values.push(value);
                    }
                },
                _ => {}
            }
        }
        
        let cpu_avg = if !cpu_values.is_empty() {
            cpu_values.iter().sum::<f64>() / cpu_values.len() as f64
        } else { 0.0 };
        
        let memory_avg = if !memory_values.is_empty() {
            memory_values.iter().sum::<f64>() / memory_values.len() as f64
        } else { 0.0 };
        
        // Calculate health score (0-100)
        let cpu_health = (100.0 - cpu_avg).max(0.0);
        let memory_health = (100.0 - memory_avg).max(0.0);
        let health_score = (cpu_health + memory_health) / 2.0;
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("avg_cpu_usage".to_string(), cpu_avg);
        performance_metrics.insert("avg_memory_usage".to_string(), memory_avg);
        performance_metrics.insert("max_cpu_usage".to_string(), 
                                 cpu_values.iter().copied().fold(0.0f64, f64::max));
        performance_metrics.insert("max_memory_usage".to_string(), 
                                 memory_values.iter().copied().fold(0.0f64, f64::max));
        
        Ok(ComponentAnalysis {
            health_score,
            performance_metrics,
            issues: if cpu_avg > 80.0 || memory_avg > 85.0 {
                vec!["High resource utilization detected".to_string()]
            } else {
                vec![]
            },
        })
    }
    
    async fn analyze_consensus_performance(
        storage: &MetricsStorage,
        time_range: &TimeRange,
    ) -> Result<ComponentAnalysis, Box<dyn std::error::Error>> {
        let metrics = storage.query_metrics("consensus", time_range.clone()).await?;
        
        if metrics.is_empty() {
            return Ok(ComponentAnalysis::default());
        }
        
        let mut commit_latencies = Vec::new();
        let mut election_durations = Vec::new();
        let mut total_commits = 0u64;
        
        for metric in &metrics {
            match metric.name.as_str() {
                "consensus_commit_latency_ms" => {
                    if let super::MetricValue::Histogram(hist) = &metric.value {
                        // Estimate average from histogram
                        if hist.count > 0 {
                            commit_latencies.push(hist.sum / hist.count as f64);
                        }
                    }
                },
                "consensus_election_duration_ms" => {
                    if let super::MetricValue::Histogram(hist) = &metric.value {
                        if hist.count > 0 {
                            election_durations.push(hist.sum / hist.count as f64);
                        }
                    }
                },
                "consensus_commits_total" => {
                    if let super::MetricValue::Counter(value) = metric.value {
                        total_commits = total_commits.max(value);
                    }
                },
                _ => {}
            }
        }
        
        let avg_commit_latency = if !commit_latencies.is_empty() {
            commit_latencies.iter().sum::<f64>() / commit_latencies.len() as f64
        } else { 0.0 };
        
        let avg_election_duration = if !election_durations.is_empty() {
            election_durations.iter().sum::<f64>() / election_durations.len() as f64
        } else { 0.0 };
        
        // Health score based on latency thresholds
        let latency_health = if avg_commit_latency < 10.0 {
            100.0
        } else if avg_commit_latency < 50.0 {
            80.0
        } else if avg_commit_latency < 100.0 {
            60.0
        } else {
            30.0
        };
        
        let election_health = if avg_election_duration < 1000.0 {
            100.0
        } else if avg_election_duration < 5000.0 {
            70.0
        } else {
            40.0
        };
        
        let health_score = (latency_health + election_health) / 2.0;
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("avg_commit_latency_ms".to_string(), avg_commit_latency);
        performance_metrics.insert("avg_election_duration_ms".to_string(), avg_election_duration);
        performance_metrics.insert("total_commits".to_string(), total_commits as f64);
        
        let mut issues = Vec::new();
        if avg_commit_latency > 50.0 {
            issues.push("High consensus commit latency".to_string());
        }
        if avg_election_duration > 5000.0 {
            issues.push("Slow leader elections".to_string());
        }
        
        Ok(ComponentAnalysis {
            health_score,
            performance_metrics,
            issues,
        })
    }
    
    async fn analyze_network_performance(
        storage: &MetricsStorage,
        time_range: &TimeRange,
    ) -> Result<ComponentAnalysis, Box<dyn std::error::Error>> {
        let metrics = storage.query_metrics("network", time_range.clone()).await?;
        
        if metrics.is_empty() {
            return Ok(ComponentAnalysis::default());
        }
        
        let mut bandwidth_utilizations = Vec::new();
        let mut latencies = Vec::new();
        let mut error_rates = Vec::new();
        
        for metric in &metrics {
            match metric.name.as_str() {
                "network_bandwidth_utilization_percent" => {
                    if let super::MetricValue::Gauge(value) = metric.value {
                        bandwidth_utilizations.push(value);
                    }
                },
                "network_latency_ms" => {
                    if let super::MetricValue::Histogram(hist) = &metric.value {
                        if hist.count > 0 {
                            latencies.push(hist.sum / hist.count as f64);
                        }
                    }
                },
                _ => {}
            }
        }
        
        let avg_bandwidth_utilization = if !bandwidth_utilizations.is_empty() {
            bandwidth_utilizations.iter().sum::<f64>() / bandwidth_utilizations.len() as f64
        } else { 0.0 };
        
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<f64>() / latencies.len() as f64
        } else { 0.0 };
        
        // Health score based on utilization and latency
        let bandwidth_health = if avg_bandwidth_utilization < 60.0 {
            100.0
        } else if avg_bandwidth_utilization < 80.0 {
            70.0
        } else {
            30.0
        };
        
        let latency_health = if avg_latency < 10.0 {
            100.0
        } else if avg_latency < 50.0 {
            80.0
        } else {
            50.0
        };
        
        let health_score = (bandwidth_health + latency_health) / 2.0;
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("avg_bandwidth_utilization".to_string(), avg_bandwidth_utilization);
        performance_metrics.insert("avg_latency_ms".to_string(), avg_latency);
        
        let mut issues = Vec::new();
        if avg_bandwidth_utilization > 80.0 {
            issues.push("High network utilization".to_string());
        }
        if avg_latency > 50.0 {
            issues.push("High network latency".to_string());
        }
        
        Ok(ComponentAnalysis {
            health_score,
            performance_metrics,
            issues,
        })
    }
    
    async fn analyze_ebpf_performance(
        storage: &MetricsStorage,
        time_range: &TimeRange,
    ) -> Result<ComponentAnalysis, Box<dyn std::error::Error>> {
        let metrics = storage.query_metrics("ebpf", time_range.clone()).await?;
        
        if metrics.is_empty() {
            return Ok(ComponentAnalysis::default());
        }
        
        let mut execution_times = Vec::new();
        let mut drop_rates = Vec::new();
        let mut program_executions = Vec::new();
        
        for metric in &metrics {
            match metric.name.as_str() {
                "ebpf_program_execution_time_ns" => {
                    if let super::MetricValue::Histogram(hist) = &metric.value {
                        if hist.count > 0 {
                            execution_times.push(hist.sum / hist.count as f64);
                        }
                    }
                },
                "ebpf_program_executions_total" => {
                    if let super::MetricValue::Counter(value) = metric.value {
                        program_executions.push(value as f64);
                    }
                },
                _ => {}
            }
        }
        
        let avg_execution_time = if !execution_times.is_empty() {
            execution_times.iter().sum::<f64>() / execution_times.len() as f64
        } else { 0.0 };
        
        let total_executions = program_executions.iter().sum::<f64>();
        
        // Health score based on execution efficiency
        let execution_health = if avg_execution_time < 1000.0 { // < 1μs
            100.0
        } else if avg_execution_time < 5000.0 { // < 5μs
            80.0
        } else if avg_execution_time < 10000.0 { // < 10μs
            60.0
        } else {
            30.0
        };
        
        let health_score = execution_health;
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("avg_execution_time_ns".to_string(), avg_execution_time);
        performance_metrics.insert("total_executions".to_string(), total_executions);
        
        let mut issues = Vec::new();
        if avg_execution_time > 5000.0 {
            issues.push("Slow eBPF program execution".to_string());
        }
        
        Ok(ComponentAnalysis {
            health_score,
            performance_metrics,
            issues,
        })
    }
    
    fn calculate_overall_health_score(
        runtime: &ComponentAnalysis,
        consensus: &ComponentAnalysis,
        network: &ComponentAnalysis,
        ebpf: &ComponentAnalysis,
    ) -> f64 {
        // Weighted average with consensus being most important
        let consensus_weight = 0.4;
        let network_weight = 0.3;
        let runtime_weight = 0.2;
        let ebpf_weight = 0.1;
        
        consensus.health_score * consensus_weight +
        network.health_score * network_weight +
        runtime.health_score * runtime_weight +
        ebpf.health_score * ebpf_weight
    }
    
    fn identify_bottlenecks(
        runtime: &ComponentAnalysis,
        consensus: &ComponentAnalysis,
        network: &ComponentAnalysis,
        ebpf: &ComponentAnalysis,
    ) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();
        
        // Identify bottlenecks based on health scores and specific metrics
        if runtime.health_score < 70.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: "runtime".to_string(),
                severity: if runtime.health_score < 50.0 {
                    BottleneckSeverity::Critical
                } else {
                    BottleneckSeverity::Warning
                },
                description: "Runtime resource constraints detected".to_string(),
                impact: "May cause overall system slowdown".to_string(),
            });
        }
        
        if consensus.health_score < 70.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: "consensus".to_string(),
                severity: if consensus.health_score < 50.0 {
                    BottleneckSeverity::Critical
                } else {
                    BottleneckSeverity::Warning
                },
                description: "Consensus performance degradation".to_string(),
                impact: "Will directly impact system responsiveness and reliability".to_string(),
            });
        }
        
        if network.health_score < 70.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: "network".to_string(),
                severity: if network.health_score < 50.0 {
                    BottleneckSeverity::Critical
                } else {
                    BottleneckSeverity::Warning
                },
                description: "Network performance issues".to_string(),
                impact: "May cause communication delays and partitions".to_string(),
            });
        }
        
        if ebpf.health_score < 70.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: "ebpf".to_string(),
                severity: BottleneckSeverity::Warning,
                description: "eBPF program efficiency concerns".to_string(),
                impact: "May reduce packet processing throughput".to_string(),
            });
        }
        
        bottlenecks
    }
    
    fn generate_recommendations(bottlenecks: &[PerformanceBottleneck]) -> Vec<PerformanceRecommendation> {
        let mut recommendations = Vec::new();
        
        for bottleneck in bottlenecks {
            match bottleneck.component.as_str() {
                "runtime" => {
                    recommendations.push(PerformanceRecommendation {
                        component: "runtime".to_string(),
                        action: "Scale up resources".to_string(),
                        description: "Consider increasing CPU cores or memory allocation".to_string(),
                        priority: RecommendationPriority::High,
                        estimated_impact: "20-40% performance improvement".to_string(),
                    });
                },
                "consensus" => {
                    recommendations.push(PerformanceRecommendation {
                        component: "consensus".to_string(),
                        action: "Optimize consensus parameters".to_string(),
                        description: "Review election timeout and heartbeat intervals".to_string(),
                        priority: RecommendationPriority::Critical,
                        estimated_impact: "Significant latency reduction".to_string(),
                    });
                },
                "network" => {
                    recommendations.push(PerformanceRecommendation {
                        component: "network".to_string(),
                        action: "Upgrade network infrastructure".to_string(),
                        description: "Consider higher bandwidth links or network optimization".to_string(),
                        priority: RecommendationPriority::Medium,
                        estimated_impact: "Improved throughput and reduced latency".to_string(),
                    });
                },
                "ebpf" => {
                    recommendations.push(PerformanceRecommendation {
                        component: "ebpf".to_string(),
                        action: "Optimize eBPF programs".to_string(),
                        description: "Review program logic and reduce instruction count".to_string(),
                        priority: RecommendationPriority::Low,
                        estimated_impact: "Faster packet processing".to_string(),
                    });
                },
                _ => {}
            }
        }
        
        recommendations
    }
    
    pub async fn generate_performance_report(&self) -> Result<PerformanceReport, Box<dyn std::error::Error>> {
        Self::analyze_performance(&self.storage).await
    }
}

// Helper structures

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub timestamp: SystemTime,
    pub time_range: TimeRange,
    pub overall_health_score: f64,
    pub system_metrics: HashMap<String, ComponentAnalysis>,
    pub bottlenecks: Vec<PerformanceBottleneck>,
    pub recommendations: Vec<PerformanceRecommendation>,
}

#[derive(Debug, Clone)]
pub struct ComponentAnalysis {
    pub health_score: f64,
    pub performance_metrics: HashMap<String, f64>,
    pub issues: Vec<String>,
}

impl Default for ComponentAnalysis {
    fn default() -> Self {
        Self {
            health_score: 100.0,
            performance_metrics: HashMap::new(),
            issues: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceBottleneck {
    pub component: String,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub impact: String,
}

#[derive(Debug, Clone)]
pub enum BottleneckSeverity {
    Low,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct PerformanceRecommendation {
    pub component: String,
    pub action: String,
    pub description: String,
    pub priority: RecommendationPriority,
    pub estimated_impact: String,
}

#[derive(Debug, Clone)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

// Test data generation helpers

fn create_test_runtime_metric(timestamp: SystemTime) -> Metric {
    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "test-node".to_string());
    
    Metric {
        name: "cpu_usage_percent".to_string(),
        value: super::MetricValue::Gauge(30.0 + (rand::random::<f64>() * 40.0)), // 30-70% CPU
        timestamp,
        labels,
    }
}

fn create_test_consensus_metric(timestamp: SystemTime) -> Metric {
    Metric {
        name: "consensus_commit_latency_ms".to_string(),
        value: super::MetricValue::Histogram(super::HistogramData {
            buckets: vec![
                super::HistogramBucket { upper_bound: 10.0, count: 80 },
                super::HistogramBucket { upper_bound: 25.0, count: 15 },
                super::HistogramBucket { upper_bound: 50.0, count: 5 },
            ],
            sum: 800.0,
            count: 100,
        }),
        timestamp,
        labels: HashMap::new(),
    }
}

fn create_test_network_metric(timestamp: SystemTime) -> Metric {
    let mut labels = HashMap::new();
    labels.insert("interface".to_string(), "eth0".to_string());
    
    Metric {
        name: "network_bandwidth_utilization_percent".to_string(),
        value: super::MetricValue::Gauge(40.0 + (rand::random::<f64>() * 30.0)), // 40-70% utilization
        timestamp,
        labels,
    }
}

fn create_test_ebpf_metric(timestamp: SystemTime) -> Metric {
    let mut labels = HashMap::new();
    labels.insert("program".to_string(), "xdp_load_balancer".to_string());
    
    Metric {
        name: "ebpf_program_execution_time_ns".to_string(),
        value: super::MetricValue::Histogram(super::HistogramData {
            buckets: vec![
                super::HistogramBucket { upper_bound: 500.0, count: 90 },
                super::HistogramBucket { upper_bound: 1000.0, count: 8 },
                super::HistogramBucket { upper_bound: 5000.0, count: 2 },
            ],
            sum: 35000.0,
            count: 100,
        }),
        timestamp,
        labels,
    }
}
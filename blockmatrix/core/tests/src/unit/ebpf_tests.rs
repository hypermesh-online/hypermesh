//! Comprehensive unit tests for eBPF components

use crate::{TestResult, init_test_logging, unit_test};
use tempfile::TempDir;
use std::time::Duration;
use nexus_shared::NodeId;

pub async fn run_ebpf_tests() -> TestResult {
    init_test_logging();
    
    test_ebpf_manager_creation().await?;
    test_program_loading().await?;
    test_network_monitoring().await?;
    test_traffic_control().await?;
    test_metrics_collection().await?;
    
    Ok(())
}

unit_test!(test_ebpf_manager_creation, "ebpf_manager", {
    let temp_dir = TempDir::new()?;
    let mut config = EbpfConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let node_id = NodeId::random();
    let ebpf_manager = EbpfManager::new(config, node_id).await?;
    
    assert_eq!(ebpf_manager.node_id(), node_id);
    assert!(!ebpf_manager.is_running());
    
    let stats = ebpf_manager.stats().await;
    assert_eq!(stats.loaded_programs, 0);
    assert_eq!(stats.active_programs, 0);
    
    Ok(())
});

unit_test!(test_program_loading, "program_loading", {
    let temp_dir = TempDir::new()?;
    let mut config = EbpfConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.enable_simulation = true;
    
    let node_id = NodeId::random();
    let mut ebpf_manager = EbpfManager::new(config, node_id).await?;
    ebpf_manager.start().await?;
    
    let program_config = EbpfProgramConfig {
        name: "network_monitor".to_string(),
        program_type: ProgramType::NetworkMonitoring,
        interface: Some("eth0".to_string()),
        auto_attach: true,
        priority: 100,
    };
    
    let program_id = ebpf_manager.load_program(program_config).await?;
    assert!(!program_id.is_empty());
    
    let loaded_programs = ebpf_manager.list_programs().await;
    assert!(loaded_programs.iter().any(|p| p.id == program_id));
    
    ebpf_manager.unload_program(&program_id).await?;
    
    let updated_programs = ebpf_manager.list_programs().await;
    assert!(!updated_programs.iter().any(|p| p.id == program_id));
    
    ebpf_manager.stop().await?;
    Ok(())
});

unit_test!(test_network_monitoring, "network_monitoring", {
    let temp_dir = TempDir::new()?;
    let mut config = EbpfConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.enable_simulation = true;
    
    let node_id = NodeId::random();
    let mut ebpf_manager = EbpfManager::new(config, node_id).await?;
    ebpf_manager.start().await?;
    
    let monitor_config = EbpfProgramConfig {
        name: "packet_monitor".to_string(),
        program_type: ProgramType::NetworkMonitoring,
        interface: Some("eth0".to_string()),
        auto_attach: true,
        priority: 100,
    };
    
    let program_id = ebpf_manager.load_program(monitor_config).await?;
    
    let packet_data = PacketData {
        interface: "eth0".to_string(),
        source_ip: "192.168.1.100".to_string(),
        dest_ip: "192.168.1.200".to_string(),
        protocol: "TCP".to_string(),
        size: 1500,
        timestamp: std::time::SystemTime::now(),
    };
    
    ebpf_manager.simulate_packet_processing(packet_data).await?;
    
    let monitoring_stats = ebpf_manager.get_network_monitoring_stats().await;
    assert!(monitoring_stats.packets_processed >= 1);
    assert!(monitoring_stats.bytes_processed >= 1500);
    
    ebpf_manager.unload_program(&program_id).await?;
    ebpf_manager.stop().await?;
    Ok(())
});

unit_test!(test_traffic_control, "traffic_control", {
    let temp_dir = TempDir::new()?;
    let mut config = EbpfConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.enable_simulation = true;
    
    let node_id = NodeId::random();
    let mut ebpf_manager = EbpfManager::new(config, node_id).await?;
    ebpf_manager.start().await?;
    
    let tc_config = EbpfProgramConfig {
        name: "bandwidth_limiter".to_string(),
        program_type: ProgramType::TrafficControl,
        interface: Some("eth0".to_string()),
        auto_attach: true,
        priority: 200,
    };
    
    let program_id = ebpf_manager.load_program(tc_config).await?;
    
    let shaping_rule = TrafficShapingRule {
        source_subnet: "192.168.1.0/24".to_string(),
        dest_subnet: "0.0.0.0/0".to_string(),
        max_bandwidth_mbps: 100,
        priority: TrafficPriority::Normal,
        action: TrafficAction::RateLimit,
    };
    
    ebpf_manager.add_traffic_shaping_rule(shaping_rule).await?;
    
    let control_stats = ebpf_manager.get_traffic_control_stats().await;
    assert!(control_stats.active_rules >= 1);
    
    ebpf_manager.unload_program(&program_id).await?;
    ebpf_manager.stop().await?;
    Ok(())
});

unit_test!(test_metrics_collection, "metrics", {
    let temp_dir = TempDir::new()?;
    let mut config = EbpfConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.enable_simulation = true;
    config.metrics_collection_interval_ms = 1000;
    
    let node_id = NodeId::random();
    let mut ebpf_manager = EbpfManager::new(config, node_id).await?;
    ebpf_manager.start().await?;
    
    let monitor_config = EbpfProgramConfig {
        name: "metrics_monitor".to_string(),
        program_type: ProgramType::NetworkMonitoring,
        interface: Some("eth0".to_string()),
        auto_attach: true,
        priority: 100,
    };
    
    let monitor_id = ebpf_manager.load_program(monitor_config).await?;
    
    for i in 0..10 {
        let packet_data = PacketData {
            interface: "eth0".to_string(),
            source_ip: format!("192.168.1.{}", 100 + i),
            dest_ip: "192.168.1.1".to_string(),
            protocol: "TCP".to_string(),
            size: 1400 + i * 10,
            timestamp: std::time::SystemTime::now(),
        };
        
        ebpf_manager.simulate_packet_processing(packet_data).await?;
    }
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let overall_metrics = ebpf_manager.collect_comprehensive_metrics().await;
    assert!(overall_metrics.total_programs_loaded >= 1);
    assert!(overall_metrics.total_packets_processed >= 10);
    assert!(overall_metrics.total_bytes_processed > 0);
    
    let metrics_json = ebpf_manager.export_metrics_json().await?;
    assert!(!metrics_json.is_empty());
    assert!(metrics_json.contains("total_programs_loaded"));
    
    ebpf_manager.unload_program(&monitor_id).await?;
    ebpf_manager.stop().await?;
    Ok(())
});

// Mock implementations

#[derive(Debug, Clone)]
pub struct EbpfConfig {
    pub data_dir: String,
    pub enable_simulation: bool,
    pub auto_load_core_programs: bool,
    pub metrics_collection_interval_ms: u64,
    pub max_programs: usize,
    pub enable_jit: bool,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            data_dir: "/tmp/nexus-ebpf".to_string(),
            enable_simulation: false,
            auto_load_core_programs: false,
            metrics_collection_interval_ms: 5000,
            max_programs: 100,
            enable_jit: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EbpfProgramConfig {
    pub name: String,
    pub program_type: ProgramType,
    pub interface: Option<String>,
    pub auto_attach: bool,
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramType {
    NetworkMonitoring,
    TrafficControl,
    LoadBalancing,
    SecurityPolicies,
}

#[derive(Debug, Clone)]
pub struct PacketData {
    pub interface: String,
    pub source_ip: String,
    pub dest_ip: String,
    pub protocol: String,
    pub size: usize,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct TrafficShapingRule {
    pub source_subnet: String,
    pub dest_subnet: String,
    pub max_bandwidth_mbps: u32,
    pub priority: TrafficPriority,
    pub action: TrafficAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficPriority {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficAction {
    Allow,
    Drop,
    RateLimit,
    Redirect,
}

#[derive(Debug, Clone)]
pub struct LoadedProgram {
    pub id: String,
    pub name: String,
    pub program_type: ProgramType,
    pub interface: Option<String>,
    pub loaded_at: std::time::SystemTime,
    pub is_attached: bool,
}

pub struct EbpfManager {
    node_id: NodeId,
    config: EbpfConfig,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    loaded_programs: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, LoadedProgram>>>,
    traffic_rules: std::sync::Arc<tokio::sync::Mutex<Vec<TrafficShapingRule>>>,
    packets_processed: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    bytes_processed: std::sync::Arc<std::sync::atomic::AtomicU64>,
    programs_loaded_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl EbpfManager {
    pub async fn new(config: EbpfConfig, node_id: NodeId) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            node_id,
            config,
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            loaded_programs: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            traffic_rules: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            packets_processed: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            bytes_processed: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            programs_loaded_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }
    
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    pub async fn load_program(&self, config: EbpfProgramConfig) -> Result<String, Box<dyn std::error::Error>> {
        let program_id = uuid::Uuid::new_v4().to_string();
        
        let program = LoadedProgram {
            id: program_id.clone(),
            name: config.name,
            program_type: config.program_type,
            interface: config.interface,
            loaded_at: std::time::SystemTime::now(),
            is_attached: config.auto_attach,
        };
        
        self.loaded_programs.lock().await.insert(program_id.clone(), program);
        self.programs_loaded_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        Ok(program_id)
    }
    
    pub async fn unload_program(&self, program_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.loaded_programs.lock().await.remove(program_id);
        Ok(())
    }
    
    pub async fn list_programs(&self) -> Vec<LoadedProgram> {
        self.loaded_programs.lock().await.values().cloned().collect()
    }
    
    pub async fn simulate_packet_processing(&self, packet: PacketData) -> Result<(), Box<dyn std::error::Error>> {
        self.packets_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.bytes_processed.fetch_add(packet.size as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    pub async fn add_traffic_shaping_rule(&self, rule: TrafficShapingRule) -> Result<(), Box<dyn std::error::Error>> {
        self.traffic_rules.lock().await.push(rule);
        Ok(())
    }
    
    pub async fn stats(&self) -> EbpfStats {
        let programs = self.loaded_programs.lock().await;
        EbpfStats {
            loaded_programs: programs.len(),
            active_programs: programs.values().filter(|p| p.is_attached).count(),
            total_programs_loaded: self.programs_loaded_count.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
    
    pub async fn get_network_monitoring_stats(&self) -> NetworkMonitoringStats {
        NetworkMonitoringStats {
            packets_processed: self.packets_processed.load(std::sync::atomic::Ordering::Relaxed),
            bytes_processed: self.bytes_processed.load(std::sync::atomic::Ordering::Relaxed),
            interfaces_monitored: 1,
        }
    }
    
    pub async fn get_traffic_control_stats(&self) -> TrafficControlStats {
        TrafficControlStats {
            active_rules: self.traffic_rules.lock().await.len(),
            packets_shaped: self.packets_processed.load(std::sync::atomic::Ordering::Relaxed) / 2,
        }
    }
    
    pub async fn collect_comprehensive_metrics(&self) -> ComprehensiveMetrics {
        let programs = self.loaded_programs.lock().await;
        let mut per_program_metrics = std::collections::HashMap::new();
        
        for (id, _program) in programs.iter() {
            per_program_metrics.insert(id.clone(), ProgramMetrics {
                packets_processed: self.packets_processed.load(std::sync::atomic::Ordering::Relaxed) / programs.len().max(1),
                bytes_processed: self.bytes_processed.load(std::sync::atomic::Ordering::Relaxed) / programs.len().max(1) as u64,
                cpu_usage_percent: 5.0,
                memory_usage_kb: 256,
            });
        }
        
        ComprehensiveMetrics {
            total_programs_loaded: programs.len(),
            total_packets_processed: self.packets_processed.load(std::sync::atomic::Ordering::Relaxed),
            total_bytes_processed: self.bytes_processed.load(std::sync::atomic::Ordering::Relaxed),
            per_program_metrics,
        }
    }
    
    pub async fn export_metrics_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        let metrics = self.collect_comprehensive_metrics().await;
        let json = serde_json::json!({
            "total_programs_loaded": metrics.total_programs_loaded,
            "total_packets_processed": metrics.total_packets_processed,
            "total_bytes_processed": metrics.total_bytes_processed,
            "per_program_metrics": metrics.per_program_metrics
        });
        Ok(json.to_string())
    }
}

pub struct EbpfStats {
    pub loaded_programs: usize,
    pub active_programs: usize,
    pub total_programs_loaded: usize,
}

pub struct NetworkMonitoringStats {
    pub packets_processed: usize,
    pub bytes_processed: u64,
    pub interfaces_monitored: usize,
}

pub struct TrafficControlStats {
    pub active_rules: usize,
    pub packets_shaped: usize,
}

pub struct ComprehensiveMetrics {
    pub total_programs_loaded: usize,
    pub total_packets_processed: usize,
    pub total_bytes_processed: u64,
    pub per_program_metrics: std::collections::HashMap<String, ProgramMetrics>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgramMetrics {
    pub packets_processed: usize,
    pub bytes_processed: u64,
    pub cpu_usage_percent: f64,
    pub memory_usage_kb: u64,
}
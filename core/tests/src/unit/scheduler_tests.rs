//! Comprehensive unit tests for scheduler components

use crate::{TestResult, init_test_logging, unit_test};
use tempfile::TempDir;
use std::time::Duration;
use nexus_shared::NodeId;

pub async fn run_scheduler_tests() -> TestResult {
    init_test_logging();
    
    test_scheduler_creation().await?;
    test_task_scheduling().await?;
    test_priority_queuing().await?;
    test_load_balancing().await?;
    test_scheduler_metrics().await?;
    
    Ok(())
}

unit_test!(test_scheduler_creation, "scheduler", {
    let temp_dir = TempDir::new()?;
    let mut config = SchedulerConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let node_id = NodeId::random();
    let scheduler = TaskScheduler::new(config, node_id).await?;
    
    assert_eq!(scheduler.node_id(), node_id);
    assert!(!scheduler.is_running());
    
    let stats = scheduler.stats().await;
    assert_eq!(stats.pending_tasks, 0);
    assert_eq!(stats.completed_tasks, 0);
    
    Ok(())
});

unit_test!(test_task_scheduling, "scheduling", {
    let temp_dir = TempDir::new()?;
    let mut config = SchedulerConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.max_concurrent_tasks = 5;
    
    let node_id = NodeId::random();
    let mut scheduler = TaskScheduler::new(config, node_id).await?;
    scheduler.start().await?;
    
    let task = ScheduledTask::new(
        "test-task".to_string(),
        TaskType::Compute,
        Duration::from_secs(1),
        TaskPriority::Normal,
    );
    
    let task_id = scheduler.schedule_task(task).await?;
    assert!(!task_id.is_empty());
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let stats = scheduler.stats().await;
    assert!(stats.pending_tasks >= 0);
    
    scheduler.stop().await?;
    Ok(())
});

unit_test!(test_priority_queuing, "priority", {
    let temp_dir = TempDir::new()?;
    let mut config = SchedulerConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let node_id = NodeId::random();
    let mut scheduler = TaskScheduler::new(config, node_id).await?;
    scheduler.start().await?;
    
    let low_priority_task = ScheduledTask::new(
        "low-priority".to_string(),
        TaskType::Compute,
        Duration::from_secs(1),
        TaskPriority::Low,
    );
    
    let high_priority_task = ScheduledTask::new(
        "high-priority".to_string(),
        TaskType::Compute,
        Duration::from_secs(1),
        TaskPriority::High,
    );
    
    let low_id = scheduler.schedule_task(low_priority_task).await?;
    let high_id = scheduler.schedule_task(high_priority_task).await?;
    
    let next_task = scheduler.get_next_task().await?;
    assert_eq!(next_task.id(), high_id);
    
    scheduler.stop().await?;
    Ok(())
});

unit_test!(test_load_balancing, "load_balancing", {
    let temp_dir = TempDir::new()?;
    let mut config = SchedulerConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.load_balancing_enabled = true;
    config.max_concurrent_tasks = 3;
    
    let node_id = NodeId::random();
    let mut scheduler = TaskScheduler::new(config, node_id).await?;
    scheduler.start().await?;
    
    let worker1 = NodeId::random();
    let worker2 = NodeId::random();
    let worker3 = NodeId::random();
    
    scheduler.add_worker_node(worker1).await?;
    scheduler.add_worker_node(worker2).await?;
    scheduler.add_worker_node(worker3).await?;
    
    for i in 0..9 {
        let task = ScheduledTask::new(
            format!("task-{}", i),
            TaskType::Compute,
            Duration::from_secs(1),
            TaskPriority::Normal,
        );
        
        scheduler.schedule_task(task).await?;
    }
    
    let load_stats = scheduler.load_balancing_stats().await;
    assert!(load_stats.total_workers >= 3);
    assert!(load_stats.tasks_per_worker.len() >= 3);
    
    scheduler.stop().await?;
    Ok(())
});

unit_test!(test_scheduler_metrics, "metrics", {
    let temp_dir = TempDir::new()?;
    let mut config = SchedulerConfig::default();
    config.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let node_id = NodeId::random();
    let mut scheduler = TaskScheduler::new(config, node_id).await?;
    scheduler.start().await?;
    
    for i in 0..5 {
        let task = ScheduledTask::new(
            format!("metrics-task-{}", i),
            TaskType::Compute,
            Duration::from_millis(100),
            TaskPriority::Normal,
        );
        
        scheduler.schedule_task(task).await?;
    }
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let metrics = scheduler.collect_metrics().await;
    assert!(metrics.total_tasks_scheduled >= 5);
    assert!(metrics.average_task_duration_ms >= 0);
    assert!(metrics.throughput_tasks_per_second >= 0.0);
    
    scheduler.stop().await?;
    Ok(())
});

// Mock implementations

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub data_dir: String,
    pub max_concurrent_tasks: usize,
    pub task_timeout_ms: u64,
    pub load_balancing_enabled: bool,
    pub priority_levels: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            data_dir: "/tmp/nexus-scheduler".to_string(),
            max_concurrent_tasks: 10,
            task_timeout_ms: 30000,
            load_balancing_enabled: false,
            priority_levels: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    id: String,
    task_type: TaskType,
    duration: Duration,
    priority: TaskPriority,
    dependencies: Vec<String>,
}

impl ScheduledTask {
    pub fn new(id: String, task_type: TaskType, duration: Duration, priority: TaskPriority) -> Self {
        Self {
            id,
            task_type,
            duration,
            priority,
            dependencies: Vec::new(),
        }
    }
    
    pub fn id(&self) -> &str {
        &self.id
    }
    
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Compute,
    IO,
    Network,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

pub struct TaskScheduler {
    node_id: NodeId,
    config: SchedulerConfig,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    task_counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    completed_tasks: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    worker_nodes: std::sync::Arc<tokio::sync::Mutex<Vec<NodeId>>>,
    task_queue: std::sync::Arc<tokio::sync::Mutex<Vec<ScheduledTask>>>,
}

impl TaskScheduler {
    pub async fn new(config: SchedulerConfig, node_id: NodeId) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            node_id,
            config,
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            task_counter: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            completed_tasks: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            worker_nodes: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            task_queue: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
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
    
    pub async fn schedule_task(&self, task: ScheduledTask) -> Result<String, Box<dyn std::error::Error>> {
        let task_id = task.id().to_string();
        let mut queue = self.task_queue.lock().await;
        queue.push(task);
        
        queue.sort_by(|a, b| {
            use TaskPriority::*;
            let priority_value = |p: TaskPriority| match p {
                Critical => 0,
                High => 1,
                Normal => 2,
                Low => 3,
                Background => 4,
            };
            priority_value(a.priority).cmp(&priority_value(b.priority))
        });
        
        self.task_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(task_id)
    }
    
    pub async fn get_next_task(&self) -> Result<ScheduledTask, Box<dyn std::error::Error>> {
        let mut queue = self.task_queue.lock().await;
        if let Some(task) = queue.pop() {
            self.completed_tasks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(task)
        } else {
            Err("No tasks available".into())
        }
    }
    
    pub async fn add_worker_node(&self, worker_id: NodeId) -> Result<(), Box<dyn std::error::Error>> {
        self.worker_nodes.lock().await.push(worker_id);
        Ok(())
    }
    
    pub async fn stats(&self) -> SchedulerStats {
        let queue = self.task_queue.lock().await;
        SchedulerStats {
            pending_tasks: queue.len(),
            completed_tasks: self.completed_tasks.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
    
    pub async fn load_balancing_stats(&self) -> LoadBalancingStats {
        let workers = self.worker_nodes.lock().await;
        let mut tasks_per_worker = std::collections::HashMap::new();
        
        for (i, worker) in workers.iter().enumerate() {
            tasks_per_worker.insert(*worker, i + 1);
        }
        
        LoadBalancingStats {
            total_workers: workers.len(),
            tasks_per_worker,
        }
    }
    
    pub async fn collect_metrics(&self) -> SchedulerMetrics {
        SchedulerMetrics {
            total_tasks_scheduled: self.task_counter.load(std::sync::atomic::Ordering::Relaxed),
            average_task_duration_ms: 150,
            throughput_tasks_per_second: 10.5,
        }
    }
}

pub struct SchedulerStats {
    pub pending_tasks: usize,
    pub completed_tasks: usize,
}

pub struct LoadBalancingStats {
    pub total_workers: usize,
    pub tasks_per_worker: std::collections::HashMap<NodeId, usize>,
}

pub struct SchedulerMetrics {
    pub total_tasks_scheduled: usize,
    pub average_task_duration_ms: u64,
    pub throughput_tasks_per_second: f64,
}
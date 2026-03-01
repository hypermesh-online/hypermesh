// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Processing and Retrieval Workflows
//!
//! This module implements the core workflows for the Intelligence Layer,
//! orchestrating the flow of assets through all Phase 2 components.

use anyhow::Result;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, instrument, warn};

use crate::assets::pipeline::{Asset, ProcessedAsset};

/// Result type for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Workflow errors
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Pipeline processing failed: {0}")]
    PipelineFailed(String),

    #[error("Storage operation failed: {0}")]
    StorageFailed(String),

    #[error("Network operation failed: {0}")]
    NetworkFailed(String),

    #[error("Privacy validation failed: {0}")]
    PrivacyFailed(String),

    #[error("Timeout exceeded: {0}")]
    Timeout(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Workflow error: {0}")]
    Workflow(#[from] anyhow::Error),
}

/// Metrics for workflow execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    /// Total executions
    pub total_executions: u64,

    /// Successful executions
    pub successful_executions: u64,

    /// Failed executions
    pub failed_executions: u64,

    /// Average execution time (ms)
    pub avg_execution_time_ms: u64,

    /// Minimum execution time (ms)
    pub min_execution_time_ms: u64,

    /// Maximum execution time (ms)
    pub max_execution_time_ms: u64,

    /// Stage-wise timing breakdown
    pub stage_timings: HashMap<String, u64>,
}

/// Asset workflow orchestrator
pub struct AssetWorkflow {
    /// Maximum concurrent operations
    concurrency_limit: Arc<Semaphore>,

    /// Workflow metrics
    metrics: Arc<RwLock<WorkflowMetrics>>,

    /// Processing timeout
    processing_timeout: Duration,

    /// Retrieval timeout
    _retrieval_timeout: Duration,
}

impl AssetWorkflow {
    /// Create new asset workflow
    pub fn new(
        max_concurrent: usize,
        processing_timeout: Duration,
        retrieval_timeout: Duration,
    ) -> Self {
        Self {
            concurrency_limit: Arc::new(Semaphore::new(max_concurrent)),
            metrics: Arc::new(RwLock::new(WorkflowMetrics::default())),
            processing_timeout,
            _retrieval_timeout: retrieval_timeout,
        }
    }

    /// Execute batch processing workflow
    #[instrument(skip(self, assets, processor))]
    pub async fn batch_process<F, Fut>(
        &self,
        assets: Vec<Asset>,
        processor: F,
    ) -> WorkflowResult<Vec<ProcessedAsset>>
    where
        F: Fn(Asset) -> Fut + Clone + Send + Sync,
        Fut: std::future::Future<Output = Result<ProcessedAsset>> + Send,
    {
        let start = Instant::now();
        info!("Starting batch processing for {} assets", assets.len());

        // Process assets concurrently with semaphore limiting
        let results = stream::iter(assets)
            .map(|asset| {
                let processor = processor.clone();
                let semaphore = self.concurrency_limit.clone();
                let timeout = self.processing_timeout;

                async move {
                    // Acquire permit for concurrency control
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| WorkflowError::ResourceExhausted(e.to_string()))?;

                    // Process with timeout
                    tokio::time::timeout(timeout, processor(asset))
                        .await
                        .map_err(|_| WorkflowError::Timeout("Processing timeout".to_string()))?
                        .map_err(|e| WorkflowError::PipelineFailed(e.to_string()))
                }
            })
            .buffer_unordered(10) // Process up to 10 concurrently
            .collect::<Vec<_>>()
            .await;

        // Collect results and errors
        let mut processed = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(asset) => processed.push(asset),
                Err(e) => errors.push(e),
            }
        }

        // Update metrics
        self.update_metrics(start.elapsed(), errors.is_empty())
            .await;

        if !errors.is_empty() {
            warn!("Batch processing completed with {} errors", errors.len());
            return Err(WorkflowError::Workflow(anyhow::anyhow!(
                "Batch processing failed with {} errors",
                errors.len()
            )));
        }

        info!(
            "Batch processing completed successfully in {:?}",
            start.elapsed()
        );

        Ok(processed)
    }

    /// Update workflow metrics
    async fn update_metrics(&self, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;

        metrics.total_executions += 1;
        if success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }

        let time_ms = duration.as_millis() as u64;

        // Update min/max
        if metrics.min_execution_time_ms == 0 || time_ms < metrics.min_execution_time_ms {
            metrics.min_execution_time_ms = time_ms;
        }
        if time_ms > metrics.max_execution_time_ms {
            metrics.max_execution_time_ms = time_ms;
        }

        // Update average
        if metrics.total_executions == 1 {
            metrics.avg_execution_time_ms = time_ms;
        } else {
            metrics.avg_execution_time_ms =
                (metrics.avg_execution_time_ms * (metrics.total_executions - 1) + time_ms)
                    / metrics.total_executions;
        }
    }

    /// Get workflow metrics
    pub async fn get_metrics(&self) -> WorkflowMetrics {
        self.metrics.read().await.clone()
    }
}

/// Processing workflow for assets
pub struct ProcessingWorkflow {
    /// Workflow stages
    stages: Vec<ProcessingStage>,

    /// Stage metrics
    stage_metrics: Arc<RwLock<HashMap<String, StageMetrics>>>,
}

/// Processing stage
#[derive(Clone)]
pub struct ProcessingStage {
    /// Stage name
    pub name: String,

    /// Stage timeout
    pub timeout: Duration,

    /// Retry count
    pub retry_count: usize,

    /// Retry delay
    pub retry_delay: Duration,
}

/// Stage execution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageMetrics {
    /// Total executions
    pub executions: u64,

    /// Successful executions
    pub successes: u64,

    /// Failed executions
    pub failures: u64,

    /// Average duration (ms)
    pub avg_duration_ms: u64,

    /// Total retries
    pub total_retries: u64,
}

impl Default for ProcessingWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingWorkflow {
    /// Create new processing workflow
    pub fn new() -> Self {
        let stages = vec![
            ProcessingStage {
                name: "validation".to_string(),
                timeout: Duration::from_secs(5),
                retry_count: 2,
                retry_delay: Duration::from_millis(100),
            },
            ProcessingStage {
                name: "compression".to_string(),
                timeout: Duration::from_secs(10),
                retry_count: 1,
                retry_delay: Duration::from_millis(500),
            },
            ProcessingStage {
                name: "encryption".to_string(),
                timeout: Duration::from_secs(10),
                retry_count: 1,
                retry_delay: Duration::from_millis(500),
            },
            ProcessingStage {
                name: "sharding".to_string(),
                timeout: Duration::from_secs(10),
                retry_count: 1,
                retry_delay: Duration::from_millis(500),
            },
            ProcessingStage {
                name: "distribution".to_string(),
                timeout: Duration::from_secs(15),
                retry_count: 3,
                retry_delay: Duration::from_secs(1),
            },
        ];

        Self {
            stages,
            stage_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute processing workflow with stage tracking
    #[instrument(skip(self, processor))]
    pub async fn execute<F, Fut, T>(&self, stage_name: &str, processor: F) -> WorkflowResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let stage = self
            .stages
            .iter()
            .find(|s| s.name == stage_name)
            .ok_or_else(|| WorkflowError::InvalidState(format!("Unknown stage: {stage_name}")))?;

        let start = Instant::now();
        let mut retries = 0;
        let mut last_error = None;

        // Execute with retries
        while retries <= stage.retry_count {
            if retries > 0 {
                debug!("Retrying stage {} (attempt {})", stage_name, retries + 1);
                tokio::time::sleep(stage.retry_delay).await;
            }

            match tokio::time::timeout(stage.timeout, processor()).await {
                Ok(Ok(result)) => {
                    // Success - update metrics
                    self.update_stage_metrics(stage_name, start.elapsed(), true, retries)
                        .await;
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    last_error = Some(WorkflowError::Workflow(e));
                    retries += 1;
                }
                Err(_) => {
                    last_error = Some(WorkflowError::Timeout(format!(
                        "Stage {} timeout after {:?}",
                        stage_name, stage.timeout
                    )));
                    retries += 1;
                }
            }
        }

        // All retries exhausted - update metrics and return error
        self.update_stage_metrics(stage_name, start.elapsed(), false, retries - 1)
            .await;

        Err(last_error.unwrap_or_else(|| {
            WorkflowError::Workflow(anyhow::anyhow!(
                "Stage {stage_name} failed after {retries} retries"
            ))
        }))
    }

    /// Update stage metrics
    async fn update_stage_metrics(
        &self,
        stage_name: &str,
        duration: Duration,
        success: bool,
        retries: usize,
    ) {
        let mut metrics = self.stage_metrics.write().await;
        let stage_metrics = metrics.entry(stage_name.to_string()).or_default();

        stage_metrics.executions += 1;
        if success {
            stage_metrics.successes += 1;
        } else {
            stage_metrics.failures += 1;
        }

        stage_metrics.total_retries += retries as u64;

        let duration_ms = duration.as_millis() as u64;
        if stage_metrics.executions == 1 {
            stage_metrics.avg_duration_ms = duration_ms;
        } else {
            stage_metrics.avg_duration_ms =
                (stage_metrics.avg_duration_ms * (stage_metrics.executions - 1) + duration_ms)
                    / stage_metrics.executions;
        }
    }

    /// Get stage metrics
    pub async fn get_stage_metrics(&self) -> HashMap<String, StageMetrics> {
        self.stage_metrics.read().await.clone()
    }
}

/// Retrieval workflow for assets
pub struct RetrievalWorkflow {
    /// Cache for recently retrieved assets
    cache: Arc<RwLock<HashMap<String, CachedAsset>>>,

    /// Cache TTL
    cache_ttl: Duration,

    /// Maximum cache size
    max_cache_size: usize,
}

/// Cached asset entry
#[derive(Clone)]
struct CachedAsset {
    /// Asset data
    data: Vec<u8>,

    /// Cache timestamp
    cached_at: Instant,

    /// Access count
    access_count: u64,
}

impl RetrievalWorkflow {
    /// Create new retrieval workflow
    pub fn new(cache_ttl: Duration, max_cache_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            max_cache_size,
        }
    }

    /// Execute retrieval with caching
    #[instrument(skip(self, retriever))]
    pub async fn retrieve_with_cache<F, Fut>(
        &self,
        asset_id: &str,
        retriever: F,
    ) -> WorkflowResult<Vec<u8>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<u8>>>,
    {
        // Check cache first
        if let Some(data) = self.get_from_cache(asset_id).await {
            debug!("Cache hit for asset {}", asset_id);
            return Ok(data);
        }

        debug!("Cache miss for asset {} - retrieving", asset_id);

        // Retrieve from source
        let data = retriever()
            .await
            .map_err(|e| WorkflowError::StorageFailed(e.to_string()))?;

        // Store in cache
        self.store_in_cache(asset_id.to_string(), data.clone())
            .await;

        Ok(data)
    }

    /// Get asset from cache
    async fn get_from_cache(&self, asset_id: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.write().await;

        if let Some(cached) = cache.get_mut(asset_id) {
            // Check if cache entry is still valid
            if cached.cached_at.elapsed() < self.cache_ttl {
                cached.access_count += 1;
                return Some(cached.data.clone());
            } else {
                // Remove expired entry
                cache.remove(asset_id);
            }
        }

        None
    }

    /// Store asset in cache
    async fn store_in_cache(&self, asset_id: String, data: Vec<u8>) {
        let mut cache = self.cache.write().await;

        // Evict least recently used if cache is full
        if cache.len() >= self.max_cache_size {
            self.evict_lru(&mut cache);
        }

        cache.insert(
            asset_id,
            CachedAsset {
                data,
                cached_at: Instant::now(),
                access_count: 1,
            },
        );
    }

    /// Evict least recently used entry
    fn evict_lru(&self, cache: &mut HashMap<String, CachedAsset>) {
        if let Some((lru_key, _)) = cache
            .iter()
            .min_by_key(|(_, v)| v.access_count)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            cache.remove(&lru_key);
        }
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;

        let total_size: usize = cache.values().map(|v| v.data.len()).sum();
        let total_accesses: u64 = cache.values().map(|v| v.access_count).sum();

        CacheStats {
            entries: cache.len(),
            total_size_bytes: total_size,
            total_accesses,
            avg_access_count: if cache.is_empty() {
                0.0
            } else {
                total_accesses as f64 / cache.len() as f64
            },
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,

    /// Total size in bytes
    pub total_size_bytes: usize,

    /// Total access count
    pub total_accesses: u64,

    /// Average access count per entry
    pub avg_access_count: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::distribution::DistributedAsset;

    #[tokio::test]
    async fn test_asset_workflow_batch_processing() {
        let workflow = AssetWorkflow::new(10, Duration::from_secs(30), Duration::from_secs(10));

        let assets = vec![
            Asset {
                id: "asset1".to_string(),
                data: vec![1, 2, 3],
                metadata: Default::default(),
            },
            Asset {
                id: "asset2".to_string(),
                data: vec![4, 5, 6],
                metadata: Default::default(),
            },
        ];

        let processor = |asset: Asset| async move {
            // Simulate processing
            let asset_id = asset.id.clone();
            Ok(ProcessedAsset {
                asset_id: asset.id,
                shards: vec![],
                decryption_key: crate::assets::pipeline::DecryptionKey::Aes(
                    crate::assets::pipeline::AesKey {
                        key: vec![0u8; 32],
                        nonce: vec![0u8; 12],
                    },
                ),
                distributed: DistributedAsset {
                    asset_id,
                    placements: vec![],
                    metadata: Default::default(),
                },
                stats: Default::default(),
            })
        };

        let results = workflow.batch_process(assets, processor).await;
        assert!(results.is_ok());
        assert_eq!(results.expect("test: expected success").len(), 2);

        let metrics = workflow.get_metrics().await;
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.successful_executions, 1);
    }

    #[tokio::test]
    async fn test_processing_workflow_stages() {
        let workflow = ProcessingWorkflow::new();

        let result = workflow
            .execute("validation", || async {
                Ok::<_, anyhow::Error>("validated")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.expect("test: expected success"), "validated");

        let metrics = workflow.get_stage_metrics().await;
        assert!(metrics.contains_key("validation"));
        assert_eq!(metrics["validation"].executions, 1);
        assert_eq!(metrics["validation"].successes, 1);
    }

    #[tokio::test]
    async fn test_retrieval_workflow_caching() {
        let workflow = RetrievalWorkflow::new(Duration::from_secs(60), 10);

        let mut call_count = 0;

        // First call - should retrieve
        let data1 = workflow
            .retrieve_with_cache("asset1", || async {
                call_count += 1;
                Ok(vec![1, 2, 3])
            })
            .await
            .expect("test: expected success");

        assert_eq!(data1, vec![1, 2, 3]);
        assert_eq!(call_count, 1);

        // Second call - should use cache
        let data2 = workflow
            .retrieve_with_cache("asset1", || async {
                call_count += 1;
                Ok(vec![1, 2, 3])
            })
            .await
            .expect("test: expected success");

        assert_eq!(data2, vec![1, 2, 3]);
        assert_eq!(call_count, 1); // Should not have called retriever again

        let stats = workflow.cache_stats().await;
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.total_accesses, 2);
    }
}

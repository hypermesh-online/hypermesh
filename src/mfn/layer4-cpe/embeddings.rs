//! Context Embedding and Similarity Search
//!
//! This module implements context vector embedding generation and fast similarity search
//! for pattern matching and context retrieval in the CPE system.

use anyhow::Result;
use nalgebra::{DVector, DMatrix};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use lru::LruCache;
use probabilistic_collections::similarity::SimHash;

use crate::ContextVector;

/// Configuration for context embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub dimension: usize,
    pub similarity_threshold: f64,
    pub max_neighbors: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimension: 256,
            similarity_threshold: 0.8,
            max_neighbors: 10,
        }
    }
}

/// Context embedding representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEmbedding {
    pub vector: Vec<f32>,
    pub context_id: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, f32>,
    pub pattern_hash: u64,
}

impl ContextEmbedding {
    pub fn new(vector: Vec<f32>, context_id: String) -> Self {
        let pattern_hash = Self::compute_pattern_hash(&vector);
        
        Self {
            vector,
            context_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: HashMap::new(),
            pattern_hash,
        }
    }
    
    pub fn with_metadata(mut self, key: String, value: f32) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    fn compute_pattern_hash(vector: &[f32]) -> u64 {
        // Use SimHash for pattern similarity hashing
        let mut hasher = SimHash::new();
        for &val in vector {
            hasher.write(&val.to_le_bytes());
        }
        hasher.finish()
    }
}

/// Similarity search result
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub embedding: ContextEmbedding,
    pub similarity_score: f32,
    pub distance: f32,
}

impl PartialEq for SimilarityResult {
    fn eq(&self, other: &Self) -> bool {
        self.similarity_score == other.similarity_score
    }
}

impl Eq for SimilarityResult {}

impl PartialOrd for SimilarityResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.similarity_score.partial_cmp(&other.similarity_score)
    }
}

impl Ord for SimilarityResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.similarity_score.partial_cmp(&other.similarity_score)
            .unwrap_or(Ordering::Equal)
    }
}

/// Context embedder for generating and managing embeddings
pub struct ContextEmbedder {
    config: EmbeddingConfig,
    embedding_store: Arc<RwLock<HashMap<String, ContextEmbedding>>>,
    similarity_cache: Arc<RwLock<LruCache<String, Vec<SimilarityResult>>>>,
    pattern_index: Arc<RwLock<HashMap<u64, Vec<String>>>>, // Hash -> context_ids
    
    // Embedding transformation matrix (learned)
    transformation_matrix: Arc<RwLock<Option<DMatrix<f32>>>>,
    
    // Statistics
    embedding_count: Arc<std::sync::atomic::AtomicU64>,
    cache_hits: Arc<std::sync::atomic::AtomicU64>,
    cache_misses: Arc<std::sync::atomic::AtomicU64>,
}

impl ContextEmbedder {
    /// Create a new context embedder
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!("Initializing ContextEmbedder with dimension {}", config.dimension);
        
        let similarity_cache = Arc::new(RwLock::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap())));
        
        Ok(Self {
            config,
            embedding_store: Arc::new(RwLock::new(HashMap::new())),
            similarity_cache,
            pattern_index: Arc::new(RwLock::new(HashMap::new())),
            transformation_matrix: Arc::new(RwLock::new(None)),
            embedding_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cache_hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cache_misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }
    
    /// Embed a single context vector
    pub async fn embed_context(&mut self, context: &ContextVector) -> Result<ContextEmbedding> {
        let context_id = format!("ctx_{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos());
        
        // Generate embedding (with potential dimensionality reduction/expansion)
        let embedding_vector = self.generate_embedding(&context.features).await?;
        
        let embedding = ContextEmbedding::new(embedding_vector, context_id.clone())
            .with_metadata("flow_confidence".to_string(), 
                         context.metadata.get("confidence").copied().unwrap_or(0.5));
        
        // Store embedding
        {
            let mut store = self.embedding_store.write().await;
            store.insert(context_id.clone(), embedding.clone());
        }
        
        // Update pattern index
        {
            let mut index = self.pattern_index.write().await;
            index.entry(embedding.pattern_hash)
                .or_default()
                .push(context_id);
        }
        
        self.embedding_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        debug!("Context embedded with ID: {}", context_id);
        Ok(embedding)
    }
    
    /// Embed multiple contexts in batch
    pub async fn embed_contexts(&mut self, contexts: &[ContextVector]) -> Result<Vec<ContextEmbedding>> {
        let mut embeddings = Vec::with_capacity(contexts.len());
        
        for context in contexts {
            let embedding = self.embed_context(context).await?;
            embeddings.push(embedding);
        }
        
        debug!("Batch embedded {} contexts", contexts.len());
        Ok(embeddings)
    }
    
    /// Find similar contexts using various similarity metrics
    pub async fn find_similar_contexts(
        &self,
        query_embedding: &ContextEmbedding,
        k: usize,
    ) -> Result<Vec<SimilarityResult>> {
        let cache_key = format!("sim_{}_{}", query_embedding.context_id, k);
        
        // Check cache first
        {
            let mut cache = self.similarity_cache.write().await;
            if let Some(cached_results) = cache.get(&cache_key) {
                self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(cached_results.clone());
            }
        }
        
        self.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // First, try pattern-based similarity using hash index
        let mut candidates = self.find_pattern_candidates(query_embedding).await?;
        
        // If not enough candidates, fall back to exhaustive search
        if candidates.len() < k {
            let additional = self.exhaustive_similarity_search(query_embedding, k).await?;
            candidates.extend(additional);
        }
        
        // Sort by similarity and take top-k
        candidates.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        candidates.truncate(k);
        
        // Cache results
        {
            let mut cache = self.similarity_cache.write().await;
            cache.put(cache_key, candidates.clone());
        }
        
        debug!("Found {} similar contexts", candidates.len());
        Ok(candidates)
    }
    
    /// Find similar contexts to a raw feature vector
    pub async fn find_similar_to_features(
        &mut self,
        features: &[f32],
        k: usize,
    ) -> Result<Vec<SimilarityResult>> {
        let embedding_vector = self.generate_embedding(features).await?;
        let query_embedding = ContextEmbedding::new(embedding_vector, "query_temp".to_string());
        
        self.find_similar_contexts(&query_embedding, k).await
    }
    
    /// Generate embedding from raw features
    async fn generate_embedding(&self, features: &[f32]) -> Result<Vec<f32>> {
        let target_dim = self.config.dimension;
        
        // Apply learned transformation if available
        let transformation = self.transformation_matrix.read().await;
        if let Some(ref matrix) = *transformation {
            let input = DVector::from_vec(features.to_vec());
            let transformed = matrix * input;
            return Ok(transformed.data.as_vec().clone());
        }
        
        // Default embedding generation
        let mut embedding = Vec::with_capacity(target_dim);
        
        if features.len() == target_dim {
            // Direct copy if dimensions match
            embedding.extend_from_slice(features);
        } else if features.len() < target_dim {
            // Expand with interpolation and padding
            embedding.extend_from_slice(features);
            
            // Interpolate additional features
            let expansion_ratio = (target_dim - features.len()) as f32 / features.len() as f32;
            for i in 0..features.len() {
                for j in 1..=(expansion_ratio.ceil() as usize) {
                    if embedding.len() >= target_dim { break; }
                    
                    let next_idx = (i + 1) % features.len();
                    let interpolated = features[i] + (features[next_idx] - features[i]) * (j as f32 / (expansion_ratio + 1.0));
                    embedding.push(interpolated);
                }
            }
            
            // Pad if still needed
            while embedding.len() < target_dim {
                embedding.push(0.0);
            }
        } else {
            // Reduce dimensionality using PCA-like approach
            let chunk_size = features.len() / target_dim;
            let remainder = features.len() % target_dim;
            
            for i in 0..target_dim {
                let start = i * chunk_size;
                let end = if i == target_dim - 1 {
                    start + chunk_size + remainder
                } else {
                    start + chunk_size
                };
                
                let chunk_mean = features[start..end].iter().sum::<f32>() / (end - start) as f32;
                embedding.push(chunk_mean);
            }
        }
        
        // L2 normalize the embedding
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        
        Ok(embedding)
    }
    
    /// Find candidates using pattern hash similarity
    async fn find_pattern_candidates(&self, query: &ContextEmbedding) -> Result<Vec<SimilarityResult>> {
        let mut candidates = Vec::new();
        
        // Look for contexts with similar pattern hashes
        let pattern_index = self.pattern_index.read().await;
        let embedding_store = self.embedding_store.read().await;
        
        // Get contexts with the same pattern hash
        if let Some(context_ids) = pattern_index.get(&query.pattern_hash) {
            for context_id in context_ids {
                if let Some(embedding) = embedding_store.get(context_id) {
                    let similarity = Self::cosine_similarity(&query.vector, &embedding.vector);
                    if similarity >= self.config.similarity_threshold as f32 {
                        candidates.push(SimilarityResult {
                            embedding: embedding.clone(),
                            similarity_score: similarity,
                            distance: 1.0 - similarity,
                        });
                    }
                }
            }
        }
        
        // Also check similar pattern hashes (Hamming distance <= 2)
        for (&hash, context_ids) in pattern_index.iter() {
            let hamming_distance = (query.pattern_hash ^ hash).count_ones();
            if hamming_distance <= 2 && hash != query.pattern_hash {
                for context_id in context_ids {
                    if let Some(embedding) = embedding_store.get(context_id) {
                        let similarity = Self::cosine_similarity(&query.vector, &embedding.vector);
                        if similarity >= self.config.similarity_threshold as f32 {
                            candidates.push(SimilarityResult {
                                embedding: embedding.clone(),
                                similarity_score: similarity,
                                distance: 1.0 - similarity,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(candidates)
    }
    
    /// Exhaustive similarity search across all embeddings
    async fn exhaustive_similarity_search(&self, query: &ContextEmbedding, k: usize) -> Result<Vec<SimilarityResult>> {
        let mut heap = BinaryHeap::new();
        
        let embedding_store = self.embedding_store.read().await;
        
        for embedding in embedding_store.values() {
            if embedding.context_id == query.context_id {
                continue; // Skip self
            }
            
            let similarity = Self::cosine_similarity(&query.vector, &embedding.vector);
            
            if heap.len() < k {
                heap.push(std::cmp::Reverse(SimilarityResult {
                    embedding: embedding.clone(),
                    similarity_score: similarity,
                    distance: 1.0 - similarity,
                }));
            } else if similarity > heap.peek().unwrap().0.similarity_score {
                heap.pop();
                heap.push(std::cmp::Reverse(SimilarityResult {
                    embedding: embedding.clone(),
                    similarity_score: similarity,
                    distance: 1.0 - similarity,
                }));
            }
        }
        
        let mut results: Vec<SimilarityResult> = heap.into_iter().map(|r| r.0).collect();
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        
        Ok(results)
    }
    
    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let min_len = a.len().min(b.len());
        if min_len == 0 {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).take(min_len).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().take(min_len).map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a * norm_b == 0.0 {
            0.0
        } else {
            (dot_product / (norm_a * norm_b)).max(0.0).min(1.0)
        }
    }
    
    /// Compute Euclidean distance between two vectors
    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        let min_len = a.len().min(b.len());
        a.iter().zip(b.iter()).take(min_len)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
    
    /// Learn embedding transformation from training data
    pub async fn learn_embedding_transformation(
        &mut self,
        training_pairs: &[(Vec<f32>, Vec<f32>)], // (input_features, target_embedding)
    ) -> Result<()> {
        if training_pairs.is_empty() {
            return Err(anyhow::anyhow!("No training data provided"));
        }
        
        let input_dim = training_pairs[0].0.len();
        let output_dim = training_pairs[0].1.len();
        
        // Create input and output matrices
        let mut input_matrix = DMatrix::zeros(training_pairs.len(), input_dim);
        let mut output_matrix = DMatrix::zeros(training_pairs.len(), output_dim);
        
        for (i, (input, output)) in training_pairs.iter().enumerate() {
            for (j, &val) in input.iter().enumerate() {
                input_matrix[(i, j)] = val;
            }
            for (j, &val) in output.iter().enumerate() {
                output_matrix[(i, j)] = val;
            }
        }
        
        // Solve for transformation matrix using pseudo-inverse
        // T = (X^T * X)^-1 * X^T * Y
        let xt = input_matrix.transpose();
        let xtx = &xt * &input_matrix;
        
        if let Some(xtx_inv) = xtx.try_inverse() {
            let transformation = xtx_inv * xt * output_matrix;
            
            *self.transformation_matrix.write().await = Some(transformation.transpose());
            info!("Learned embedding transformation: {}x{} -> {}x{}", 
                  training_pairs.len(), input_dim, training_pairs.len(), output_dim);
        } else {
            return Err(anyhow::anyhow!("Failed to compute matrix inverse for transformation"));
        }
        
        Ok(())
    }
    
    /// Get embedding statistics
    pub async fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        
        stats.insert("total_embeddings".to_string(), 
                    self.embedding_count.load(std::sync::atomic::Ordering::Relaxed) as f64);
        
        stats.insert("cache_hits".to_string(),
                    self.cache_hits.load(std::sync::atomic::Ordering::Relaxed) as f64);
        
        stats.insert("cache_misses".to_string(),
                    self.cache_misses.load(std::sync::atomic::Ordering::Relaxed) as f64);
        
        let cache_hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed) as f64;
        let cache_misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed) as f64;
        let total_queries = cache_hits + cache_misses;
        
        if total_queries > 0.0 {
            stats.insert("cache_hit_rate".to_string(), cache_hits / total_queries);
        }
        
        let store = self.embedding_store.read().await;
        stats.insert("stored_embeddings".to_string(), store.len() as f64);
        
        let index = self.pattern_index.read().await;
        stats.insert("pattern_buckets".to_string(), index.len() as f64);
        
        stats
    }
    
    /// Clear embedding cache and reset statistics
    pub async fn clear_cache(&mut self) {
        let mut cache = self.similarity_cache.write().await;
        cache.clear();
        
        self.cache_hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_misses.store(0, std::sync::atomic::Ordering::Relaxed);
        
        info!("Embedding cache cleared");
    }
}

/// Similarity search interface
pub struct SimilaritySearch {
    embedder: ContextEmbedder,
}

impl SimilaritySearch {
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let embedder = ContextEmbedder::new(config).await?;
        Ok(Self { embedder })
    }
    
    pub async fn search(&mut self, query: &[f32], k: usize) -> Result<Vec<SimilarityResult>> {
        self.embedder.find_similar_to_features(query, k).await
    }
    
    pub async fn add_context(&mut self, context: &ContextVector) -> Result<()> {
        self.embedder.embed_context(context).await?;
        Ok(())
    }
    
    pub async fn get_statistics(&self) -> HashMap<String, f64> {
        self.embedder.get_statistics().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextVector;
    
    fn create_test_context(features: Vec<f32>) -> ContextVector {
        let flow_key = [0u8; 32];
        ContextVector::new(flow_key, features)
    }
    
    #[tokio::test]
    async fn test_context_embedder_creation() {
        let config = EmbeddingConfig::default();
        let embedder = ContextEmbedder::new(config).await;
        assert!(embedder.is_ok());
    }
    
    #[tokio::test]
    async fn test_embedding_generation() {
        let config = EmbeddingConfig {
            dimension: 128,
            similarity_threshold: 0.8,
            max_neighbors: 10,
        };
        
        let mut embedder = ContextEmbedder::new(config).await.unwrap();
        let context = create_test_context(vec![0.1, 0.2, 0.3, 0.4, 0.5]);
        
        let result = embedder.embed_context(&context).await;
        assert!(result.is_ok());
        
        let embedding = result.unwrap();
        assert_eq!(embedding.vector.len(), 128);
    }
    
    #[tokio::test]
    async fn test_similarity_search() {
        let config = EmbeddingConfig {
            dimension: 64,
            similarity_threshold: 0.7,
            max_neighbors: 5,
        };
        
        let mut embedder = ContextEmbedder::new(config).await.unwrap();
        
        // Add some contexts
        let contexts = vec![
            create_test_context(vec![1.0, 2.0, 3.0]),
            create_test_context(vec![1.1, 2.1, 3.1]),
            create_test_context(vec![5.0, 6.0, 7.0]),
        ];
        
        for context in &contexts {
            embedder.embed_context(context).await.unwrap();
        }
        
        // Search for similar contexts
        let query_features = vec![1.05, 2.05, 3.05];
        let results = embedder.find_similar_to_features(&query_features, 2).await;
        
        assert!(results.is_ok());
        let similar = results.unwrap();
        assert!(!similar.is_empty());
    }
    
    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let similarity = ContextEmbedder::cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 1e-6);
        
        let c = vec![0.0, 0.0, 0.0];
        let similarity2 = ContextEmbedder::cosine_similarity(&a, &c);
        assert_eq!(similarity2, 0.0);
    }
    
    #[tokio::test]
    async fn test_batch_embedding() {
        let config = EmbeddingConfig {
            dimension: 32,
            similarity_threshold: 0.8,
            max_neighbors: 5,
        };
        
        let mut embedder = ContextEmbedder::new(config).await.unwrap();
        
        let contexts = vec![
            create_test_context(vec![0.1; 10]),
            create_test_context(vec![0.2; 10]),
            create_test_context(vec![0.3; 10]),
        ];
        
        let embeddings = embedder.embed_contexts(&contexts).await;
        assert!(embeddings.is_ok());
        
        let emb_vec = embeddings.unwrap();
        assert_eq!(emb_vec.len(), 3);
        
        for embedding in &emb_vec {
            assert_eq!(embedding.vector.len(), 32);
        }
    }
}
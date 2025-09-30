//! Package Manager for P2P Distribution
//!
//! Handles package storage, chunking, and peer-to-peer transfers

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use futures::stream::{self, StreamExt};

use crate::assets::{AssetPackage, AssetPackageId};
use super::{
    ContentStore, ContentAddress,
    content_addressing::{MerkleTree, ContentChunker, CompressionType, Chunk},
    stoq_transport::{StoqTransportLayer, PackageInfo, ChunkData, RequestType, ResponseData, PackageMetadata},
    dht::NodeId,
};

/// Package manager for handling package storage and transfers
pub struct PackageManager {
    /// Content store for chunks
    content_store: Arc<ContentStore>,
    /// Storage directory
    storage_dir: PathBuf,
    /// Package metadata cache
    metadata_cache: Arc<RwLock<HashMap<AssetPackageId, PackageMetadata>>>,
    /// Chunk cache
    chunk_cache: Arc<RwLock<ChunkCache>>,
    /// Download semaphore for concurrency control
    download_semaphore: Arc<Semaphore>,
    /// Upload semaphore for concurrency control
    upload_semaphore: Arc<Semaphore>,
    /// Content chunker
    chunker: ContentChunker,
}

/// Chunk cache for efficient retrieval
struct ChunkCache {
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size
    current_size: usize,
    /// Cached chunks
    chunks: HashMap<(AssetPackageId, usize), Vec<u8>>,
    /// Access timestamps for LRU
    access_times: HashMap<(AssetPackageId, usize), std::time::Instant>,
}

impl PackageManager {
    /// Create a new package manager
    pub async fn new(
        content_store: Arc<ContentStore>,
        storage_dir: PathBuf,
    ) -> Result<Self> {
        // Create storage directory
        tokio::fs::create_dir_all(&storage_dir).await
            .context("Failed to create storage directory")?;

        Ok(Self {
            content_store,
            storage_dir,
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            chunk_cache: Arc::new(RwLock::new(ChunkCache::new(100 * 1024 * 1024))), // 100MB cache
            download_semaphore: Arc::new(Semaphore::new(10)), // Max 10 concurrent downloads
            upload_semaphore: Arc::new(Semaphore::new(10)),   // Max 10 concurrent uploads
            chunker: ContentChunker::new(1024 * 1024, CompressionType::Zstd), // 1MB chunks with Zstd
        })
    }

    /// Store a package and return content addresses
    pub async fn store_package(&self, package: &AssetPackage) -> Result<Vec<String>> {
        let package_id = package.get_package_id();

        // Serialize package
        let package_data = bincode::serialize(package)
            .context("Failed to serialize package")?;

        // Create chunks
        let chunks = self.chunker.chunk_data(&package_data)
            .context("Failed to chunk package data")?;

        // Store chunks in content store
        let mut content_addresses = Vec::new();
        for chunk in &chunks {
            let address = ContentAddress::from_data(&chunk.data);
            self.content_store.storage
                .store_chunk(&address, &chunk.data)
                .await
                .context("Failed to store chunk")?;
            content_addresses.push(address.to_hex());
        }

        // Update content index
        {
            let mut index = self.content_store.index.write().await;
            index.by_package.insert(
                package_id,
                chunks.iter()
                    .map(|c| ContentAddress::from_data(&c.data))
                    .collect(),
            );
        }

        // Create and store Merkle tree
        let merkle_tree = MerkleTree::from_chunks(
            &chunks.iter().map(|c| c.data.clone()).collect::<Vec<_>>()
        ).context("Failed to create Merkle tree")?;

        {
            let mut trees = self.content_store.merkle_trees.write().await;
            trees.insert(package_id, merkle_tree);
        }

        // Store metadata
        let metadata = PackageMetadata {
            name: package.spec.metadata.name.clone(),
            version: package.spec.metadata.version.clone(),
            size: package_data.len() as u64,
            chunk_count: chunks.len(),
            chunk_size: 1024 * 1024,
            hash: ContentAddress::from_data(&package_data).to_hex(),
            created_at: chrono::Utc::now(),
        };

        {
            let mut cache = self.metadata_cache.write().await;
            cache.insert(package_id, metadata);
        }

        // Save package file locally
        let package_path = self.get_package_path(&package_id);
        if let Some(parent) = package_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&package_path, &package_data).await
            .context("Failed to save package file")?;

        Ok(content_addresses)
    }

    /// Download a package from peers
    pub async fn download_from_peers(
        &self,
        package_id: &AssetPackageId,
        peers: &[NodeId],
        transport: Arc<StoqTransportLayer>,
    ) -> Result<AssetPackage> {
        if peers.is_empty() {
            return Err(anyhow::anyhow!("No peers available for download"));
        }

        // Get package info from first available peer
        let package_info = self.get_package_info_from_peers(package_id, peers, &transport).await?;

        // Calculate chunks to download
        let chunk_indices: Vec<usize> = (0..package_info.metadata.chunk_count).collect();

        // Download chunks in parallel from multiple peers
        let chunks = self.download_chunks_parallel(
            package_id,
            &chunk_indices,
            peers,
            &transport,
        ).await?;

        // Reassemble package
        let package_data = self.chunker.reassemble(&chunks)
            .context("Failed to reassemble package")?;

        // Deserialize package
        let package: AssetPackage = bincode::deserialize(&package_data)
            .context("Failed to deserialize package")?;

        // Store locally for future seeding
        self.store_package(&package).await?;

        Ok(package)
    }

    /// Get package info from peers
    async fn get_package_info_from_peers(
        &self,
        package_id: &AssetPackageId,
        peers: &[NodeId],
        transport: &Arc<StoqTransportLayer>,
    ) -> Result<PackageInfo> {
        for peer in peers {
            match transport.send_request(
                peer,
                RequestType::GetPackageInfo(*package_id),
            ).await {
                Ok(ResponseData::PackageInfo(info)) => return Ok(info),
                Ok(_) => continue,
                Err(_) => continue,
            }
        }

        Err(anyhow::anyhow!("Failed to get package info from any peer"))
    }

    /// Download chunks in parallel from multiple peers
    async fn download_chunks_parallel(
        &self,
        package_id: &AssetPackageId,
        chunk_indices: &[usize],
        peers: &[NodeId],
        transport: &Arc<StoqTransportLayer>,
    ) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::with_capacity(chunk_indices.len());
        let chunk_count = chunk_indices.len();

        // Create download tasks
        let mut download_futures = Vec::new();

        for (i, &chunk_index) in chunk_indices.iter().enumerate() {
            let peer = &peers[i % peers.len()]; // Round-robin peer selection
            let transport = transport.clone();
            let package_id = *package_id;
            let peer_id = peer.clone();
            let semaphore = self.download_semaphore.clone();

            let fut = async move {
                let _permit = semaphore.acquire().await?;

                match transport.send_request(
                    &peer_id,
                    RequestType::GetChunk {
                        package_id,
                        chunk_index,
                    },
                ).await {
                    Ok(ResponseData::Chunk(chunk_data)) => {
                        Ok::<_, anyhow::Error>((chunk_index, chunk_data))
                    }
                    Ok(_) => Err(anyhow::anyhow!("Invalid response for chunk {}", chunk_index)),
                    Err(e) => Err(e),
                }
            };

            download_futures.push(fut);
        }

        // Execute downloads in parallel
        let mut stream = stream::iter(download_futures)
            .buffer_unordered(10); // Max 10 concurrent downloads

        let mut chunk_map = HashMap::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok((index, chunk_data)) => {
                    chunk_map.insert(index, chunk_data);
                }
                Err(e) => {
                    tracing::warn!("Failed to download chunk: {}", e);
                }
            }
        }

        // Verify we got all chunks
        if chunk_map.len() != chunk_count {
            return Err(anyhow::anyhow!(
                "Failed to download all chunks: got {}/{}",
                chunk_map.len(),
                chunk_count
            ));
        }

        // Sort chunks by index
        for i in 0..chunk_count {
            let chunk_data = chunk_map.remove(&i)
                .ok_or_else(|| anyhow::anyhow!("Missing chunk {}", i))?;

            chunks.push(Chunk {
                index: chunk_data.index,
                data: chunk_data.data,
                hash: ContentAddress::from_hex(&chunk_data.hash)?,
                size: chunk_data.data.len(),
                compressed_size: chunk_data.data.len(),
                compression: CompressionType::Zstd,
            });
        }

        Ok(chunks)
    }

    /// Get package info
    pub async fn get_package_info(&self, package_id: &AssetPackageId) -> Result<PackageInfo> {
        // Check metadata cache
        let metadata = {
            let cache = self.metadata_cache.read().await;
            cache.get(package_id).cloned()
        };

        let metadata = metadata.ok_or_else(|| anyhow::anyhow!("Package {} not found", package_id))?;

        // Get available chunks
        let available_chunks = self.get_available_chunks(package_id).await?;

        // Get Merkle root
        let merkle_root = {
            let trees = self.content_store.merkle_trees.read().await;
            trees.get(package_id)
                .map(|tree| tree.root_hash().to_hex())
                .unwrap_or_default()
        };

        Ok(PackageInfo {
            metadata,
            available_chunks,
            merkle_root,
        })
    }

    /// Get a specific chunk
    pub async fn get_chunk(&self, package_id: &AssetPackageId, chunk_index: usize) -> Result<ChunkData> {
        // Check chunk cache first
        {
            let mut cache = self.chunk_cache.write().await;
            if let Some(data) = cache.get(&(*package_id, chunk_index)) {
                return Ok(ChunkData {
                    index: chunk_index,
                    data: data.clone(),
                    hash: ContentAddress::from_data(&data).to_hex(),
                });
            }
        }

        // Load from content store
        let index = self.content_store.index.read().await;
        let addresses = index.by_package.get(package_id)
            .ok_or_else(|| anyhow::anyhow!("Package {} not found", package_id))?;

        if chunk_index >= addresses.len() {
            return Err(anyhow::anyhow!("Chunk index {} out of bounds", chunk_index));
        }

        let address = &addresses[chunk_index];
        let data = self.content_store.storage
            .get_chunk(address)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chunk not found"))?;

        // Add to cache
        {
            let mut cache = self.chunk_cache.write().await;
            cache.put((*package_id, chunk_index), data.clone());
        }

        Ok(ChunkData {
            index: chunk_index,
            data: data.clone(),
            hash: address.to_hex(),
        })
    }

    /// Get available chunks for a package
    async fn get_available_chunks(&self, package_id: &AssetPackageId) -> Result<Vec<usize>> {
        let index = self.content_store.index.read().await;
        let addresses = index.by_package.get(package_id)
            .ok_or_else(|| anyhow::anyhow!("Package {} not found", package_id))?;

        let mut available = Vec::new();
        for (i, address) in addresses.iter().enumerate() {
            if self.content_store.storage.has_chunk(address).await? {
                available.push(i);
            }
        }

        Ok(available)
    }

    /// Get package file path
    fn get_package_path(&self, package_id: &AssetPackageId) -> PathBuf {
        self.storage_dir
            .join("packages")
            .join(format!("{}.pkg", package_id))
    }

    /// Load package from local storage
    pub async fn load_package(&self, package_id: &AssetPackageId) -> Result<Option<AssetPackage>> {
        let package_path = self.get_package_path(package_id);

        if !package_path.exists() {
            return Ok(None);
        }

        let package_data = tokio::fs::read(&package_path).await?;
        let package: AssetPackage = bincode::deserialize(&package_data)?;

        Ok(Some(package))
    }

    /// Delete package from local storage
    pub async fn delete_package(&self, package_id: &AssetPackageId) -> Result<()> {
        // Remove from content store
        let addresses = {
            let index = self.content_store.index.read().await;
            index.by_package.get(package_id).cloned()
        };

        if let Some(addresses) = addresses {
            for address in addresses {
                self.content_store.storage.delete_chunk(&address).await?;
            }
        }

        // Remove from index
        {
            let mut index = self.content_store.index.write().await;
            index.by_package.remove(package_id);

            // Also remove from by_content index
            index.by_content.retain(|_, packages| {
                packages.retain(|id| id != package_id);
                !packages.is_empty()
            });
        }

        // Remove from Merkle trees
        {
            let mut trees = self.content_store.merkle_trees.write().await;
            trees.remove(package_id);
        }

        // Remove from metadata cache
        {
            let mut cache = self.metadata_cache.write().await;
            cache.remove(package_id);
        }

        // Remove package file
        let package_path = self.get_package_path(package_id);
        if package_path.exists() {
            tokio::fs::remove_file(&package_path).await?;
        }

        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<super::StorageStats> {
        self.content_store.storage.get_stats().await
    }
}

impl ChunkCache {
    fn new(max_size: usize) -> Self {
        Self {
            max_size,
            current_size: 0,
            chunks: HashMap::new(),
            access_times: HashMap::new(),
        }
    }

    fn get(&mut self, key: &(AssetPackageId, usize)) -> Option<&Vec<u8>> {
        if let Some(data) = self.chunks.get(key) {
            self.access_times.insert(*key, std::time::Instant::now());
            Some(data)
        } else {
            None
        }
    }

    fn put(&mut self, key: (AssetPackageId, usize), data: Vec<u8>) {
        let data_size = data.len();

        // Evict old entries if needed
        while self.current_size + data_size > self.max_size && !self.chunks.is_empty() {
            self.evict_lru();
        }

        if self.current_size + data_size <= self.max_size {
            self.chunks.insert(key, data);
            self.access_times.insert(key, std::time::Instant::now());
            self.current_size += data_size;
        }
    }

    fn evict_lru(&mut self) {
        if let Some((oldest_key, _)) = self.access_times
            .iter()
            .min_by_key(|(_, time)| *time)
            .map(|(k, v)| (*k, *v))
        {
            if let Some(data) = self.chunks.remove(&oldest_key) {
                self.current_size -= data.len();
                self.access_times.remove(&oldest_key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_path_buf();

        // Create mock content store
        let storage = super::super::FileBasedStorage::new(storage_dir.clone()).unwrap();
        let content_store = Arc::new(ContentStore {
            storage: Arc::new(storage),
            index: Arc::new(RwLock::new(super::super::ContentIndex::default())),
            merkle_trees: Arc::new(RwLock::new(HashMap::new())),
        });

        let manager = PackageManager::new(content_store, storage_dir).await.unwrap();

        // Test basic operations
        assert!(manager.get_storage_stats().await.is_ok());
    }
}
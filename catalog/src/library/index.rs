//! Fast Package Discovery Index
//!
//! Provides high-performance search and discovery capabilities for the asset library.

use super::types::*;
use super::SearchQuery;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet, BTreeMap};

/// Search index for fast package discovery
pub struct LibraryIndex {
    /// Name index (package name -> IDs)
    name_index: Arc<RwLock<HashMap<String, HashSet<Arc<str>>>>>,
    /// Tag index (tag -> package IDs)
    tag_index: Arc<RwLock<HashMap<Arc<str>, HashSet<Arc<str>>>>>,
    /// Type index (asset type -> package IDs)
    type_index: Arc<RwLock<HashMap<AssetType, HashSet<Arc<str>>>>>,
    /// Author index (author -> package IDs)
    author_index: Arc<RwLock<HashMap<Arc<str>, HashSet<Arc<str>>>>>,
    /// Keyword index (keyword -> package IDs)
    keyword_index: Arc<RwLock<HashMap<Arc<str>, HashSet<Arc<str>>>>>,
    /// Version index (name -> version -> ID)
    version_index: Arc<RwLock<HashMap<Arc<str>, BTreeMap<Arc<str>, Arc<str>>>>>,
    /// Full-text search index (simplified)
    text_index: Arc<RwLock<TextSearchIndex>>,
    /// All indexed package IDs
    all_packages: Arc<RwLock<HashSet<Arc<str>>>>,
}

/// Simple text search index
struct TextSearchIndex {
    /// Word -> package IDs mapping
    word_to_packages: HashMap<String, HashSet<Arc<str>>>,
    /// Bigram -> package IDs mapping for fuzzy search
    bigram_to_packages: HashMap<String, HashSet<Arc<str>>>,
}

impl TextSearchIndex {
    fn new() -> Self {
        Self {
            word_to_packages: HashMap::new(),
            bigram_to_packages: HashMap::new(),
        }
    }

    fn index_text(&mut self, text: &str, package_id: Arc<str>) {
        // Index words
        for word in tokenize(text) {
            self.word_to_packages
                .entry(word.to_lowercase())
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&package_id));
        }

        // Index bigrams for fuzzy matching
        for bigram in generate_bigrams(text) {
            self.bigram_to_packages
                .entry(bigram)
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&package_id));
        }
    }

    fn search(&self, query: &str) -> HashSet<Arc<str>> {
        let mut results = HashSet::new();

        // Search for exact word matches
        for word in tokenize(query) {
            if let Some(packages) = self.word_to_packages.get(&word.to_lowercase()) {
                results.extend(packages.iter().cloned());
            }
        }

        // If no exact matches, try fuzzy search with bigrams
        if results.is_empty() {
            let query_bigrams = generate_bigrams(query);
            let mut scored_results: HashMap<Arc<str>, usize> = HashMap::new();

            for bigram in query_bigrams {
                if let Some(packages) = self.bigram_to_packages.get(&bigram) {
                    for package_id in packages {
                        *scored_results.entry(Arc::clone(package_id)).or_insert(0) += 1;
                    }
                }
            }

            // Take packages with highest bigram matches
            let mut sorted_results: Vec<_> = scored_results.into_iter().collect();
            sorted_results.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

            results.extend(sorted_results.into_iter().take(20).map(|(id, _)| id));
        }

        results
    }

    fn remove(&mut self, package_id: &Arc<str>) {
        // Remove from word index
        self.word_to_packages.retain(|_, packages| {
            packages.remove(package_id);
            !packages.is_empty()
        });

        // Remove from bigram index
        self.bigram_to_packages.retain(|_, packages| {
            packages.remove(package_id);
            !packages.is_empty()
        });
    }
}

impl LibraryIndex {
    /// Create a new search index
    pub fn new() -> Self {
        Self {
            name_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
            type_index: Arc::new(RwLock::new(HashMap::new())),
            author_index: Arc::new(RwLock::new(HashMap::new())),
            keyword_index: Arc::new(RwLock::new(HashMap::new())),
            version_index: Arc::new(RwLock::new(HashMap::new())),
            text_index: Arc::new(RwLock::new(TextSearchIndex::new())),
            all_packages: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Initialize the index
    pub async fn initialize(&self) -> Result<()> {
        // Clear all indices
        self.clear().await?;
        Ok(())
    }

    /// Index a package for search
    pub async fn index_package(&self, package: &LibraryAssetPackage) -> Result<()> {
        let package_id = Arc::clone(&package.id);

        // Add to all packages set
        {
            let mut all = self.all_packages.write().await;
            all.insert(Arc::clone(&package_id));
        }

        // Index by name
        {
            let mut name_index = self.name_index.write().await;
            let name_key = package.metadata.name.to_lowercase();
            name_index
                .entry(name_key)
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&package_id));
        }

        // Index by tags
        {
            let mut tag_index = self.tag_index.write().await;
            for tag in package.metadata.tags.iter() {
                tag_index
                    .entry(Arc::clone(tag))
                    .or_insert_with(HashSet::new)
                    .insert(Arc::clone(&package_id));
            }
        }

        // Index by type
        {
            let mut type_index = self.type_index.write().await;
            type_index
                .entry(package.spec.asset_type)
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&package_id));
        }

        // Index by author
        if let Some(author) = &package.metadata.author {
            let mut author_index = self.author_index.write().await;
            author_index
                .entry(Arc::clone(author))
                .or_insert_with(HashSet::new)
                .insert(Arc::clone(&package_id));
        }

        // Index by keywords
        {
            let mut keyword_index = self.keyword_index.write().await;
            for keyword in package.metadata.keywords.iter() {
                keyword_index
                    .entry(Arc::clone(keyword))
                    .or_insert_with(HashSet::new)
                    .insert(Arc::clone(&package_id));
            }
        }

        // Index by version
        {
            let mut version_index = self.version_index.write().await;
            version_index
                .entry(Arc::clone(&package.metadata.name))
                .or_insert_with(BTreeMap::new)
                .insert(Arc::clone(&package.metadata.version), Arc::clone(&package_id));
        }

        // Index for text search
        {
            let mut text_index = self.text_index.write().await;

            // Index name
            text_index.index_text(&package.metadata.name, Arc::clone(&package_id));

            // Index description
            if let Some(desc) = &package.metadata.description {
                text_index.index_text(desc, Arc::clone(&package_id));
            }

            // Index keywords
            for keyword in package.metadata.keywords.iter() {
                text_index.index_text(keyword, Arc::clone(&package_id));
            }
        }

        Ok(())
    }

    /// Remove a package from the index
    pub async fn remove_package(&self, package_id: &Arc<str>) -> Result<()> {
        // Remove from all packages
        {
            let mut all = self.all_packages.write().await;
            all.remove(package_id);
        }

        // Remove from name index
        {
            let mut name_index = self.name_index.write().await;
            name_index.retain(|_, packages| {
                packages.remove(package_id);
                !packages.is_empty()
            });
        }

        // Remove from tag index
        {
            let mut tag_index = self.tag_index.write().await;
            tag_index.retain(|_, packages| {
                packages.remove(package_id);
                !packages.is_empty()
            });
        }

        // Remove from type index
        {
            let mut type_index = self.type_index.write().await;
            type_index.retain(|_, packages| {
                packages.remove(package_id);
                !packages.is_empty()
            });
        }

        // Remove from author index
        {
            let mut author_index = self.author_index.write().await;
            author_index.retain(|_, packages| {
                packages.remove(package_id);
                !packages.is_empty()
            });
        }

        // Remove from keyword index
        {
            let mut keyword_index = self.keyword_index.write().await;
            keyword_index.retain(|_, packages| {
                packages.remove(package_id);
                !packages.is_empty()
            });
        }

        // Remove from version index
        {
            let mut version_index = self.version_index.write().await;
            version_index.retain(|_, versions| {
                versions.retain(|_, id| id != package_id);
                !versions.is_empty()
            });
        }

        // Remove from text index
        {
            let mut text_index = self.text_index.write().await;
            text_index.remove(package_id);
        }

        Ok(())
    }

    /// Search for packages based on query
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Arc<str>>> {
        let mut result_sets = Vec::new();

        // Text search
        if !query.query.is_empty() {
            let text_index = self.text_index.read().await;
            let text_results = text_index.search(&query.query);
            if !text_results.is_empty() {
                result_sets.push(text_results);
            }
        }

        // Tag filter
        if !query.tags.is_empty() {
            let tag_index = self.tag_index.read().await;
            let mut tag_results = HashSet::new();

            for tag in &query.tags {
                if let Some(packages) = tag_index.get(tag.as_str()) {
                    if tag_results.is_empty() {
                        tag_results = packages.clone();
                    } else {
                        // Intersection for multiple tags
                        tag_results = tag_results.intersection(packages).cloned().collect();
                    }
                }
            }

            if !tag_results.is_empty() {
                result_sets.push(tag_results);
            }
        }

        // Type filter
        if let Some(asset_type) = &query.asset_type {
            if let Some(typed) = AssetType::from_str(asset_type) {
                let type_index = self.type_index.read().await;
                if let Some(packages) = type_index.get(&typed) {
                    result_sets.push(packages.clone());
                }
            }
        }

        // Author filter
        if let Some(author) = &query.author {
            let author_index = self.author_index.read().await;
            if let Some(packages) = author_index.get(author.as_str()) {
                result_sets.push(packages.clone());
            }
        }

        // Combine results (intersection if multiple filters)
        let final_results = if result_sets.is_empty() {
            // No filters, return all packages
            let all = self.all_packages.read().await;
            all.iter().cloned().collect()
        } else if result_sets.len() == 1 {
            result_sets.into_iter().next().unwrap().into_iter().collect()
        } else {
            // Intersection of all result sets
            let mut combined = result_sets[0].clone();
            for set in result_sets.iter().skip(1) {
                combined = combined.intersection(set).cloned().collect();
            }
            combined.into_iter().collect()
        };

        // Apply pagination
        let start = query.offset;
        let end = (query.offset + query.limit).min(final_results.len());

        Ok(final_results[start..end].to_vec())
    }

    /// Find packages by name (exact or prefix match)
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<Arc<str>>> {
        let name_index = self.name_index.read().await;
        let name_lower = name.to_lowercase();

        let mut results = Vec::new();

        // Exact match
        if let Some(packages) = name_index.get(&name_lower) {
            results.extend(packages.iter().cloned());
        }

        // Prefix match
        for (indexed_name, packages) in name_index.iter() {
            if indexed_name.starts_with(&name_lower) && indexed_name != &name_lower {
                results.extend(packages.iter().cloned());
            }
        }

        Ok(results)
    }

    /// Get latest version of a package
    pub async fn get_latest_version(&self, name: &str) -> Result<Option<Arc<str>>> {
        let version_index = self.version_index.read().await;

        if let Some(versions) = version_index.get(name) {
            // BTreeMap keeps versions sorted, get the last (highest) one
            if let Some((_, package_id)) = versions.iter().last() {
                return Ok(Some(Arc::clone(package_id)));
            }
        }

        Ok(None)
    }

    /// Clear all indices
    pub async fn clear(&self) -> Result<()> {
        let mut name_index = self.name_index.write().await;
        name_index.clear();
        drop(name_index);

        let mut tag_index = self.tag_index.write().await;
        tag_index.clear();
        drop(tag_index);

        let mut type_index = self.type_index.write().await;
        type_index.clear();
        drop(type_index);

        let mut author_index = self.author_index.write().await;
        author_index.clear();
        drop(author_index);

        let mut keyword_index = self.keyword_index.write().await;
        keyword_index.clear();
        drop(keyword_index);

        let mut version_index = self.version_index.write().await;
        version_index.clear();
        drop(version_index);

        let mut text_index = self.text_index.write().await;
        *text_index = TextSearchIndex::new();
        drop(text_index);

        let mut all = self.all_packages.write().await;
        all.clear();

        Ok(())
    }

    /// Rebuild the index (for optimization)
    pub async fn rebuild(&self) -> Result<()> {
        // In a real implementation, this would:
        // 1. Create new indices
        // 2. Re-index all packages
        // 3. Swap indices atomically
        // For now, just a placeholder
        Ok(())
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> Result<IndexStats> {
        let all = self.all_packages.read().await;
        let name_index = self.name_index.read().await;
        let tag_index = self.tag_index.read().await;
        let type_index = self.type_index.read().await;
        let author_index = self.author_index.read().await;
        let keyword_index = self.keyword_index.read().await;

        Ok(IndexStats {
            total_packages: all.len(),
            unique_names: name_index.len(),
            unique_tags: tag_index.len(),
            unique_types: type_index.len(),
            unique_authors: author_index.len(),
            unique_keywords: keyword_index.len(),
        })
    }
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_packages: usize,
    pub unique_names: usize,
    pub unique_tags: usize,
    pub unique_types: usize,
    pub unique_authors: usize,
    pub unique_keywords: usize,
}

/// Tokenize text into words
fn tokenize(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .flat_map(|word| word.split(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty() && word.len() > 1)
        .collect()
}

/// Generate bigrams for fuzzy matching
fn generate_bigrams(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let chars: Vec<char> = text_lower.chars().collect();
    let mut bigrams = Vec::new();

    for window in chars.windows(2) {
        bigrams.push(window.iter().collect());
    }

    bigrams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let text = "hello-world test_package 123";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"hello"));
        assert!(tokens.contains(&"world"));
        assert!(tokens.contains(&"test"));
        assert!(tokens.contains(&"package"));
        assert!(tokens.contains(&"123"));
    }

    #[test]
    fn test_bigrams() {
        let text = "test";
        let bigrams = generate_bigrams(text);
        assert_eq!(bigrams, vec!["te", "es", "st"]);
    }

    #[tokio::test]
    async fn test_index_and_search() {
        let index = LibraryIndex::new();

        let package = create_test_package();
        index.index_package(&package).await.unwrap();

        // Search by text
        let query = SearchQuery {
            query: "test".to_string(),
            tags: vec![],
            asset_type: None,
            author: None,
            limit: 10,
            offset: 0,
        };

        let results = index.search(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref(), "test-pkg");

        // Search by tag
        let query = SearchQuery {
            query: String::new(),
            tags: vec!["testing".to_string()],
            asset_type: None,
            author: None,
            limit: 10,
            offset: 0,
        };

        let results = index.search(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_from_index() {
        let index = LibraryIndex::new();

        let package = create_test_package();
        let package_id = Arc::clone(&package.id);

        index.index_package(&package).await.unwrap();

        // Verify it's indexed
        let stats = index.get_stats().await.unwrap();
        assert_eq!(stats.total_packages, 1);

        // Remove it
        index.remove_package(&package_id).await.unwrap();

        // Verify it's gone
        let stats = index.get_stats().await.unwrap();
        assert_eq!(stats.total_packages, 0);
    }

    fn create_test_package() -> LibraryAssetPackage {
        LibraryAssetPackage {
            id: Arc::from("test-pkg"),
            metadata: PackageMetadata {
                name: Arc::from("test-package"),
                version: Arc::from("1.0.0"),
                description: Some(Arc::from("A test package")),
                author: Some(Arc::from("test-author")),
                license: Some(Arc::from("MIT")),
                tags: Arc::new([Arc::from("testing")]),
                keywords: Arc::new([Arc::from("test"), Arc::from("example")]),
                created: 0,
                modified: 0,
            },
            spec: PackageSpec {
                asset_type: AssetType::JuliaProgram,
                resources: ResourceRequirements::default(),
                security: SecurityConfig {
                    consensus_required: false,
                    sandbox_level: SandboxLevel::Standard,
                    network_access: false,
                    filesystem_access: FilesystemAccess::ReadOnly,
                    permissions: Arc::new([]),
                },
                execution: ExecutionConfig {
                    strategy: ExecutionStrategy::NearestNode,
                    min_consensus: 1,
                    max_concurrent: None,
                    priority: ExecutionPriority::Normal,
                    retry_policy: RetryPolicy::default(),
                },
                dependencies: Arc::new([]),
                environment: Arc::new(HashMap::new()),
            },
            content_refs: ContentReferences {
                main_ref: ContentRef {
                    path: Arc::from("main.jl"),
                    hash: Arc::from("hash"),
                    size: 100,
                    content_type: ContentType::Source,
                },
                file_refs: Arc::new([]),
                binary_refs: Arc::new([]),
                total_size: 100,
            },
            validation: None,
            hash: Arc::from("hash"),
        }
    }
}
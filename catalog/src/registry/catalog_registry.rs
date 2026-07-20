// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog Registry Service
//!
//! Provides indexing and discovery for asset type definitions.
//! The registry itself is stored as a BlockMatrix Asset.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use blockmatrix::assets::core::{AssetCategory, AssetData, BaseSystemType, NetworkScope};
use blockmatrix::assets::{AssetRegistration, StateProof};
use hypermesh_lib::{ContentHash, PrivacyMode};

use super::asset_type::AssetTypeDefinition;
use super::schema_scanner::scan_schema_for_typedef_refs;

/// Content-addressed type registration record.
///
/// Created when a type definition is registered. The `type_hash` is the
/// BLAKE3 hash of the canonical JSON-serialized schema, providing a
/// stable, content-addressed identifier for deduplication and lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRegistration {
    /// Human-readable type name (e.g. "Message", "Document")
    pub type_name: String,

    /// BLAKE3 hash of the canonical schema JSON (hex-encoded)
    pub type_hash: String,

    /// The JSON schema defining the type structure
    pub schema: serde_json::Value,

    /// Semantic version at registration time
    pub version: String,

    /// Unix timestamp (seconds) when the type was registered
    pub registered_at: i64,
}

/// Catalog Registry - provides indexing/discovery for asset types
///
/// The registry itself is stored as a BlockMatrix Asset
#[derive(Clone)]
pub struct CatalogRegistry {
    /// Registry ID (this registry is an Asset)
    registry_id: AssetRegistration,

    /// Index of type names → asset IDs
    index: Arc<RwLock<HashMap<String, AssetRegistration>>>,

    /// Privacy level configuration
    privacy: PrivacyMode,

    /// Full type definitions (for scoring metadata)
    type_definitions: Arc<RwLock<HashMap<String, AssetTypeDefinition>>>,

    /// Content-addressed type registrations (type_hash → TypeRegistration)
    type_registrations: Arc<RwLock<HashMap<String, TypeRegistration>>>,

    /// Trust policy
    trust_policy: TrustPolicy,

    /// Registry configuration
    config: RegistryConfig,
}

/// Trust policy for registry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicy {
    /// Require Proof of State for registration
    pub require_state_proof: bool,

    /// Allowed publishers (empty = allow all)
    pub allowed_publishers: Vec<String>,

    /// Require certificate validation
    pub require_certificate: bool,
}

impl Default for TrustPolicy {
    fn default() -> Self {
        Self {
            require_state_proof: true,
            allowed_publishers: Vec::new(),
            require_certificate: true,
        }
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry name
    pub name: String,

    /// Registry description
    pub description: Option<String>,

    /// Maximum entries
    pub max_entries: usize,

    /// Enable versioning
    pub enable_versioning: bool,

    /// Enable dependency resolution
    pub enable_dependency_resolution: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: None,
            max_entries: 100_000,
            enable_versioning: true,
            enable_dependency_resolution: true,
        }
    }
}

impl CatalogRegistry {
    /// Create a new registry
    pub fn new(privacy: PrivacyMode, trust_policy: TrustPolicy, config: RegistryConfig) -> Self {
        // Create registry AssetRegistration from registry configuration
        let asset_data = AssetData {
            config: format!("registry_{privacy:?}").as_bytes().to_vec(),
            definition: b"catalog_registry".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let registry_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        );

        Self {
            registry_id,
            index: Arc::new(RwLock::new(HashMap::new())),
            type_definitions: Arc::new(RwLock::new(HashMap::new())),
            type_registrations: Arc::new(RwLock::new(HashMap::new())),
            privacy,
            trust_policy,
            config,
        }
    }

    /// Create a registry pre-populated with built-in HyperMesh types.
    ///
    /// Built-in types (Message, Invitation, Document) are registered with
    /// a permissive trust policy so they do not require Proof of State.
    pub async fn with_builtin_types(
        privacy: PrivacyMode,
        trust_policy: TrustPolicy,
        config: RegistryConfig,
    ) -> Self {
        // Use a relaxed trust policy for built-in registration
        let builtin_policy = TrustPolicy {
            require_state_proof: false,
            allowed_publishers: Vec::new(),
            require_certificate: false,
        };
        let registry = Self::new(privacy.clone(), builtin_policy, config.clone());

        let builtins: Vec<(&str, serde_json::Value)> = vec![
            (
                "Message",
                serde_json::json!({
                    "type": "object",
                    "required": ["sender_node_id", "recipient_node_id", "encrypted_body"],
                    "properties": {
                        "sender_node_id": { "type": "string" },
                        "recipient_node_id": { "type": "string" },
                        "encrypted_body": { "type": "string", "format": "hex" },
                        "content_type": { "type": "string", "default": "text/plain" },
                        "reply_to": { "type": "string" }
                    }
                }),
            ),
            (
                "Invitation",
                serde_json::json!({
                    "type": "object",
                    "required": ["sender_node_id", "recipient_node_id", "asset_id", "encrypted_key"],
                    "properties": {
                        "sender_node_id": { "type": "string" },
                        "recipient_node_id": { "type": "string" },
                        "asset_id": { "type": "string" },
                        "encrypted_key": { "type": "string", "format": "hex" }
                    }
                }),
            ),
            (
                "Document",
                serde_json::json!({
                    "type": "object",
                    "required": ["name", "content_hash"],
                    "properties": {
                        "name": { "type": "string" },
                        "content_hash": { "type": "string" },
                        "content_type": { "type": "string" },
                        "size": { "type": "integer" }
                    }
                }),
            ),
        ];

        for (name, schema) in builtins {
            let state_proof = Self::builtin_state_proof();
            let type_def = AssetTypeDefinition::new(name.to_string(), schema, state_proof);
            // Ignore errors for built-in types (should never fail)
            if let Err(e) = registry.register_type(type_def).await {
                tracing::warn!("Failed to register built-in type '{}': {}", name, e);
            }
        }

        // Now swap in the caller's real trust policy
        // We do this by reconstructing with the correct policy. Since the
        // type data lives in Arcs, we can just swap the policy field.
        Self {
            registry_id: registry.registry_id,
            index: registry.index,
            type_definitions: registry.type_definitions,
            type_registrations: registry.type_registrations,
            privacy,
            trust_policy,
            config,
        }
    }

    /// Construct a minimal state proof for built-in type registration.
    ///
    /// This is public so STOQ API handlers can create type definitions
    /// without requiring callers to construct full proofs during alpha.
    pub fn builtin_state_proof() -> StateProof {
        use blockmatrix::proof_of_state::proof_of_state_integration::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        use std::time::Duration;

        // Authorization (WHO): a bound "builtin" identity, no magnitude.
        let stake = StakeProof::new("builtin".to_string(), "builtin".to_string());
        let space = SpaceProof::new("builtin".to_string(), "/builtin".to_string(), 0);
        // WHAT: BLAKE3 hash of the built-in registration work.
        let work = WorkProof::from_work("builtin".to_string(), "builtin".to_string(), b"builtin");
        let time = TimeProof::new(Duration::from_secs(0));
        StateProof::new(stake, time, space, work)
    }

    /// Register a new asset type definition
    pub async fn register_type(&self, type_def: AssetTypeDefinition) -> Result<AssetRegistration> {
        // Validate Proof of State if required
        if self.trust_policy.require_state_proof {
            self.validate_state_proof(&type_def.state_proof)?;
        }

        // Check if type already exists
        let mut index = self.index.write().await;
        if index.contains_key(&type_def.type_name) {
            return Err(anyhow::anyhow!(
                "Type '{}' already registered",
                type_def.type_name
            ));
        }

        // Check registry capacity
        if index.len() >= self.config.max_entries {
            return Err(anyhow::anyhow!("Registry capacity exceeded"));
        }

        // Store in index
        let asset_id = type_def.asset_id.clone();
        let type_name = type_def.type_name.clone();
        index.insert(type_name.clone(), asset_id.clone());

        // Compute content-addressed type hash (BLAKE3 of canonical schema JSON)
        let schema_json = serde_json::to_string(&type_def.schema)
            .map_err(|e| anyhow::anyhow!("failed to serialize schema: {e}"))?;
        let type_hash = hex::encode(blake3::hash(schema_json.as_bytes()).as_bytes());

        let registration = TypeRegistration {
            type_name: type_name.clone(),
            type_hash: type_hash.clone(),
            schema: type_def.schema.clone(),
            version: type_def.metadata.version.clone(),
            registered_at: chrono::Utc::now().timestamp(),
        };

        // Store content-addressed registration
        let mut regs = self.type_registrations.write().await;
        regs.insert(type_hash, registration);

        // Store full definition for scoring
        let mut defs = self.type_definitions.write().await;
        defs.insert(type_name.clone(), type_def);

        tracing::info!("Registered asset type: {}", type_name);
        Ok(asset_id)
    }

    /// Find asset type by name
    pub async fn find_type(&self, name: &str) -> Result<AssetRegistration> {
        let index = self.index.read().await;
        index
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Type '{name}' not found"))
    }

    /// List all registered types
    pub async fn list_types(&self) -> Result<Vec<String>> {
        let index = self.index.read().await;
        let types: Vec<String> = index.keys().cloned().collect();
        Ok(types)
    }

    /// Search types by query with multi-factor relevance scoring
    pub async fn search_types(&self, query: &SearchQuery) -> Result<SearchResults> {
        let index = self.index.read().await;
        let defs = self.type_definitions.read().await;

        let mut matching_types = Vec::new();
        let query_lower = query.query.to_lowercase();

        for (type_name, asset_id) in index.iter() {
            let name_lower = type_name.to_lowercase();

            // Name matching score (0.0 - 0.5)
            let name_score = if query.query.is_empty() {
                0.3 // Neutral for browse-all queries
            } else if name_lower == query_lower {
                0.5 // Exact match
            } else if name_lower.starts_with(&query_lower) {
                0.4 // Prefix match
            } else if name_lower.contains(&query_lower) {
                0.25 // Contains
            } else {
                continue; // No match at all
            };

            // Metadata-based scoring (0.0 - 0.5)
            let metadata_score = if let Some(def) = defs.get(type_name) {
                let mut ms = 0.0f64;
                // Tag match bonus
                if !query.tags.is_empty() {
                    let tag_matches = def
                        .metadata
                        .tags
                        .iter()
                        .filter(|t| query.tags.iter().any(|qt| qt.eq_ignore_ascii_case(t)))
                        .count();
                    ms += (tag_matches as f64 / query.tags.len().max(1) as f64) * 0.2;
                }
                // Author match bonus
                if let Some(ref qa) = query.author {
                    if def.metadata.author.as_deref() == Some(qa.as_str()) {
                        ms += 0.1;
                    }
                }
                // Recency bonus (newer = higher, max 0.1)
                let age_days = (chrono::Utc::now() - def.metadata.created_at)
                    .num_days()
                    .max(0) as f64;
                ms += 0.1 * (1.0 / (1.0 + age_days / 30.0));
                // Version count bonus (more versions = more established, max 0.1)
                ms += 0.1 * (def.metadata.version_count.min(10) as f64 / 10.0);
                ms
            } else {
                0.0
            };

            let total_score = (name_score + metadata_score).min(1.0);

            matching_types.push(SearchResult {
                type_name: type_name.clone(),
                asset_id: asset_id.clone(),
                score: total_score,
                publisher_authenticated: None,
            });
        }

        // Sort by requested criteria
        match query.sort_by {
            SortCriteria::Relevance => {
                matching_types.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortCriteria::Name => {
                matching_types.sort_by(|a, b| a.type_name.cmp(&b.type_name));
            }
            _ => {
                // Rating, Downloads, Updated, Published - sort by score as fallback
                matching_types.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Apply pagination
        let total_matching = matching_types.len();
        if query.offset > 0 {
            matching_types = matching_types.into_iter().skip(query.offset).collect();
        }
        matching_types.truncate(query.limit);

        Ok(SearchResults {
            results: matching_types,
            total_count: total_matching,
            query: query.clone(),
        })
    }

    /// Resolve the full dependency graph for a typedef, by content hash.
    ///
    /// Walks declared dependencies AND scanner-discovered embedded refs in BFS
    /// order. Cycles are handled via a visited set — every typedef appears at
    /// most once regardless of how many paths reach it.
    ///
    /// Returns a [`DependencyGraph`] with:
    /// - `direct`: typedefs the root immediately depends on.
    /// - `transitive`: typedefs reached via direct deps (excludes root + direct).
    /// - `missing`: declared/embedded hashes that have no registered typedef.
    ///
    /// If the root itself is not registered, returns a graph whose `missing`
    /// contains only the root hash. No error is raised — missing is honest
    /// state, not a failure.
    pub async fn resolve_dependencies(
        &self,
        root: &ContentHash,
    ) -> Result<DependencyGraph> {
        if !self.config.enable_dependency_resolution {
            return Ok(DependencyGraph::default());
        }

        // 1. Look up the root typedef. If absent → return with missing: [root].
        let root_hex = hex::encode(root.as_bytes());
        let root_reg = match self.lookup_type_by_hash(&root_hex).await {
            Some(r) => r,
            None => {
                return Ok(DependencyGraph {
                    direct: Vec::new(),
                    transitive: Vec::new(),
                    missing: vec![*root],
                });
            }
        };

        // 2. Compute direct dependencies = declared ∪ embedded.
        let direct_hashes = self
            .deps_for(&root_reg, Some(root))
            .await;

        // 3. BFS. Track visited by ContentHash so cycles terminate.
        let mut direct: Vec<DependencyNode> = Vec::new();
        let mut transitive: Vec<DependencyNode> = Vec::new();
        let mut missing: Vec<ContentHash> = Vec::new();
        let mut visited: HashSet<ContentHash> = HashSet::new();
        visited.insert(*root);

        // Mark all direct hashes as visited up front so a direct dep does not
        // re-appear in transitive when reached via a sibling.
        let mut direct_set: HashSet<ContentHash> = HashSet::new();
        for h in &direct_hashes {
            direct_set.insert(*h);
            visited.insert(*h);
        }

        // Resolve direct nodes.
        let mut queue: std::collections::VecDeque<(ContentHash, usize)> =
            std::collections::VecDeque::new();
        for h in &direct_hashes {
            let hex_h = hex::encode(h.as_bytes());
            match self.lookup_type_by_hash(&hex_h).await {
                Some(reg) => {
                    direct.push(DependencyNode {
                        type_hash: *h,
                        name: reg.type_name.clone(),
                        version: reg.version.clone(),
                        depth: 1,
                    });
                    queue.push_back((*h, 1));
                }
                None => missing.push(*h),
            }
        }

        // BFS transitive layers.
        while let Some((parent_hash, depth)) = queue.pop_front() {
            let parent_hex = hex::encode(parent_hash.as_bytes());
            let parent_reg = match self.lookup_type_by_hash(&parent_hex).await {
                Some(r) => r,
                None => continue, // already counted as missing above
            };
            let child_hashes = self.deps_for(&parent_reg, Some(&parent_hash)).await;
            for ch in child_hashes {
                if !visited.insert(ch) {
                    continue; // already seen — cycle or shared dep
                }
                let ch_hex = hex::encode(ch.as_bytes());
                match self.lookup_type_by_hash(&ch_hex).await {
                    Some(child_reg) => {
                        let node = DependencyNode {
                            type_hash: ch,
                            name: child_reg.type_name.clone(),
                            version: child_reg.version.clone(),
                            depth: depth + 1,
                        };
                        // Direct set is already populated; everything else
                        // discovered here is transitive.
                        if direct_set.contains(&ch) {
                            // Should not happen because direct_set entries
                            // are pre-inserted into visited, but keep the
                            // branch for defensive symmetry.
                            continue;
                        }
                        transitive.push(node);
                        queue.push_back((ch, depth + 1));
                    }
                    None => missing.push(ch),
                }
            }
        }

        Ok(DependencyGraph {
            direct,
            transitive,
            missing,
        })
    }

    /// Compute the union of declared + scanner-discovered dependencies for a
    /// registered typedef. `self_hash` is excluded from scanner results so a
    /// schema that mentions its own hash does not self-depend.
    async fn deps_for(
        &self,
        reg: &TypeRegistration,
        self_hash: Option<&ContentHash>,
    ) -> Vec<ContentHash> {
        // Declared dependencies live on the full `AssetTypeDefinition`, not
        // on `TypeRegistration` (which is the wire/cache record).
        let mut out: Vec<ContentHash> = Vec::new();
        let mut seen: HashSet<ContentHash> = HashSet::new();

        if let Some(def) = self.get_type_definition(&reg.type_name).await {
            for dep in &def.dependencies {
                if seen.insert(*dep) {
                    out.push(*dep);
                }
            }
        }

        // Scan the registered schema for embedded BLAKE3 refs.
        let schema_bytes = match serde_json::to_vec(&reg.schema) {
            Ok(v) => v,
            Err(_) => return out,
        };
        let embedded =
            scan_schema_for_typedef_refs(&schema_bytes, self, self_hash).await;
        for h in embedded {
            if seen.insert(h) {
                out.push(h);
            }
        }
        out
    }

    /// Compatibility shim: resolve dependencies by typedef NAME and return the
    /// direct-dep [`AssetRegistration`]s. Used by the library resolver during
    /// the migration to content-addressed deps.
    ///
    /// Prefer [`Self::resolve_dependencies`] (by hash) for new code.
    pub async fn resolve_dependencies_by_name(
        &self,
        type_name: &str,
    ) -> Result<Vec<AssetRegistration>> {
        if !self.config.enable_dependency_resolution {
            return Ok(Vec::new());
        }

        let reg = match self.lookup_type(type_name).await {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };
        let mut root_bytes = [0u8; 32];
        match hex::decode(&reg.type_hash) {
            Ok(bs) if bs.len() == 32 => root_bytes.copy_from_slice(&bs),
            _ => return Ok(Vec::new()),
        }
        let root = ContentHash::from_bytes(root_bytes);

        let graph = self.resolve_dependencies(&root).await?;
        let index = self.index.read().await;
        let regs: Vec<AssetRegistration> = graph
            .direct
            .iter()
            .filter_map(|node| index.get(&node.name).cloned())
            .collect();
        Ok(regs)
    }

    /// Get registry statistics
    pub async fn get_statistics(&self) -> RegistryStatistics {
        let index = self.index.read().await;
        RegistryStatistics {
            total_types: index.len(),
            registry_id: self.registry_id.to_string(),
            privacy_level: format!("{:?}", self.privacy),
        }
    }

    /// Validate Proof of State
    fn validate_state_proof(&self, proof: &StateProof) -> Result<()> {
        // Basic validation
        if !proof.validate() {
            return Err(anyhow::anyhow!("Proof of State validation failed"));
        }

        // CANONICAL MODEL: PoStake is authorization (WHO), never a magnitude.
        // Registration requires a bound authorization identity, not a stake
        // amount.
        if proof.stake_proof.stake_holder_id.is_empty() {
            return Err(anyhow::anyhow!(
                "Registration requires a bound authorization identity (WHO)"
            ));
        }

        Ok(())
    }

    /// Look up a type registration by name.
    ///
    /// Scans all registrations for a matching `type_name`.
    pub async fn lookup_type(&self, name: &str) -> Option<TypeRegistration> {
        let regs = self.type_registrations.read().await;
        regs.values().find(|r| r.type_name == name).cloned()
    }

    /// Look up a type registration by its content-addressed hash.
    pub async fn lookup_type_by_hash(&self, hash: &str) -> Option<TypeRegistration> {
        let regs = self.type_registrations.read().await;
        regs.get(hash).cloned()
    }

    /// List all type registrations.
    pub async fn list_type_registrations(&self) -> Vec<TypeRegistration> {
        let regs = self.type_registrations.read().await;
        regs.values().cloned().collect()
    }

    /// Get a type definition by name
    pub async fn get_type_definition(&self, name: &str) -> Option<AssetTypeDefinition> {
        let defs = self.type_definitions.read().await;
        defs.get(name).cloned()
    }

    /// Get registry ID
    pub fn registry_id(&self) -> &AssetRegistration {
        &self.registry_id
    }

    /// Get privacy level
    pub fn privacy_level(&self) -> &PrivacyMode {
        &self.privacy
    }
}

/// A single resolved dependency node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyNode {
    /// Content-addressed typedef hash.
    pub type_hash: ContentHash,
    /// Human-readable typedef name at resolution time.
    pub name: String,
    /// Typedef version at resolution time.
    pub version: String,
    /// BFS distance from the root (root = 0, direct = 1, transitive ≥ 2).
    pub depth: usize,
}

/// Full dependency graph returned by [`CatalogRegistry::resolve_dependencies`].
///
/// `missing` captures any declared/embedded reference that has no registered
/// typedef — honest tracking, NOT silently dropped.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Typedefs the root immediately depends on (depth = 1).
    pub direct: Vec<DependencyNode>,
    /// Typedefs reached via direct deps (depth ≥ 2). Each appears at most
    /// once even if reachable by multiple paths.
    pub transitive: Vec<DependencyNode>,
    /// Declared or embedded hashes that did not resolve to a registered
    /// typedef.
    pub missing: Vec<ContentHash>,
}

/// Search query (compatible with old registry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub query: String,

    /// Asset type filter
    pub asset_type: Option<String>,

    /// Tag filters
    pub tags: Vec<String>,

    /// Author filter
    pub author: Option<String>,

    /// Version constraints
    pub version: Option<String>,

    /// Date range filter
    pub date_range: Option<DateRange>,

    /// Sort criteria
    pub sort_by: SortCriteria,

    /// Maximum results to return
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,
}

/// Sort criteria for search results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SortCriteria {
    #[default]
    Relevance,
    Name,
    Rating,
    Downloads,
    Updated,
    Published,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            asset_type: None,
            tags: Vec::new(),
            author: None,
            version: None,
            date_range: None,
            sort_by: SortCriteria::default(),
            limit: 20,
            offset: 0,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Type name
    pub type_name: String,

    /// Asset ID
    pub asset_id: AssetRegistration,

    /// Relevance score (0.0 - 1.0)
    pub score: f64,

    /// Publisher authentication status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_authenticated: Option<bool>,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Matching results
    pub results: Vec<SearchResult>,

    /// Total count (before limit)
    pub total_count: usize,

    /// Original query
    pub query: SearchQuery,
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatistics {
    /// Total registered types
    pub total_types: usize,

    /// Registry asset ID
    pub registry_id: String,

    /// Privacy level
    pub privacy_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::asset_type::AssetTypeDefinition;
    use blockmatrix::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof,
    };
    use serde_json::json;
    use std::time::Duration;

    fn create_test_state_proof() -> StateProof {
        let stake_proof = StakeProof::new("test-holder".to_string(), "test-id".to_string());
        let space_proof = SpaceProof::new("test-node".to_string(), "/test".to_string(), 1024);
        let work_proof =
            WorkProof::from_work("test-owner".to_string(), "test-workload".to_string(), b"work");
        let time_proof = TimeProof::new(Duration::from_secs(10));
        StateProof::new(stake_proof, time_proof, space_proof, work_proof)
    }

    #[tokio::test]
    async fn test_register_and_find_type() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = json!({
            "type": "object",
            "properties": {
                "vin": { "type": "string" }
            }
        });

        let state_proof = create_test_state_proof();
        let type_def = AssetTypeDefinition::new("Vehicle".to_string(), schema, state_proof);

        let asset_id = registry.register_type(type_def).await.expect("test: async operation");
        let found_id = registry.find_type("Vehicle").await.expect("test: async operation");

        assert_eq!(asset_id, found_id);
    }

    #[tokio::test]
    async fn test_search_types() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        // Register multiple types
        for name in &["Vehicle", "VehicleInsurance", "Driver"] {
            let schema = json!({ "type": "object" });
            let state_proof = create_test_state_proof();
            let type_def = AssetTypeDefinition::new(name.to_string(), schema, state_proof);
            registry.register_type(type_def).await.expect("test: async operation");
        }

        // Search for "Vehicle"
        let query = SearchQuery {
            query: "Vehicle".to_string(),
            ..Default::default()
        };

        let results = registry.search_types(&query).await.expect("test: async operation");
        assert_eq!(results.results.len(), 2); // Vehicle and VehicleInsurance
    }

    #[tokio::test]
    async fn test_registry_statistics() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let stats = registry.get_statistics().await;
        assert_eq!(stats.total_types, 0);
    }

    #[tokio::test]
    async fn test_register_type_creates_hash() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = json!({
            "type": "object",
            "properties": { "name": { "type": "string" } }
        });

        let state_proof = create_test_state_proof();
        let type_def = AssetTypeDefinition::new("TestType".to_string(), schema.clone(), state_proof);
        registry.register_type(type_def).await.expect("test: register");

        let reg = registry.lookup_type("TestType").await.expect("test: lookup");

        // Verify hash matches BLAKE3 of canonical schema JSON
        let expected_hash = hex::encode(
            blake3::hash(serde_json::to_string(&schema).expect("test: json").as_bytes()).as_bytes(),
        );
        assert_eq!(reg.type_hash, expected_hash);
        assert_eq!(reg.type_name, "TestType");
        assert_eq!(reg.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_register_duplicate_type_fails() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = json!({ "type": "object" });
        let proof1 = create_test_state_proof();
        let type_def1 = AssetTypeDefinition::new("DupType".to_string(), schema.clone(), proof1);
        registry.register_type(type_def1).await.expect("test: first register");

        let proof2 = create_test_state_proof();
        let type_def2 = AssetTypeDefinition::new("DupType".to_string(), schema, proof2);
        let result = registry.register_type(type_def2).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("already registered"),
            "Error should mention already registered"
        );
    }

    #[tokio::test]
    async fn test_lookup_type_by_name() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = json!({ "type": "object", "description": "test" });
        let proof = create_test_state_proof();
        let type_def = AssetTypeDefinition::new("ByNameType".to_string(), schema, proof);
        registry.register_type(type_def).await.expect("test: register");

        assert!(registry.lookup_type("ByNameType").await.is_some());
        assert!(registry.lookup_type("NonExistent").await.is_none());
    }

    #[tokio::test]
    async fn test_lookup_type_by_hash() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = json!({ "type": "object", "id": "hash-test" });
        let proof = create_test_state_proof();
        let type_def = AssetTypeDefinition::new("HashLookup".to_string(), schema.clone(), proof);
        registry.register_type(type_def).await.expect("test: register");

        let expected_hash = hex::encode(
            blake3::hash(serde_json::to_string(&schema).expect("test: json").as_bytes()).as_bytes(),
        );

        let reg = registry
            .lookup_type_by_hash(&expected_hash)
            .await
            .expect("test: lookup by hash");
        assert_eq!(reg.type_name, "HashLookup");

        assert!(registry.lookup_type_by_hash("badhash").await.is_none());
    }

    #[tokio::test]
    async fn test_list_type_registrations() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        for name in &["Alpha", "Beta", "Gamma"] {
            let schema = json!({ "type": "object", "name": name });
            let proof = create_test_state_proof();
            let type_def = AssetTypeDefinition::new(name.to_string(), schema, proof);
            registry.register_type(type_def).await.expect("test: register");
        }

        let all = registry.list_type_registrations().await;
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_builtin_types_preregistered() {
        let registry = CatalogRegistry::with_builtin_types(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        )
        .await;

        // Verify all three built-in types exist
        assert!(registry.lookup_type("Message").await.is_some());
        assert!(registry.lookup_type("Invitation").await.is_some());
        assert!(registry.lookup_type("Document").await.is_some());

        // Verify they appear in list
        let all = registry.list_type_registrations().await;
        assert_eq!(all.len(), 3);

        // Verify schemas have expected required fields
        let msg = registry.lookup_type("Message").await.expect("test: Message");
        let required = msg.schema.get("required").expect("test: required field");
        assert!(required.as_array().expect("test: array").iter().any(|v| v == "sender_node_id"));
    }

    // ===========================================================================
    // Dependency resolution (BFS, content-addressed)
    // ===========================================================================

    /// Permissive trust policy for dependency-resolution tests so we can wire
    /// up arbitrary registered typedefs without constructing real proofs.
    fn relaxed_policy() -> TrustPolicy {
        TrustPolicy {
            require_state_proof: false,
            allowed_publishers: Vec::new(),
            require_certificate: false,
        }
    }

    /// Register `name` with the given schema and return its computed
    /// canonical [`ContentHash`].
    async fn register_named(
        registry: &CatalogRegistry,
        name: &str,
        schema: serde_json::Value,
        dependencies: Vec<ContentHash>,
    ) -> ContentHash {
        let mut type_def =
            AssetTypeDefinition::new(name.to_string(), schema.clone(), create_test_state_proof());
        for d in dependencies {
            type_def.add_dependency(d);
        }
        registry.register_type(type_def).await.expect("test: register");
        let hex_str = hex::encode(
            blake3::hash(serde_json::to_string(&schema).expect("test: json").as_bytes()).as_bytes(),
        );
        let mut bytes = [0u8; 32];
        let decoded = hex::decode(&hex_str).expect("test: hex decode");
        bytes.copy_from_slice(&decoded);
        ContentHash::from_bytes(bytes)
    }

    #[tokio::test]
    async fn test_resolve_dependencies_empty() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        let root = register_named(&registry, "Solo", json!({"type": "object"}), vec![]).await;
        let graph = registry.resolve_dependencies(&root).await.expect("resolve");
        assert!(graph.direct.is_empty());
        assert!(graph.transitive.is_empty());
        assert!(graph.missing.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_dependencies_direct() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        // B is a leaf, A depends on B.
        let b_hash = register_named(&registry, "B", json!({"type": "object", "id": "b"}), vec![]).await;
        let a_hash =
            register_named(&registry, "A", json!({"type": "object", "id": "a"}), vec![b_hash]).await;

        let graph = registry.resolve_dependencies(&a_hash).await.expect("resolve");
        assert_eq!(graph.direct.len(), 1, "exactly one direct dep");
        assert_eq!(graph.direct[0].name, "B");
        assert_eq!(graph.direct[0].type_hash, b_hash);
        assert_eq!(graph.direct[0].depth, 1);
        assert!(graph.transitive.is_empty());
        assert!(graph.missing.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_dependencies_transitive() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        // C leaf, B → C, A → B.
        let c = register_named(&registry, "C", json!({"type": "object", "id": "c"}), vec![]).await;
        let b = register_named(&registry, "B", json!({"type": "object", "id": "b"}), vec![c]).await;
        let a = register_named(&registry, "A", json!({"type": "object", "id": "a"}), vec![b]).await;

        let graph = registry.resolve_dependencies(&a).await.expect("resolve");
        assert_eq!(graph.direct.len(), 1);
        assert_eq!(graph.direct[0].name, "B");
        assert_eq!(graph.direct[0].depth, 1);
        assert_eq!(graph.transitive.len(), 1, "C is transitive");
        assert_eq!(graph.transitive[0].name, "C");
        assert_eq!(graph.transitive[0].depth, 2);
        assert!(graph.missing.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_dependencies_missing() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        // A declares a dep on a hash that does NOT exist in the registry.
        let phantom = ContentHash::from_bytes(*blake3::hash(b"phantom").as_bytes());
        let a = register_named(
            &registry,
            "A",
            json!({"type": "object", "id": "a"}),
            vec![phantom],
        )
        .await;

        let graph = registry.resolve_dependencies(&a).await.expect("resolve");
        assert!(graph.direct.is_empty(), "phantom resolved → missing, not direct");
        assert_eq!(graph.missing.len(), 1);
        assert_eq!(graph.missing[0], phantom);
    }

    #[tokio::test]
    async fn test_resolve_dependencies_root_missing() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        let nonexistent = ContentHash::from_bytes(*blake3::hash(b"unknown-root").as_bytes());
        let graph = registry.resolve_dependencies(&nonexistent).await.expect("resolve");
        assert!(graph.direct.is_empty());
        assert!(graph.transitive.is_empty());
        assert_eq!(graph.missing, vec![nonexistent]);
    }

    #[tokio::test]
    async fn test_resolve_dependencies_cycle() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        // Register B first with NO deps (its canonical hash is fixed by schema).
        let b = register_named(&registry, "B", json!({"type": "object", "id": "b"}), vec![]).await;
        // Register A → B.
        let a = register_named(&registry, "A", json!({"type": "object", "id": "a"}), vec![b]).await;
        // Mutate B's stored definition so B → A (introduce the cycle).
        // We do this directly on the type_definitions map because the public
        // API does not support replacing a registered definition.
        {
            let mut defs = registry.type_definitions.write().await;
            let b_def = defs.get_mut("B").expect("B registered");
            b_def.add_dependency(a);
        }

        // Now A → B → A. BFS must terminate.
        let graph = registry.resolve_dependencies(&a).await.expect("resolve");
        // Direct: B. Transitive: nothing (A is the root, already visited).
        assert_eq!(graph.direct.len(), 1);
        assert_eq!(graph.direct[0].name, "B");
        assert!(
            graph.transitive.is_empty(),
            "A reaches itself via B → A but is already visited; expected empty transitive, got {:?}",
            graph.transitive
        );
        assert!(graph.missing.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_dependencies_disabled() {
        let mut cfg = RegistryConfig::default();
        cfg.enable_dependency_resolution = false;
        let registry = CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), cfg);

        let b = register_named(&registry, "B", json!({"type": "object", "id": "b"}), vec![]).await;
        let a = register_named(&registry, "A", json!({"type": "object", "id": "a"}), vec![b]).await;
        let graph = registry.resolve_dependencies(&a).await.expect("resolve");
        assert!(graph.direct.is_empty());
        assert!(graph.transitive.is_empty());
        assert!(graph.missing.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_dependencies_embedded_ref_via_scanner() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, relaxed_policy(), RegistryConfig::default());

        // Register a leaf typedef, then a parent whose schema body EMBEDS the
        // leaf's hex hash (without declaring it via `dependencies`).
        let leaf_schema = json!({"type": "object", "id": "leaf-emb"});
        let leaf_hash = register_named(&registry, "Leaf", leaf_schema, vec![]).await;
        let leaf_hex = hex::encode(leaf_hash.as_bytes());

        let parent_schema = json!({
            "type": "object",
            "linked": format!("typedef://{}", leaf_hex)
        });
        let parent_hash = register_named(&registry, "Parent", parent_schema, vec![]).await;

        let graph = registry.resolve_dependencies(&parent_hash).await.expect("resolve");
        assert_eq!(
            graph.direct.len(),
            1,
            "embedded ref must surface as a direct dep"
        );
        assert_eq!(graph.direct[0].type_hash, leaf_hash);
        assert_eq!(graph.direct[0].name, "Leaf");
        assert!(graph.transitive.is_empty());
        assert!(graph.missing.is_empty());
    }
}

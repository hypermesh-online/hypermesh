// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Catalog provider trait.
//!
//! Phase M.4.5b — defines a narrow dyn-safe trait that the IPC layer uses
//! to query the catalog registry without depending on the `catalog` crate.
//! The `catalog` crate already depends on `blockmatrix`, so the reverse
//! direction would form a cycle. Instead, an adapter in the `catalog`
//! crate (or a downstream binary that pulls both) implements this trait
//! and the daemon stores it as `Arc<dyn CatalogProvider>`.
//!
//! Alpha-default inert: when the daemon's
//! [`crate::ipc::state::DaemonState::catalog_registry`] is `None`, the
//! `catalog.dependencies` IPC handler returns an honest "not wired"
//! response rather than a fabricated empty graph.

use async_trait::async_trait;
use hypermesh_lib::ContentHash;
use serde::{Deserialize, Serialize};

/// A single resolved typedef node in a dependency graph.
///
/// Mirrors `catalog::registry::DependencyNode` so the trait remains free
/// of any catalog-crate types (which would re-introduce the dependency
/// cycle).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogDependencyNode {
    /// Content-addressed typedef hash.
    pub type_hash: ContentHash,
    /// Human-readable typedef name at resolution time.
    pub name: String,
    /// Typedef version at resolution time.
    pub version: String,
    /// BFS distance from the root (root = 0, direct = 1, transitive >= 2).
    pub depth: usize,
}

/// Full dependency graph returned by [`CatalogProvider::resolve_dependencies`].
///
/// `missing` is honest tracking of declared/embedded references that
/// failed to resolve — never silently dropped.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogDependencyGraph {
    /// Typedefs the root immediately depends on (depth = 1).
    pub direct: Vec<CatalogDependencyNode>,
    /// Typedefs reached via direct deps (depth >= 2). Each appears at
    /// most once even if reachable by multiple paths.
    pub transitive: Vec<CatalogDependencyNode>,
    /// Declared or embedded hashes that did not resolve to a registered
    /// typedef.
    pub missing: Vec<ContentHash>,
}

/// Minimal information about a registered typedef, used to translate
/// type-name lookups into a content hash before resolving the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogTypeInfo {
    /// Typedef name.
    pub type_name: String,
    /// Canonical content hash (BLAKE3 of the canonical schema JSON).
    pub type_hash: ContentHash,
    /// Typedef version at registration time.
    pub version: String,
}

/// Errors returned by [`CatalogProvider`] operations.
#[derive(Debug, thiserror::Error)]
pub enum CatalogProviderError {
    /// The underlying registry rejected the call (e.g. internal failure).
    #[error("catalog registry error: {0}")]
    Registry(String),
}

/// Read-only abstraction over a catalog registry.
///
/// Trait object–safe (only `&self` async methods). Held by the daemon as
/// `Arc<dyn CatalogProvider>` in [`crate::ipc::state::DaemonState`].
///
/// The trait deliberately uses [`ContentHash`] and the small mirror types
/// declared in this module (rather than `catalog::registry` types) so
/// that no blockmatrix code depends on the `catalog` crate.
#[async_trait]
pub trait CatalogProvider: Send + Sync {
    /// Resolve the dependency graph for a typedef identified by its
    /// content hash.
    ///
    /// Returns a graph with `direct`, `transitive`, and `missing`
    /// populated. If `root` is itself not registered, the returned graph
    /// has empty `direct`/`transitive` and `missing = [root]`. This is
    /// honest state, not an error.
    async fn resolve_dependencies(
        &self,
        root: &ContentHash,
    ) -> Result<CatalogDependencyGraph, CatalogProviderError>;

    /// Look up a typedef by human-readable name. Returns `Ok(None)` if
    /// the name is not registered.
    async fn lookup_by_name(
        &self,
        name: &str,
    ) -> Result<Option<CatalogTypeInfo>, CatalogProviderError>;

    /// Search local typedefs by name substring or exact match.
    ///
    /// Returns at most `limit` matches sorted by the underlying
    /// registry's relevance scoring. An empty query returns the most
    /// relevant typedefs the registry knows about (browse-all
    /// semantics). Returns `Ok(Vec::new())` when nothing matches —
    /// never a synthetic row.
    ///
    /// Default implementation returns an empty vector so that older
    /// `CatalogProvider` impls (e.g. test fakes) compile unchanged.
    async fn search_local(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<CatalogTypeInfo>, CatalogProviderError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! In-crate fake [`CatalogProvider`] used by the IPC handler tests so
    //! we can exercise the real-data path without pulling in the
    //! `catalog` crate (which would create a dependency cycle for
    //! `blockmatrix`'s own tests).
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    /// A simple in-memory typedef record used by the fake.
    #[derive(Debug, Clone)]
    pub struct FakeTypedef {
        pub info: CatalogTypeInfo,
        pub dependencies: Vec<ContentHash>,
    }

    /// Fake provider that supports a small, hand-built dependency graph.
    #[derive(Default)]
    pub struct FakeCatalogProvider {
        by_hash: RwLock<HashMap<ContentHash, FakeTypedef>>,
        by_name: RwLock<HashMap<String, ContentHash>>,
    }

    impl FakeCatalogProvider {
        pub fn new() -> Self {
            Self::default()
        }

        /// Insert a typedef. Repeated insertions overwrite.
        pub async fn insert(&self, def: FakeTypedef) {
            let hash = def.info.type_hash;
            let name = def.info.type_name.clone();
            self.by_hash.write().await.insert(hash, def);
            self.by_name.write().await.insert(name, hash);
        }
    }

    #[async_trait]
    impl CatalogProvider for FakeCatalogProvider {
        async fn resolve_dependencies(
            &self,
            root: &ContentHash,
        ) -> Result<CatalogDependencyGraph, CatalogProviderError> {
            let by_hash = self.by_hash.read().await;
            let root_def = match by_hash.get(root) {
                Some(d) => d,
                None => {
                    return Ok(CatalogDependencyGraph {
                        direct: Vec::new(),
                        transitive: Vec::new(),
                        missing: vec![*root],
                    });
                }
            };

            let mut direct: Vec<CatalogDependencyNode> = Vec::new();
            let mut transitive: Vec<CatalogDependencyNode> = Vec::new();
            let mut missing: Vec<ContentHash> = Vec::new();
            let mut visited: std::collections::HashSet<ContentHash> =
                std::collections::HashSet::new();
            visited.insert(*root);

            // Resolve direct deps.
            let mut queue: std::collections::VecDeque<(ContentHash, usize)> =
                std::collections::VecDeque::new();
            for dep in &root_def.dependencies {
                if !visited.insert(*dep) {
                    continue;
                }
                match by_hash.get(dep) {
                    Some(child) => {
                        direct.push(CatalogDependencyNode {
                            type_hash: *dep,
                            name: child.info.type_name.clone(),
                            version: child.info.version.clone(),
                            depth: 1,
                        });
                        queue.push_back((*dep, 1));
                    }
                    None => missing.push(*dep),
                }
            }

            // BFS transitive.
            while let Some((parent, depth)) = queue.pop_front() {
                let parent_def = match by_hash.get(&parent) {
                    Some(d) => d,
                    None => continue,
                };
                for dep in &parent_def.dependencies {
                    if !visited.insert(*dep) {
                        continue;
                    }
                    match by_hash.get(dep) {
                        Some(child) => {
                            transitive.push(CatalogDependencyNode {
                                type_hash: *dep,
                                name: child.info.type_name.clone(),
                                version: child.info.version.clone(),
                                depth: depth + 1,
                            });
                            queue.push_back((*dep, depth + 1));
                        }
                        None => missing.push(*dep),
                    }
                }
            }

            Ok(CatalogDependencyGraph {
                direct,
                transitive,
                missing,
            })
        }

        async fn lookup_by_name(
            &self,
            name: &str,
        ) -> Result<Option<CatalogTypeInfo>, CatalogProviderError> {
            let by_name = self.by_name.read().await;
            let hash = match by_name.get(name) {
                Some(h) => *h,
                None => return Ok(None),
            };
            drop(by_name);
            let by_hash = self.by_hash.read().await;
            Ok(by_hash.get(&hash).map(|d| d.info.clone()))
        }

        async fn search_local(
            &self,
            query: &str,
            limit: usize,
        ) -> Result<Vec<CatalogTypeInfo>, CatalogProviderError> {
            let by_hash = self.by_hash.read().await;
            let q = query.to_lowercase();
            let mut out: Vec<CatalogTypeInfo> = by_hash
                .values()
                .filter(|d| {
                    q.is_empty() || d.info.type_name.to_lowercase().contains(&q)
                })
                .map(|d| d.info.clone())
                .collect();
            // Deterministic ordering so tests don't flap.
            out.sort_by(|a, b| a.type_name.cmp(&b.type_name));
            out.truncate(limit);
            Ok(out)
        }
    }
}

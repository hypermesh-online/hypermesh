// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Adapter that exposes [`CatalogRegistry`] as the blockmatrix-side
//! [`CatalogProvider`] trait.
//!
//! Phase M.4.5c.1 — closes the loop opened by M.4.5b. The
//! `blockmatrix::catalog::CatalogProvider` trait was defined so the IPC
//! daemon could query a typedef registry without depending on the
//! `catalog` crate (catalog already depends on blockmatrix, so the
//! reverse direction would form a cycle). This adapter implements that
//! trait against a real [`CatalogRegistry`] and lives here in catalog —
//! the only direction that compiles.
//!
//! Wiring: see `wire_catalog_registry` for the factory that downstream
//! binaries should use to populate
//! `blockmatrix::ipc::DaemonState::catalog_registry`.

use std::sync::Arc;

use async_trait::async_trait;
use blockmatrix::catalog::provider::{
    CatalogDependencyGraph, CatalogDependencyNode, CatalogProvider, CatalogProviderError,
    CatalogTypeInfo,
};
use hypermesh_lib::ContentHash;

use crate::registry::{
    CatalogRegistry, DependencyGraph, DependencyNode, SearchQuery, SortCriteria,
};

/// Adapter wrapping an `Arc<CatalogRegistry>` and exposing the
/// `blockmatrix::catalog::CatalogProvider` trait surface.
///
/// Thin shim: every method delegates to the real registry, mapping the
/// catalog-side types to the blockmatrix mirror types so no `catalog`
/// type ever leaks across the crate boundary.
pub struct CatalogRegistryAdapter {
    inner: Arc<CatalogRegistry>,
}

impl CatalogRegistryAdapter {
    /// Wrap an existing registry. The registry stays under `Arc` so the
    /// adapter can be cheaply cloned and shared with the IPC daemon.
    pub fn new(registry: Arc<CatalogRegistry>) -> Self {
        Self { inner: registry }
    }

    /// Construct the adapter as a `dyn CatalogProvider` trait object
    /// ready to drop into `DaemonState::catalog_registry`.
    pub fn into_provider(self) -> Arc<dyn CatalogProvider> {
        Arc::new(self) as Arc<dyn CatalogProvider>
    }
}

/// Build a fully-wired `CatalogProvider` from a registry.
///
/// Convenience for downstream binaries that need to populate
/// `DaemonState::catalog_registry` at daemon startup. Takes ownership of
/// the registry `Arc` so the same registry can be shared between the
/// adapter and any other in-process catalog surfaces (e.g. the catalog
/// STOQ API server).
pub fn wire_catalog_registry(registry: Arc<CatalogRegistry>) -> Arc<dyn CatalogProvider> {
    CatalogRegistryAdapter::new(registry).into_provider()
}

#[async_trait]
impl CatalogProvider for CatalogRegistryAdapter {
    async fn resolve_dependencies(
        &self,
        root: &ContentHash,
    ) -> Result<CatalogDependencyGraph, CatalogProviderError> {
        let graph = self
            .inner
            .resolve_dependencies(root)
            .await
            .map_err(|e| CatalogProviderError::Registry(e.to_string()))?;
        Ok(map_graph(graph))
    }

    async fn lookup_by_name(
        &self,
        name: &str,
    ) -> Result<Option<CatalogTypeInfo>, CatalogProviderError> {
        // `CatalogRegistry::lookup_type` is infallible (returns Option),
        // so this method never produces a Registry error today. The
        // Result-wrapped signature is preserved by the trait so callers
        // can stay forward-compatible with future stricter lookups.
        let reg = match self.inner.lookup_type(name).await {
            Some(r) => r,
            None => return Ok(None),
        };
        let hash = parse_hex_hash(&reg.type_hash).map_err(CatalogProviderError::Registry)?;
        Ok(Some(CatalogTypeInfo {
            type_name: reg.type_name,
            type_hash: hash,
            version: reg.version,
        }))
    }

    async fn search_local(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CatalogTypeInfo>, CatalogProviderError> {
        // Drive the real registry's relevance-sorted search and then
        // re-look-up each result to obtain its canonical content hash
        // (search results carry only the type_name + AssetRegistration;
        // the BLAKE3 hash lives on the TypeRegistration record).
        let search_query = SearchQuery {
            query: query.to_string(),
            sort_by: SortCriteria::Relevance,
            limit: limit.max(1),
            offset: 0,
            ..Default::default()
        };
        let results = self
            .inner
            .search_types(&search_query)
            .await
            .map_err(|e| CatalogProviderError::Registry(e.to_string()))?;

        let mut out: Vec<CatalogTypeInfo> = Vec::with_capacity(results.results.len());
        for r in &results.results {
            let reg = match self.inner.lookup_type(&r.type_name).await {
                Some(reg) => reg,
                None => continue,
            };
            match parse_hex_hash(&reg.type_hash) {
                Ok(hash) => out.push(CatalogTypeInfo {
                    type_name: reg.type_name,
                    type_hash: hash,
                    version: reg.version,
                }),
                // Skip a row with a malformed stored hash rather than
                // fabricating one. Logged at the registry layer.
                Err(_) => continue,
            }
        }
        Ok(out)
    }
}

/// Convert a `catalog::registry::DependencyGraph` into the blockmatrix
/// mirror type. Pure data mapping — no async, no fallibility.
fn map_graph(g: DependencyGraph) -> CatalogDependencyGraph {
    CatalogDependencyGraph {
        direct: g.direct.into_iter().map(map_node).collect(),
        transitive: g.transitive.into_iter().map(map_node).collect(),
        missing: g.missing,
    }
}

fn map_node(n: DependencyNode) -> CatalogDependencyNode {
    CatalogDependencyNode {
        type_hash: n.type_hash,
        name: n.name,
        version: n.version,
        depth: n.depth,
    }
}

/// Parse a 64-char hex string into a [`ContentHash`]. Returns an error
/// describing the failure so it can surface through
/// [`CatalogProviderError::Registry`].
fn parse_hex_hash(hex_str: &str) -> Result<ContentHash, String> {
    if hex_str.len() != 64 {
        return Err(format!(
            "stored type_hash must be 64 hex chars (BLAKE3-256), got {}",
            hex_str.len()
        ));
    }
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex in type_hash: {e}"))?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(ContentHash::from_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::asset_type::AssetTypeDefinition;
    use crate::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
    use hypermesh_lib::PrivacyMode;
    use serde_json::json;

    /// Permissive policy so tests can register typedefs without
    /// constructing real Proof-of-State proofs.
    fn relaxed_policy() -> TrustPolicy {
        TrustPolicy {
            require_state_proof: false,
            minimum_stake: 0,
            allowed_publishers: Vec::new(),
            require_certificate: false,
        }
    }

    fn relaxed_state_proof() -> blockmatrix::assets::StateProof {
        CatalogRegistry::builtin_state_proof()
    }

    /// Register `name` with the given schema + dependency list and
    /// return its canonical content hash.
    async fn register_named(
        registry: &CatalogRegistry,
        name: &str,
        schema: serde_json::Value,
        dependencies: Vec<ContentHash>,
    ) -> ContentHash {
        let mut type_def =
            AssetTypeDefinition::new(name.to_string(), schema.clone(), relaxed_state_proof());
        for d in dependencies {
            type_def.add_dependency(d);
        }
        registry
            .register_type(type_def)
            .await
            .expect("test: register typedef");
        let hex_str = hex::encode(
            blake3::hash(
                serde_json::to_string(&schema)
                    .expect("test: schema serialization")
                    .as_bytes(),
            )
            .as_bytes(),
        );
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&hex::decode(&hex_str).expect("test: hex decode"));
        ContentHash::from_bytes(bytes)
    }

    fn new_registry() -> Arc<CatalogRegistry> {
        Arc::new(CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            relaxed_policy(),
            RegistryConfig::default(),
        ))
    }

    #[tokio::test]
    async fn test_adapter_resolves_real_graph() {
        // A -> B -> C registered through the real registry; the
        // adapter must surface direct = [B], transitive = [C], no
        // missing, all depths correct.
        let registry = new_registry();
        let c = register_named(&registry, "C", json!({"type": "object", "id": "c"}), vec![]).await;
        let b =
            register_named(&registry, "B", json!({"type": "object", "id": "b"}), vec![c]).await;
        let a =
            register_named(&registry, "A", json!({"type": "object", "id": "a"}), vec![b]).await;

        let adapter = CatalogRegistryAdapter::new(registry);
        let graph = adapter
            .resolve_dependencies(&a)
            .await
            .expect("test: resolve through adapter");

        assert_eq!(graph.direct.len(), 1, "exactly one direct dep");
        assert_eq!(graph.direct[0].name, "B");
        assert_eq!(graph.direct[0].type_hash, b);
        assert_eq!(graph.direct[0].depth, 1);

        assert_eq!(graph.transitive.len(), 1, "C is transitive");
        assert_eq!(graph.transitive[0].name, "C");
        assert_eq!(graph.transitive[0].type_hash, c);
        assert_eq!(graph.transitive[0].depth, 2);

        assert!(graph.missing.is_empty(), "no missing deps");
    }

    #[tokio::test]
    async fn test_adapter_resolves_missing_root_honestly() {
        let registry = new_registry();
        let adapter = CatalogRegistryAdapter::new(registry);
        let phantom = ContentHash::from_bytes(*blake3::hash(b"phantom-root").as_bytes());
        let graph = adapter
            .resolve_dependencies(&phantom)
            .await
            .expect("test: resolve unknown root");
        assert!(graph.direct.is_empty());
        assert!(graph.transitive.is_empty());
        assert_eq!(graph.missing, vec![phantom]);
    }

    #[tokio::test]
    async fn test_adapter_resolves_missing_dep_surfaces_in_missing() {
        let registry = new_registry();
        let phantom = ContentHash::from_bytes(*blake3::hash(b"phantom-dep").as_bytes());
        let a = register_named(
            &registry,
            "A",
            json!({"type": "object", "id": "a-missing-dep"}),
            vec![phantom],
        )
        .await;

        let adapter = CatalogRegistryAdapter::new(registry);
        let graph = adapter
            .resolve_dependencies(&a)
            .await
            .expect("test: resolve through adapter");
        assert!(graph.direct.is_empty(), "phantom is missing, not direct");
        assert_eq!(graph.missing, vec![phantom]);
    }

    #[tokio::test]
    async fn test_adapter_lookup_by_name() {
        let registry = new_registry();
        let b_hash =
            register_named(&registry, "B", json!({"type": "object", "id": "b-lk"}), vec![]).await;

        let adapter = CatalogRegistryAdapter::new(registry);
        let info = adapter
            .lookup_by_name("B")
            .await
            .expect("test: lookup_by_name")
            .expect("test: B registered");

        assert_eq!(info.type_name, "B");
        assert_eq!(info.type_hash, b_hash);
        assert_eq!(info.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_adapter_lookup_by_name_unknown_returns_none() {
        let registry = new_registry();
        let adapter = CatalogRegistryAdapter::new(registry);
        let info = adapter
            .lookup_by_name("Nonexistent")
            .await
            .expect("test: lookup_by_name");
        assert!(info.is_none(), "unknown name must return None");
    }

    #[tokio::test]
    async fn test_adapter_search_local_returns_matches_with_hash() {
        let registry = new_registry();
        register_named(
            &registry,
            "Message",
            json!({"type": "object", "id": "msg"}),
            vec![],
        )
        .await;
        register_named(
            &registry,
            "Invitation",
            json!({"type": "object", "id": "inv"}),
            vec![],
        )
        .await;
        register_named(
            &registry,
            "Document",
            json!({"type": "object", "id": "doc"}),
            vec![],
        )
        .await;

        let adapter = CatalogRegistryAdapter::new(registry);
        let results = adapter
            .search_local("Mess", 10)
            .await
            .expect("test: search_local");
        assert_eq!(results.len(), 1, "exactly one prefix match");
        assert_eq!(results[0].type_name, "Message");
        // Hash must be the canonical BLAKE3 of the schema bytes.
        let expected_hash = blake3::hash(
            serde_json::to_string(&json!({"type": "object", "id": "msg"}))
                .expect("test: schema serialization")
                .as_bytes(),
        );
        assert_eq!(results[0].type_hash.as_bytes(), expected_hash.as_bytes());
    }

    #[tokio::test]
    async fn test_adapter_search_local_no_matches_returns_empty() {
        let registry = new_registry();
        register_named(
            &registry,
            "Message",
            json!({"type": "object", "id": "msg2"}),
            vec![],
        )
        .await;
        let adapter = CatalogRegistryAdapter::new(registry);
        let results = adapter
            .search_local("ZZZ-no-match", 10)
            .await
            .expect("test: search_local");
        assert!(results.is_empty(), "no matches must be honest empty");
    }

    #[tokio::test]
    async fn test_wire_catalog_registry_factory() {
        // Confirm the public factory returns a usable trait object.
        let registry = new_registry();
        let _hash =
            register_named(&registry, "X", json!({"type": "object", "id": "x"}), vec![]).await;
        let provider = wire_catalog_registry(registry);
        // Trait method through Arc<dyn ...>
        let info = provider
            .lookup_by_name("X")
            .await
            .expect("test: provider lookup")
            .expect("test: X registered");
        assert_eq!(info.type_name, "X");
    }
}

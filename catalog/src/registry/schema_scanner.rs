// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Schema-content typedef-reference scanner.
//!
//! Walks a typedef's schema body looking for embedded BLAKE3 hashes that
//! happen to be the content-hash of another registered typedef. This is a
//! pure type-system probe — it does NOT inspect instance payloads, only the
//! schema definition itself.
//!
//! Two streams of dependencies are tracked by the registry:
//!   1. Declared (`AssetTypeDefinition.dependencies`) — explicit, primary.
//!   2. Embedded (this scanner) — implicit, derived from the schema body.
//!
//! The two are unioned by `CatalogRegistry::resolve_dependencies`.

use std::collections::HashSet;
use std::sync::OnceLock;

use hypermesh_lib::ContentHash;
use regex::Regex;

use super::catalog_registry::CatalogRegistry;

/// Matches any 64-character lower-hex run — the canonical BLAKE3 hex form.
///
/// Word boundaries (`\b`) ensure we don't match a substring of a longer
/// hex run (e.g. 128-char SHA-512). The pattern is intentionally simple:
/// over-matching is fine because each candidate is then validated against
/// the registry, and uppercase variants are normalized below.
fn hex64_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b[0-9a-fA-F]{64}\b").expect("static regex compiles")
    })
}

/// Scan a typedef's schema bytes for embedded BLAKE3 hashes that match a
/// registered typedef. Returns the set of validated typedef hashes.
///
/// `self_hash` (if provided) is excluded from the result so a schema that
/// happens to mention its own content-hash does not declare itself as a
/// dependency.
pub async fn scan_schema_for_typedef_refs(
    schema_bytes: &[u8],
    registry: &CatalogRegistry,
    self_hash: Option<&ContentHash>,
) -> HashSet<ContentHash> {
    let mut found: HashSet<ContentHash> = HashSet::new();

    // Schemas are JSON in practice; if they are not valid UTF-8 we silently
    // skip the embedded-ref pass. Declared dependencies remain authoritative.
    let text = match std::str::from_utf8(schema_bytes) {
        Ok(t) => t,
        Err(_) => return found,
    };

    let re = hex64_regex();
    for m in re.find_iter(text) {
        let hex = m.as_str().to_ascii_lowercase();
        let bytes = match decode_hex32(&hex) {
            Some(b) => b,
            None => continue,
        };
        let candidate = ContentHash::from_bytes(bytes);

        if let Some(sh) = self_hash {
            if &candidate == sh {
                continue;
            }
        }

        if registry.lookup_type_by_hash(&hex).await.is_some() {
            found.insert(candidate);
        }
    }

    found
}

/// Decode a 64-char lowercase hex string into 32 raw bytes.
fn decode_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks_exact(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::asset_type::AssetTypeDefinition;
    use crate::registry::catalog_registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
    use blockmatrix::assets::StateProof;
    use blockmatrix::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use hypermesh_lib::PrivacyMode;
    use serde_json::json;
    use std::time::Duration;

    fn permissive_policy() -> TrustPolicy {
        TrustPolicy {
            require_state_proof: false,
            minimum_stake: 0,
            allowed_publishers: Vec::new(),
            require_certificate: false,
        }
    }

    fn test_state_proof() -> StateProof {
        let stake = StakeProof::new("t".to_string(), "t".to_string(), 0);
        let space = SpaceProof::new("t".to_string(), "/t".to_string(), 0);
        let work = WorkProof::new(
            "t".to_string(),
            "t".to_string(),
            0,
            0,
            WorkloadType::Compute,
            WorkState::Completed,
        );
        let time = TimeProof::new(Duration::from_secs(0));
        StateProof::new(stake, time, space, work)
    }

    /// Register a typedef and return its computed content hash (hex).
    async fn register(registry: &CatalogRegistry, name: &str, schema: serde_json::Value) -> String {
        let type_def = AssetTypeDefinition::new(name.to_string(), schema.clone(), test_state_proof());
        registry.register_type(type_def).await.expect("test: register");
        hex::encode(
            blake3::hash(serde_json::to_string(&schema).expect("test: json").as_bytes()).as_bytes(),
        )
    }

    #[tokio::test]
    async fn test_scan_schema_for_typedef_refs_finds_hex() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, permissive_policy(), RegistryConfig::default());

        // Register a "Target" typedef and capture its hex hash.
        let target_hex = register(&registry, "Target", json!({"type": "object", "id": "t"})).await;

        // Embed the target's hash inside another schema as a $ref.
        let schema_with_ref = json!({
            "type": "object",
            "properties": {
                "linked": { "$ref": format!("typedef://{}", target_hex) }
            }
        });
        let schema_bytes = serde_json::to_vec(&schema_with_ref).expect("test: vec");

        let refs = scan_schema_for_typedef_refs(&schema_bytes, &registry, None).await;
        assert_eq!(refs.len(), 1, "scanner should find the embedded ref");

        let mut bytes = [0u8; 32];
        for (i, chunk) in target_hex.as_bytes().chunks_exact(2).enumerate() {
            bytes[i] = u8::from_str_radix(
                std::str::from_utf8(chunk).expect("test: utf8"),
                16,
            )
            .expect("test: hex");
        }
        let target_ch = ContentHash::from_bytes(bytes);
        assert!(refs.contains(&target_ch));
    }

    #[tokio::test]
    async fn test_scan_schema_for_typedef_refs_ignores_unknown() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, permissive_policy(), RegistryConfig::default());

        // 64-char hex that is NOT a registered typedef hash.
        let unknown_hex = "deadbeef".repeat(8); // 64 chars
        assert_eq!(unknown_hex.len(), 64);

        let schema = json!({
            "type": "object",
            "note": format!("unknown ref {}", unknown_hex)
        });
        let schema_bytes = serde_json::to_vec(&schema).expect("test: vec");

        let refs = scan_schema_for_typedef_refs(&schema_bytes, &registry, None).await;
        assert!(
            refs.is_empty(),
            "scanner must not invent dependencies on unknown hashes"
        );
    }

    #[tokio::test]
    async fn test_scan_schema_excludes_self_hash() {
        let registry =
            CatalogRegistry::new(PrivacyMode::PUBLIC, permissive_policy(), RegistryConfig::default());

        let self_hex = register(&registry, "SelfRef", json!({"type": "object"})).await;

        // A schema that mentions its own hash should not depend on itself.
        let schema = json!({"type": "object", "self": self_hex});
        let schema_bytes = serde_json::to_vec(&schema).expect("test: vec");

        let mut bytes = [0u8; 32];
        for (i, chunk) in self_hex.as_bytes().chunks_exact(2).enumerate() {
            bytes[i] = u8::from_str_radix(
                std::str::from_utf8(chunk).expect("test: utf8"),
                16,
            )
            .expect("test: hex");
        }
        let self_ch = ContentHash::from_bytes(bytes);

        let refs = scan_schema_for_typedef_refs(&schema_bytes, &registry, Some(&self_ch)).await;
        assert!(refs.is_empty(), "self-references must be filtered");
    }

    #[test]
    fn test_decode_hex32_roundtrip() {
        let bytes = [0xABu8; 32];
        let hex = hex::encode(bytes);
        let decoded = decode_hex32(&hex).expect("test: decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn test_decode_hex32_rejects_wrong_length() {
        assert!(decode_hex32("abcd").is_none());
        assert!(decode_hex32(&"a".repeat(63)).is_none());
        assert!(decode_hex32(&"a".repeat(65)).is_none());
    }
}

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Dashboard deployment: bundle files, hash, register on blockchain, persist.

use std::collections::BTreeMap;
use std::path::Path;

use walkdir::WalkDir;

/// Collect all files under `dir` into a sorted map of relative path -> contents.
///
/// Returns an empty map if the directory does not exist.
pub fn collect_dir_files(dir: &Path) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read(entry.path())?;
            files.insert(rel, content);
        }
    }
    Ok(files)
}

/// Collect all dashboard files from the access scope directories defined in
/// the manifest.  Each entry key is prefixed with its scope
/// (e.g. `public/index.html`).
pub fn collect_dashboard_files(
    base_dir: &Path,
    access: &super::DashboardAccess,
) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    let mut all = BTreeMap::new();
    for (scope, rel_opt) in [
        ("public", &access.public),
        ("private", &access.private),
    ] {
        if let Some(rel) = rel_opt {
            let scope_dir = base_dir.join(rel);
            for (path, content) in collect_dir_files(&scope_dir)? {
                all.insert(format!("{scope}/{path}"), content);
            }
        }
    }
    Ok(all)
}

/// Encode a file map into a simple binary bundle format.
///
/// Format per entry: `path_len:u32le + path_bytes + content_len:u32le + content_bytes`
pub fn bundle_files(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut buf = Vec::new();
    for (path, content) in files {
        let path_bytes = path.as_bytes();
        buf.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(path_bytes);
        buf.extend_from_slice(&(content.len() as u32).to_le_bytes());
        buf.extend_from_slice(content);
    }
    buf
}

/// Compute the BLAKE3 hash of a byte slice, returning a 32-byte array.
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Store a dashboard bundle blob in the asset store, keyed by its BLAKE3 content hash.
///
/// Path: `<data_dir>/assets/dashboards/<hex_hash>.bundle`
///
/// The bundle contains both the manifest and all file data so that a single
/// content-hash lookup is sufficient to serve any dashboard.
pub fn store_dashboard_bundle(
    data_dir: &Path,
    content_hash: &[u8; 32],
    manifest_toml: &str,
    bundle: &[u8],
) -> std::io::Result<()> {
    let store_dir = data_dir.join("assets").join("dashboards");
    std::fs::create_dir_all(&store_dir)?;

    // Write the bundle (binary file map produced by bundle_files)
    let hash_hex = hex::encode(content_hash);
    std::fs::write(store_dir.join(format!("{hash_hex}.bundle")), bundle)?;

    // Write manifest alongside it for quick metadata access
    std::fs::write(store_dir.join(format!("{hash_hex}.toml")), manifest_toml)?;

    Ok(())
}

/// Load a dashboard bundle blob from the asset store by content hash.
///
/// Returns `(manifest_toml, bundle_bytes)` or `None` if not found.
pub fn load_dashboard_bundle(
    data_dir: &Path,
    content_hash: &[u8; 32],
) -> Option<(String, Vec<u8>)> {
    let store_dir = data_dir.join("assets").join("dashboards");
    let hash_hex = hex::encode(content_hash);

    let bundle = std::fs::read(store_dir.join(format!("{hash_hex}.bundle"))).ok()?;
    let manifest = std::fs::read_to_string(store_dir.join(format!("{hash_hex}.toml"))).ok()?;

    Some((manifest, bundle))
}

/// Decode a binary bundle back into a file map.
///
/// Inverse of [`bundle_files`]. Returns `None` on malformed data.
pub fn unbundle_files(bundle: &[u8]) -> Option<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let mut cursor = bundle;

    while cursor.len() >= 4 {
        let path_len = u32::from_le_bytes(cursor[..4].try_into().ok()?) as usize;
        cursor = &cursor[4..];
        if cursor.len() < path_len {
            return None;
        }
        let path = std::str::from_utf8(&cursor[..path_len]).ok()?.to_string();
        cursor = &cursor[path_len..];

        if cursor.len() < 4 {
            return None;
        }
        let content_len = u32::from_le_bytes(cursor[..4].try_into().ok()?) as usize;
        cursor = &cursor[4..];
        if cursor.len() < content_len {
            return None;
        }
        let content = cursor[..content_len].to_vec();
        cursor = &cursor[content_len..];

        files.insert(path, content);
    }

    Some(files)
}

/// Scan the blockchain for all Dashboard-type asset registrations.
///
/// Returns `Vec<(content_hash, block_index)>` ordered by block index
/// (most recent last).
pub fn find_dashboard_assets(
    blocks: &[crate::blockchain::block::Block],
) -> Vec<([u8; 32], u64)> {
    use crate::assets::core::{AssetCategory, BaseSystemType};

    let mut results = Vec::new();
    for block in blocks {
        for asset in block.get_assets() {
            if asset.category == AssetCategory::BaseSystem(BaseSystemType::Dashboard) {
                results.push((asset.content_hash, block.index));
            }
        }
    }
    results
}

/// Find the most recently registered dashboard asset (by block index).
///
/// Returns `(content_hash, block_index)` or `None`.
pub fn find_active_dashboard(
    blocks: &[crate::blockchain::block::Block],
) -> Option<([u8; 32], u64)> {
    find_dashboard_assets(blocks)
        .last()
        .map(|(hash, idx)| (*hash, *idx))
}

// --- Legacy filesystem helpers (retained for migration) ---

/// Save dashboard files to `<data_dir>/dashboards/<name>/`.
///
/// **Deprecated**: prefer `store_dashboard_bundle` (asset-store based).
pub fn persist_dashboard(
    data_dir: &Path,
    name: &str,
    manifest_toml: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    let dash_dir = data_dir.join("dashboards").join(name);
    std::fs::create_dir_all(&dash_dir)?;

    // Save manifest
    std::fs::write(dash_dir.join("dashboard.toml"), manifest_toml)?;

    // Save each file
    for (rel_path, content) in files {
        let target = dash_dir.join(rel_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, content)?;
    }
    Ok(())
}

/// List dashboards stored under `<data_dir>/dashboards/`.
///
/// **Deprecated**: prefer scanning the blockchain via `find_dashboard_assets`.
pub fn list_dashboards(data_dir: &Path) -> Vec<String> {
    let dir = data_dir.join("dashboards");
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().join("dashboard.toml").exists() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// Load a dashboard manifest from `<data_dir>/dashboards/<name>/dashboard.toml`.
///
/// **Deprecated**: prefer `load_dashboard_bundle`.
pub fn load_dashboard_manifest(
    data_dir: &Path,
    name: &str,
) -> Option<super::DashboardManifest> {
    let path = data_dir.join("dashboards").join(name).join("dashboard.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    super::parse_manifest(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_collect_dir_files_empty() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let files = collect_dir_files(&tmp.path().join("nonexistent"))
            .expect("test: collect");
        assert!(files.is_empty());
    }

    #[test]
    fn test_collect_dir_files_with_content() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).expect("test: mkdir");
        std::fs::write(tmp.path().join("a.txt"), "hello").expect("test: write");
        std::fs::write(sub.join("b.txt"), "world").expect("test: write");

        let files = collect_dir_files(tmp.path()).expect("test: collect");
        assert_eq!(files.len(), 2);
        assert_eq!(files["a.txt"], b"hello");
        assert_eq!(files["sub/b.txt"], b"world");
    }

    #[test]
    fn test_bundle_roundtrip() {
        let mut files = BTreeMap::new();
        files.insert("index.html".into(), b"<h1>hi</h1>".to_vec());
        files.insert("style.css".into(), b"body{}".to_vec());

        let bundle = bundle_files(&files);
        assert!(!bundle.is_empty());

        // Verify we can parse it back
        let mut cursor = &bundle[..];
        let mut parsed = BTreeMap::new();
        while cursor.len() >= 4 {
            let path_len = u32::from_le_bytes(cursor[..4].try_into().expect("test: u32")) as usize;
            cursor = &cursor[4..];
            let path = std::str::from_utf8(&cursor[..path_len])
                .expect("test: utf8")
                .to_string();
            cursor = &cursor[path_len..];
            let content_len =
                u32::from_le_bytes(cursor[..4].try_into().expect("test: u32")) as usize;
            cursor = &cursor[4..];
            let content = cursor[..content_len].to_vec();
            cursor = &cursor[content_len..];
            parsed.insert(path, content);
        }
        assert_eq!(files, parsed);
    }

    #[test]
    fn test_blake3_hash_deterministic() {
        let a = blake3_hash(b"hello");
        let b = blake3_hash(b"hello");
        assert_eq!(a, b);
        let c = blake3_hash(b"world");
        assert_ne!(a, c);
    }

    #[test]
    fn test_persist_and_list() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let mut files = BTreeMap::new();
        files.insert("public/index.html".into(), b"<h1>hi</h1>".to_vec());

        let toml = r#"[dashboard]
name = "test"
version = "0.1.0"
description = "Test"
domain = "test.hypermesh"

[access]
public = "dist/public/"
"#;

        persist_dashboard(tmp.path(), "test", toml, &files).expect("test: persist");

        let names = list_dashboards(tmp.path());
        assert_eq!(names, vec!["test"]);

        let manifest = load_dashboard_manifest(tmp.path(), "test");
        assert!(manifest.is_some());
        assert_eq!(manifest.expect("test: manifest").dashboard.name, "test");
    }

    #[test]
    fn test_load_dashboard_manifest_missing() {
        let tmp = TempDir::new().expect("test: tmpdir");
        assert!(load_dashboard_manifest(tmp.path(), "nope").is_none());
    }

    #[test]
    fn test_unbundle_roundtrip() {
        let mut files = BTreeMap::new();
        files.insert("index.html".into(), b"<h1>hello</h1>".to_vec());
        files.insert("assets/app.js".into(), b"console.log('hi')".to_vec());

        let bundle = bundle_files(&files);
        let unbundled = unbundle_files(&bundle).expect("test: unbundle");
        assert_eq!(files, unbundled);
    }

    #[test]
    fn test_unbundle_empty() {
        let files = BTreeMap::new();
        let bundle = bundle_files(&files);
        let unbundled = unbundle_files(&bundle).expect("test: unbundle empty");
        assert!(unbundled.is_empty());
    }

    #[test]
    fn test_store_and_load_dashboard_bundle() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let mut files = BTreeMap::new();
        files.insert("public/index.html".into(), b"<h1>hi</h1>".to_vec());
        let bundle = bundle_files(&files);
        let hash = blake3_hash(&bundle);
        let manifest = "[dashboard]\nname = \"test\"\nversion = \"1.0.0\"\n";

        store_dashboard_bundle(tmp.path(), &hash, manifest, &bundle)
            .expect("test: store bundle");

        let (loaded_manifest, loaded_bundle) =
            load_dashboard_bundle(tmp.path(), &hash).expect("test: load bundle");
        assert_eq!(loaded_manifest, manifest);
        assert_eq!(loaded_bundle, bundle);

        // Unbundle and verify files match
        let unbundled = unbundle_files(&loaded_bundle).expect("test: unbundle");
        assert_eq!(files, unbundled);
    }

    #[test]
    fn test_load_dashboard_bundle_missing() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let hash = [0u8; 32];
        assert!(load_dashboard_bundle(tmp.path(), &hash).is_none());
    }
}

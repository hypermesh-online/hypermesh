// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Scope-aware dashboard server for the gateway.
//!
//! Serves dashboard content with different directories based on authentication
//! level: anonymous visitors see `public/`, authenticated users see `private/`,
//! and the node owner sees `admin/`. Includes SPA fallback routing and
//! scope-based fallback (admin -> private -> public).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Scope determined from the gateway's authentication result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DashboardScope {
    /// Unauthenticated / anonymous visitors.
    Public,
    /// Authenticated but non-owner users.
    Private,
    /// The asset / node owner.
    Owner,
}

impl DashboardScope {
    /// Directory name used as key in `DashboardCache.scopes`.
    #[allow(dead_code)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Owner => "owner",
        }
    }

    /// Ordered fallback chain for the given scope.
    fn fallback_chain(self) -> &'static [&'static str] {
        match self {
            Self::Owner => &["owner", "private", "public"],
            Self::Private => &["private", "public"],
            Self::Public => &["public"],
        }
    }
}

/// A single cached file.
#[derive(Debug, Clone)]
pub struct CachedFile {
    pub content: Vec<u8>,
    pub content_type: String,
}

/// Cache entry for a single domain's dashboard content.
#[derive(Debug, Clone)]
pub struct DashboardCache {
    /// Files per scope directory: `"public"` / `"private"` / `"owner"` -> path -> file.
    pub scopes: HashMap<String, HashMap<String, CachedFile>>,
    /// Identity string that maps to [`DashboardScope::Owner`].
    pub owner_identity: String,
    /// Timestamp when this cache entry was loaded.
    pub loaded_at: Instant,
}

/// Response returned by [`DashboardServer::serve`].
#[derive(Debug, Clone)]
pub struct ServedFile {
    pub content: Vec<u8>,
    pub content_type: String,
}

/// Point-in-time snapshot of dashboard serving statistics.
#[derive(Debug, Clone, Default)]
pub struct DashboardStatsSnapshot {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub scope_public: u64,
    pub scope_private: u64,
    pub scope_owner: u64,
    pub not_found: u64,
}

/// Atomic counters backing [`DashboardStatsSnapshot`].
#[derive(Debug, Default)]
pub struct DashboardServerStats {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub scope_public: AtomicU64,
    pub scope_private: AtomicU64,
    pub scope_owner: AtomicU64,
    pub not_found: AtomicU64,
}

// ---------------------------------------------------------------------------
// DashboardServer
// ---------------------------------------------------------------------------

/// Serves dashboard content with scope-aware routing.
///
/// Each registered domain maps to a [`DashboardCache`] containing files
/// organised by scope directory. The server resolves files using the
/// requesting user's [`DashboardScope`] and falls back through less-
/// privileged scopes when a file is not found.
pub struct DashboardServer {
    cache: Arc<RwLock<HashMap<String, DashboardCache>>>,
    cache_ttl: Duration,
    stats: Arc<DashboardServerStats>,
}

impl DashboardServer {
    /// Create a new dashboard server with the given cache TTL.
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            stats: Arc::new(DashboardServerStats::default()),
        }
    }

    /// Register (or replace) a dashboard for `domain`.
    pub async fn register_dashboard(&self, domain: &str, entry: DashboardCache) {
        let mut cache = self.cache.write().await;
        cache.insert(domain.to_string(), entry);
    }

    /// Load default dashboard HTML for a domain.
    ///
    /// Accepts raw HTML strings for each scope and registers them as the
    /// `index.html` for the corresponding scope directory. This avoids a
    /// dependency on blockmatrix — the caller passes the HTML constants.
    pub async fn load_defaults(
        &self,
        domain: &str,
        owner_identity: &str,
        public_html: &str,
        private_html: &str,
        admin_html: &str,
    ) {
        let mut scopes: HashMap<String, HashMap<String, CachedFile>> = HashMap::new();

        for (scope_dir, html) in [
            ("public", public_html),
            ("private", private_html),
            ("admin", admin_html),
        ] {
            if html.is_empty() {
                continue;
            }
            let mut files = HashMap::new();
            files.insert(
                "index.html".to_string(),
                CachedFile {
                    content: html.as_bytes().to_vec(),
                    content_type: "text/html; charset=utf-8".to_string(),
                },
            );
            scopes.insert(scope_dir.to_string(), files);
        }

        let entry = DashboardCache {
            scopes,
            owner_identity: owner_identity.to_string(),
            loaded_at: Instant::now(),
        };
        self.register_dashboard(domain, entry).await;
        debug!(domain, "loaded default dashboard HTML");
    }

    /// Load dashboard files from a directory on disk.
    ///
    /// Expects the directory to contain `public/`, `private/`, and/or `admin/`
    /// subdirectories. All files within those subdirectories are read and
    /// cached with content-types detected from their extensions.
    ///
    /// Returns `Ok(count)` with the total number of files loaded, or an error
    /// if the base directory cannot be read.
    pub async fn load_from_directory(
        &self,
        domain: &str,
        owner_identity: &str,
        base_dir: &Path,
    ) -> Result<usize, std::io::Error> {
        let mut scopes: HashMap<String, HashMap<String, CachedFile>> = HashMap::new();
        let mut total = 0usize;

        for scope_dir in &["public", "private", "admin"] {
            let scope_path = base_dir.join(scope_dir);
            if !scope_path.is_dir() {
                continue;
            }

            let mut files = HashMap::new();
            Self::read_dir_recursive(&scope_path, &scope_path, &mut files)?;
            total += files.len();
            if !files.is_empty() {
                debug!(
                    domain,
                    scope = *scope_dir,
                    count = files.len(),
                    "loaded dashboard files from disk",
                );
                scopes.insert((*scope_dir).to_string(), files);
            }
        }

        let entry = DashboardCache {
            scopes,
            owner_identity: owner_identity.to_string(),
            loaded_at: Instant::now(),
        };
        self.register_dashboard(domain, entry).await;
        debug!(domain, total, "loaded dashboard from directory");
        Ok(total)
    }

    /// Serve a file for the given `domain`, `path`, and `scope`.
    ///
    /// Returns `None` on cache miss, expiry, path traversal attempt, or when
    /// no matching file exists in any fallback scope.
    pub async fn serve(
        &self,
        domain: &str,
        path: &str,
        scope: DashboardScope,
    ) -> Option<ServedFile> {
        // Path traversal prevention.
        if path.contains("..") {
            self.stats.not_found.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        let cache = self.cache.read().await;
        let entry = match cache.get(domain) {
            Some(e) => e,
            None => {
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        // Check TTL.
        if entry.loaded_at.elapsed() > self.cache_ttl {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
        self.increment_scope_stat(scope);

        // Normalise path: strip leading slash, default to empty string.
        let normalised = path.strip_prefix('/').unwrap_or(path);

        // Walk the fallback chain looking for the exact file, then index.html.
        for scope_dir in scope.fallback_chain() {
            if let Some(files) = entry.scopes.get(*scope_dir) {
                if let Some(file) = files.get(normalised) {
                    return Some(ServedFile {
                        content: file.content.clone(),
                        content_type: file.content_type.clone(),
                    });
                }
            }
        }

        // SPA fallback: try index.html in the fallback chain.
        for scope_dir in scope.fallback_chain() {
            if let Some(files) = entry.scopes.get(*scope_dir) {
                if let Some(file) = files.get("index.html") {
                    return Some(ServedFile {
                        content: file.content.clone(),
                        content_type: file.content_type.clone(),
                    });
                }
            }
        }

        self.stats.not_found.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Return the owner identity of a registered domain.
    pub async fn owner_identity(&self, domain: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache.get(domain).map(|e| e.owner_identity.clone())
    }

    /// Best-effort cache invalidation for `domain`.
    ///
    /// Spawns a background task so the caller is never blocked.
    pub fn invalidate(&self, domain: &str) {
        let cache = Arc::clone(&self.cache);
        let key = domain.to_string();
        tokio::spawn(async move {
            let mut map = cache.write().await;
            map.remove(&key);
        });
    }

    /// Return a point-in-time snapshot of serving statistics.
    pub fn stats(&self) -> DashboardStatsSnapshot {
        DashboardStatsSnapshot {
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            scope_public: self.stats.scope_public.load(Ordering::Relaxed),
            scope_private: self.stats.scope_private.load(Ordering::Relaxed),
            scope_owner: self.stats.scope_owner.load(Ordering::Relaxed),
            not_found: self.stats.not_found.load(Ordering::Relaxed),
        }
    }

    // -- internal helpers --------------------------------------------------

    /// Recursively read all files under `dir`, storing them relative to
    /// `base` (the scope root). Skips unreadable files with a warning.
    fn read_dir_recursive(
        base: &Path,
        dir: &Path,
        out: &mut HashMap<String, CachedFile>,
    ) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::read_dir_recursive(base, &path, out)?;
                continue;
            }

            let rel = match path.strip_prefix(base) {
                Ok(r) => r,
                Err(_) => continue,
            };

            // Normalise to forward-slash relative path.
            let key = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");

            match std::fs::read(&path) {
                Ok(content) => {
                    let content_type =
                        detect_content_type(&key).to_string();
                    out.insert(key, CachedFile { content, content_type });
                }
                Err(e) => {
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "skipping unreadable dashboard file",
                    );
                }
            }
        }
        Ok(())
    }

    fn increment_scope_stat(&self, scope: DashboardScope) {
        match scope {
            DashboardScope::Public => self.stats.scope_public.fetch_add(1, Ordering::Relaxed),
            DashboardScope::Private => self.stats.scope_private.fetch_add(1, Ordering::Relaxed),
            DashboardScope::Owner => self.stats.scope_owner.fetch_add(1, Ordering::Relaxed),
        };
    }
}

// ---------------------------------------------------------------------------
// Scope determination
// ---------------------------------------------------------------------------

/// Determine the dashboard scope from an optional authenticated identity and
/// the node owner's identity string.
///
/// Phase 0 bootstrap pass-through: no authentication subsystem is wired into
/// the gateway request pipeline. Requests without an identity fall back to
/// the Public scope. When a PoS-validated identity is plumbed through,
/// pass `Some(identity)` to unlock Private/Owner.
pub fn determine_scope(identity: Option<&str>, owner_identity: &str) -> DashboardScope {
    match identity {
        None => DashboardScope::Public,
        Some(id) if id == owner_identity => DashboardScope::Owner,
        Some(_) => DashboardScope::Private,
    }
}

// ---------------------------------------------------------------------------
// Content-type detection
// ---------------------------------------------------------------------------

/// Detect a MIME content-type from the file extension in `path`.
pub fn detect_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "application/javascript",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers -----------------------------------------------------------

    /// Build a minimal `DashboardCache` with the given scope->file mappings.
    fn make_cache(
        owner: &str,
        entries: &[(&str, &str, &[u8])], // (scope_dir, path, content)
    ) -> DashboardCache {
        let mut scopes: HashMap<String, HashMap<String, CachedFile>> = HashMap::new();
        for &(scope_dir, path, content) in entries {
            let ct = detect_content_type(path).to_string();
            scopes
                .entry(scope_dir.to_string())
                .or_default()
                .insert(
                    path.to_string(),
                    CachedFile {
                        content: content.to_vec(),
                        content_type: ct,
                    },
                );
        }
        DashboardCache {
            scopes,
            owner_identity: owner.to_string(),
            loaded_at: Instant::now(),
        }
    }

    // ===== Scope determination (3 tests) ==================================

    #[test]
    fn scope_none_identity_is_public() {
        let scope = determine_scope(None, "owner");
        assert_eq!(scope, DashboardScope::Public);
    }

    #[test]
    fn scope_non_owner_identity_is_private() {
        let scope = determine_scope(Some("alice"), "bob");
        assert_eq!(scope, DashboardScope::Private);
    }

    #[test]
    fn scope_owner_identity_is_owner() {
        let scope = determine_scope(Some("owner-node"), "owner-node");
        assert_eq!(scope, DashboardScope::Owner);
    }

    // ===== Content-type detection (8 tests) ===============================

    #[test]
    fn content_type_html() {
        assert_eq!(detect_content_type("index.html"), "text/html; charset=utf-8");
    }

    #[test]
    fn content_type_js() {
        assert_eq!(detect_content_type("app.js"), "application/javascript");
        assert_eq!(detect_content_type("module.mjs"), "application/javascript");
    }

    #[test]
    fn content_type_css() {
        assert_eq!(detect_content_type("style.css"), "text/css; charset=utf-8");
    }

    #[test]
    fn content_type_json() {
        assert_eq!(detect_content_type("data.json"), "application/json");
    }

    #[test]
    fn content_type_png() {
        assert_eq!(detect_content_type("logo.png"), "image/png");
    }

    #[test]
    fn content_type_svg() {
        assert_eq!(detect_content_type("icon.svg"), "image/svg+xml");
    }

    #[test]
    fn content_type_wasm() {
        assert_eq!(detect_content_type("module.wasm"), "application/wasm");
    }

    #[test]
    fn content_type_unknown() {
        assert_eq!(detect_content_type("archive.tar"), "application/octet-stream");
        assert_eq!(detect_content_type("no-extension"), "application/octet-stream");
    }

    // ===== Serve behaviour (7 tests) ======================================

    #[tokio::test]
    async fn serve_returns_file_from_correct_scope() {
        let server = DashboardServer::new(Duration::from_secs(300));
        let cache = make_cache("owner", &[
            ("public", "index.html", b"<h1>public</h1>"),
            ("private", "index.html", b"<h1>private</h1>"),
        ]);
        server.register_dashboard("example.com", cache).await;

        let result = server
            .serve("example.com", "/index.html", DashboardScope::Private)
            .await;
        let file = result.expect("test: should find file");
        assert_eq!(file.content, b"<h1>private</h1>");
    }

    #[tokio::test]
    async fn serve_rejects_path_traversal() {
        let server = DashboardServer::new(Duration::from_secs(300));
        let cache = make_cache("owner", &[("public", "secret.html", b"secret")]);
        server.register_dashboard("example.com", cache).await;

        let result = server
            .serve("example.com", "/../secret.html", DashboardScope::Public)
            .await;
        assert!(result.is_none(), "path traversal must be rejected");
    }

    #[tokio::test]
    async fn serve_owner_falls_back_to_private_then_public() {
        let server = DashboardServer::new(Duration::from_secs(300));
        // Only public scope has the file.
        let cache = make_cache("owner", &[("public", "shared.js", b"var x = 1;")]);
        server.register_dashboard("example.com", cache).await;

        let result = server
            .serve("example.com", "/shared.js", DashboardScope::Owner)
            .await;
        let file = result.expect("test: should fall back to public");
        assert_eq!(file.content, b"var x = 1;");
    }

    #[tokio::test]
    async fn serve_spa_fallback_to_index_html() {
        let server = DashboardServer::new(Duration::from_secs(300));
        let cache = make_cache("owner", &[("public", "index.html", b"<h1>SPA</h1>")]);
        server.register_dashboard("example.com", cache).await;

        let result = server
            .serve("example.com", "/some/deep/route", DashboardScope::Public)
            .await;
        let file = result.expect("test: SPA should serve index.html");
        assert_eq!(file.content, b"<h1>SPA</h1>");
    }

    #[tokio::test]
    async fn serve_unknown_domain_returns_none() {
        let server = DashboardServer::new(Duration::from_secs(300));
        let result = server
            .serve("unknown.com", "/index.html", DashboardScope::Public)
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn serve_expired_cache_returns_none() {
        let server = DashboardServer::new(Duration::from_millis(1));
        let cache = make_cache("owner", &[("public", "index.html", b"old")]);
        server.register_dashboard("example.com", cache).await;

        // Let the TTL expire.
        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = server
            .serve("example.com", "/index.html", DashboardScope::Public)
            .await;
        assert!(result.is_none(), "expired cache should return None");
    }

    #[tokio::test]
    async fn serve_no_file_no_index_returns_none() {
        let server = DashboardServer::new(Duration::from_secs(300));
        // Register a scope with no index.html and no matching file.
        let cache = make_cache("owner", &[("public", "other.css", b"body{}")]);
        server.register_dashboard("example.com", cache).await;

        let result = server
            .serve("example.com", "/missing.js", DashboardScope::Public)
            .await;
        assert!(result.is_none());
    }

    // ===== Stats (2 tests) ================================================

    #[tokio::test]
    async fn stats_cache_hit_increments() {
        let server = DashboardServer::new(Duration::from_secs(300));
        let cache = make_cache("owner", &[("public", "index.html", b"hi")]);
        server.register_dashboard("d.com", cache).await;

        let _ = server.serve("d.com", "/index.html", DashboardScope::Public).await;
        let snap = server.stats();
        assert_eq!(snap.cache_hits, 1);
        assert_eq!(snap.scope_public, 1);
    }

    #[tokio::test]
    async fn stats_not_found_increments() {
        let server = DashboardServer::new(Duration::from_secs(300));
        // No dashboards registered → cache miss.
        let _ = server.serve("none.com", "/x", DashboardScope::Public).await;
        let snap = server.stats();
        assert_eq!(snap.cache_misses, 1);
    }

    // ===== load_defaults (3 tests) ========================================

    #[tokio::test]
    async fn load_defaults_populates_all_scopes() {
        let server = DashboardServer::new(Duration::from_secs(300));
        server
            .load_defaults("d.com", "owner", "<h1>pub</h1>", "<h1>priv</h1>", "<h1>own</h1>")
            .await;

        let pub_file = server
            .serve("d.com", "/index.html", DashboardScope::Public)
            .await
            .expect("test: public index");
        assert_eq!(pub_file.content, b"<h1>pub</h1>");

        let priv_file = server
            .serve("d.com", "/index.html", DashboardScope::Private)
            .await
            .expect("test: private index");
        assert_eq!(priv_file.content, b"<h1>priv</h1>");

        let own_file = server
            .serve("d.com", "/index.html", DashboardScope::Owner)
            .await
            .expect("test: owner index");
        assert_eq!(own_file.content, b"<h1>own</h1>");
    }

    #[tokio::test]
    async fn load_defaults_skips_empty_scopes() {
        let server = DashboardServer::new(Duration::from_secs(300));
        server.load_defaults("d.com", "owner", "<h1>pub</h1>", "", "").await;

        // Private scope has no index.html, so it falls back to public.
        let result = server
            .serve("d.com", "/index.html", DashboardScope::Private)
            .await
            .expect("test: should fall back to public");
        assert_eq!(result.content, b"<h1>pub</h1>");
    }

    #[tokio::test]
    async fn load_defaults_content_type_is_html() {
        let server = DashboardServer::new(Duration::from_secs(300));
        server.load_defaults("d.com", "owner", "<h1>hi</h1>", "", "").await;

        let file = server
            .serve("d.com", "/index.html", DashboardScope::Public)
            .await
            .expect("test: should find file");
        assert_eq!(file.content_type, "text/html; charset=utf-8");
    }

    // ===== load_from_directory (3 tests) ===================================

    #[tokio::test]
    async fn load_from_directory_reads_files() {
        let tmp = std::env::temp_dir().join("gw_test_load_dir");
        let _ = std::fs::remove_dir_all(&tmp);
        let pub_dir = tmp.join("public");
        std::fs::create_dir_all(&pub_dir).expect("test: create dir");
        std::fs::write(pub_dir.join("index.html"), b"<h1>hello</h1>").expect("test: write");
        std::fs::write(pub_dir.join("app.js"), b"console.log(1)").expect("test: write");

        let server = DashboardServer::new(Duration::from_secs(300));
        let count = server
            .load_from_directory("d.com", "owner", &tmp)
            .await
            .expect("test: load dir");
        assert_eq!(count, 2);

        let html = server
            .serve("d.com", "/index.html", DashboardScope::Public)
            .await
            .expect("test: should find index");
        assert_eq!(html.content, b"<h1>hello</h1>");

        let js = server
            .serve("d.com", "/app.js", DashboardScope::Public)
            .await
            .expect("test: should find js");
        assert_eq!(js.content, b"console.log(1)");
        assert_eq!(js.content_type, "application/javascript");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn load_from_directory_reads_nested_subdirs() {
        let tmp = std::env::temp_dir().join("gw_test_load_nested");
        let _ = std::fs::remove_dir_all(&tmp);
        let nested = tmp.join("public").join("assets").join("css");
        std::fs::create_dir_all(&nested).expect("test: create dir");
        std::fs::write(nested.join("style.css"), b"body{}").expect("test: write");

        let server = DashboardServer::new(Duration::from_secs(300));
        let count = server
            .load_from_directory("d.com", "owner", &tmp)
            .await
            .expect("test: load dir");
        assert_eq!(count, 1);

        let css = server
            .serve("d.com", "/assets/css/style.css", DashboardScope::Public)
            .await
            .expect("test: should find nested css");
        assert_eq!(css.content, b"body{}");
        assert_eq!(css.content_type, "text/css; charset=utf-8");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn load_from_directory_skips_missing_scope_dirs() {
        let tmp = std::env::temp_dir().join("gw_test_load_skip");
        let _ = std::fs::remove_dir_all(&tmp);
        // Only create owner dir — no public or private.
        let owner_dir = tmp.join("owner");
        std::fs::create_dir_all(&owner_dir).expect("test: create dir");
        std::fs::write(owner_dir.join("index.html"), b"<h1>owner</h1>").expect("test: write");

        let server = DashboardServer::new(Duration::from_secs(300));
        let count = server
            .load_from_directory("d.com", "owner", &tmp)
            .await
            .expect("test: load dir");
        assert_eq!(count, 1);

        // Public scope has nothing — should return None.
        let result = server
            .serve("d.com", "/index.html", DashboardScope::Public)
            .await;
        assert!(result.is_none());

        // Owner scope has the file.
        let own = server
            .serve("d.com", "/index.html", DashboardScope::Owner)
            .await
            .expect("test: owner index");
        assert_eq!(own.content, b"<h1>owner</h1>");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ===== DashboardScope helpers (2 tests) ===============================

    #[test]
    fn scope_as_str_values() {
        assert_eq!(DashboardScope::Public.as_str(), "public");
        assert_eq!(DashboardScope::Private.as_str(), "private");
        assert_eq!(DashboardScope::Owner.as_str(), "owner");
    }

    #[test]
    fn scope_fallback_chain_ordering() {
        assert_eq!(
            DashboardScope::Owner.fallback_chain(),
            &["owner", "private", "public"]
        );
        assert_eq!(
            DashboardScope::Private.fallback_chain(),
            &["private", "public"]
        );
        assert_eq!(DashboardScope::Public.fallback_chain(), &["public"]);
    }
}

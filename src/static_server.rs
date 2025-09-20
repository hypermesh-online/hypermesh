//! Static File Server for HyperMesh UI
//!
//! Serves the built frontend assets for the HyperMesh dashboard.
//! Falls back to index.html for client-side routing (SPA).

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn, debug};

/// Configuration for static file serving
#[derive(Debug, Clone)]
pub struct StaticServerConfig {
    /// Base path where static files are located
    pub static_dir: PathBuf,

    /// Whether to serve index.html as fallback for SPA routing
    pub spa_fallback: bool,

    /// Cache control header value
    pub cache_control: String,
}

impl Default for StaticServerConfig {
    fn default() -> Self {
        Self {
            static_dir: PathBuf::from("ui/frontend/dist"),
            spa_fallback: true,
            cache_control: "public, max-age=3600".to_string(),
        }
    }
}

/// Static file server for serving UI assets
pub struct StaticFileServer {
    config: Arc<StaticServerConfig>,
}

impl StaticFileServer {
    /// Create new static file server
    pub fn new(config: StaticServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Serve a static file based on the request path
    pub async fn serve(&self, path: &str) -> Result<StaticFileResponse> {
        // Clean and validate the path
        let clean_path = self.clean_path(path);

        // Determine the file path to serve
        let file_path = if clean_path.is_empty() || clean_path == "/" {
            // Serve index.html for root
            self.config.static_dir.join("index.html")
        } else {
            // Try to serve the requested file
            let requested_path = self.config.static_dir.join(&clean_path);

            // Check if file exists
            if requested_path.exists() && requested_path.is_file() {
                requested_path
            } else if self.config.spa_fallback {
                // For SPA routing, serve index.html for non-existent paths
                self.config.static_dir.join("index.html")
            } else {
                // File not found
                return Ok(StaticFileResponse::not_found());
            }
        };

        // Read and serve the file
        self.serve_file(&file_path).await
    }

    /// Clean and sanitize the request path
    fn clean_path(&self, path: &str) -> String {
        // Remove leading slash
        let path = path.trim_start_matches('/');

        // Remove query parameters
        let path = path.split('?').next().unwrap_or("");

        // Remove dangerous path components
        path.replace("..", "")
            .replace("//", "/")
            .trim()
            .to_string()
    }

    /// Serve a specific file
    async fn serve_file(&self, file_path: &Path) -> Result<StaticFileResponse> {
        // Check if file exists
        if !file_path.exists() {
            return Ok(StaticFileResponse::not_found());
        }

        // Read file contents
        let content = fs::read(file_path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        // Determine content type
        let content_type = self.get_content_type(file_path);

        // Create response
        Ok(StaticFileResponse {
            status: 200,
            content,
            content_type,
            cache_control: self.config.cache_control.clone(),
        })
    }

    /// Determine content type based on file extension
    fn get_content_type(&self, file_path: &Path) -> String {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "html" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "wasm" => "application/wasm",
            _ => "application/octet-stream",
        }.to_string()
    }

    /// Check if static files directory exists and is valid
    pub async fn validate(&self) -> Result<()> {
        if !self.config.static_dir.exists() {
            warn!("Static files directory does not exist: {:?}", self.config.static_dir);
            warn!("Run 'cd ui/frontend && npm run build' to create it");
            return Ok(()); // Don't fail, just warn
        }

        let index_path = self.config.static_dir.join("index.html");
        if !index_path.exists() {
            warn!("index.html not found in static directory");
            warn!("Run 'cd ui/frontend && npm run build' to create it");
        }

        info!("Static file server configured:");
        info!("  Directory: {:?}", self.config.static_dir);
        info!("  SPA Fallback: {}", self.config.spa_fallback);

        Ok(())
    }
}

/// Response from static file server
#[derive(Debug)]
pub struct StaticFileResponse {
    pub status: u16,
    pub content: Vec<u8>,
    pub content_type: String,
    pub cache_control: String,
}

impl StaticFileResponse {
    /// Create a 404 not found response
    pub fn not_found() -> Self {
        Self {
            status: 404,
            content: b"404 - Not Found".to_vec(),
            content_type: "text/plain".to_string(),
            cache_control: "no-cache".to_string(),
        }
    }
}

/// Integration with STOQ transport layer for HTTP compatibility
pub mod http_compat {
    use super::*;
    use std::collections::HashMap;

    /// Convert HTTP-like request to static file response
    pub async fn handle_http_request(
        server: &StaticFileServer,
        method: &str,
        path: &str,
        _headers: &HashMap<String, String>,
    ) -> Result<StaticFileResponse> {
        // Only handle GET requests for static files
        if method != "GET" {
            return Ok(StaticFileResponse {
                status: 405,
                content: b"Method Not Allowed".to_vec(),
                content_type: "text/plain".to_string(),
                cache_control: "no-cache".to_string(),
            });
        }

        // Serve the static file
        server.serve(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_static_file_server() {
        // Create temp directory with test files
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("index.html");
        let css_path = temp_dir.path().join("style.css");

        fs::write(&index_path, b"<html>Test</html>").await.unwrap();
        fs::write(&css_path, b"body { color: red; }").await.unwrap();

        // Create server
        let config = StaticServerConfig {
            static_dir: temp_dir.path().to_path_buf(),
            spa_fallback: true,
            cache_control: "public, max-age=3600".to_string(),
        };
        let server = StaticFileServer::new(config);

        // Test serving index.html
        let response = server.serve("/").await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content, b"<html>Test</html>");
        assert_eq!(response.content_type, "text/html; charset=utf-8");

        // Test serving CSS file
        let response = server.serve("/style.css").await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content, b"body { color: red; }");
        assert_eq!(response.content_type, "text/css; charset=utf-8");

        // Test SPA fallback
        let response = server.serve("/non-existent-route").await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content, b"<html>Test</html>"); // Falls back to index.html
    }

    #[tokio::test]
    async fn test_content_type_detection() {
        let config = StaticServerConfig::default();
        let server = StaticFileServer::new(config);

        assert_eq!(
            server.get_content_type(Path::new("test.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            server.get_content_type(Path::new("test.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            server.get_content_type(Path::new("test.wasm")),
            "application/wasm"
        );
        assert_eq!(
            server.get_content_type(Path::new("test.unknown")),
            "application/octet-stream"
        );
    }
}
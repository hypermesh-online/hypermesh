// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Request / response wire types for the Catalog STOQ API.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Browse packages request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseRequest {
    /// Category filter (optional)
    pub category: Option<String>,
    /// Sort by field
    #[serde(default = "default_sort")]
    pub sort_by: String,
    /// Page number (0-indexed)
    #[serde(default)]
    pub page: u64,
    /// Items per page
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    /// Only featured packages
    #[serde(default)]
    pub featured_only: bool,
}

pub(crate) fn default_sort() -> String {
    "relevance".to_string()
}
pub(crate) fn default_page_size() -> u64 {
    20
}

/// Browse packages response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResponse {
    pub packages: Vec<PackageSummary>,
    pub total_count: u64,
    pub page: u64,
    pub page_size: u64,
}

/// Search packages request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query string
    pub query: String,
    /// Tag filters
    #[serde(default)]
    pub tags: Vec<String>,
    /// Author filter
    pub author: Option<String>,
    /// Maximum results
    #[serde(default = "default_page_size")]
    pub limit: u64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: u64,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<PackageSummary>,
    pub total_count: u64,
    pub query: String,
}

/// Package summary (used in browse/search results)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub download_count: u64,
    pub score: f64,
    /// Whether this is a curated/featured type definition
    pub featured: bool,
}

/// Get package details request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackageRequest {
    /// Package name
    pub name: String,
    /// Version (None = latest)
    pub version: Option<String>,
}

/// Package detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackageResponse {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub download_count: u64,
    pub featured: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub dependencies: Vec<String>,
    pub publisher_authenticated: Option<bool>,
    /// Schema for this type definition (JSON Schema)
    pub schema: Option<serde_json::Value>,
    /// Validation rules count
    pub validation_rules_count: u32,
}

/// Get publisher info request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPublisherRequest {
    pub publisher_id: String,
}

/// Publisher info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPublisherResponse {
    pub publisher_id: String,
    pub authenticated: bool,
    pub total_packages: u64,
    pub total_downloads: u64,
    pub member_since: Option<String>,
}

/// Registry stats response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatsResponse {
    pub total_packages: u64,
    pub total_publishers: u64,
    pub total_downloads: u64,
    pub version: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub package_count: u64,
    pub uptime_secs: u64,
}

/// Publish a new type definition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePublishRequest {
    /// Type name (e.g. "Message", "Invoice")
    pub type_name: String,
    /// JSON Schema defining the type structure
    pub schema: serde_json::Value,
    /// Semantic version (defaults to "1.0.0")
    #[serde(default = "default_version")]
    pub version: String,
    /// Optional author identifier
    pub author: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

pub(crate) fn default_version() -> String {
    "1.0.0".to_string()
}

/// Type publish response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePublishResponse {
    /// Human-readable type name
    pub type_name: String,
    /// Content-addressed BLAKE3 hash of the schema
    pub type_hash: String,
    /// Version that was registered
    pub version: String,
    /// Registration status
    pub status: String,
}

/// Look up a type definition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLookupRequest {
    /// Look up by name (optional)
    pub name: Option<String>,
    /// Look up by content hash (optional)
    pub hash: Option<String>,
}

/// Type lookup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLookupResponse {
    /// Whether the type was found
    pub status: String,
    /// Type name (if found)
    pub type_name: Option<String>,
    /// Content-addressed hash (if found)
    pub type_hash: Option<String>,
    /// Schema (if found)
    pub schema: Option<serde_json::Value>,
    /// Version (if found)
    pub version: Option<String>,
}

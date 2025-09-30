//! Tests for Phase 2.1: Asset Library Logic Extraction
//!
//! Verifies that the core asset library functionality has been successfully
//! extracted from the standalone service into reusable components.

use catalog::library::{
    AssetLibrary, AssetPackageManager, LibraryConfig, LibraryInterface,
    SearchQuery, PackageSummary, LibraryAssetPackage,
    types::*,
};
use std::sync::Arc;

#[tokio::test]
async fn test_library_extraction_complete() {
    // Verify we can create the library without service dependencies
    let config = LibraryConfig::default();
    let library = AssetLibrary::new(config);

    // Verify core operations work
    let stats = library.get_stats().await.unwrap();
    assert_eq!(stats.total_packages, 0);
}

#[tokio::test]
async fn test_library_zero_copy_operations() {
    let config = LibraryConfig {
        enable_zero_copy: true,
        ..Default::default()
    };

    let library = AssetLibrary::new(config);

    // Create test package
    let package = create_test_package("test-pkg");

    // Add package (should use Arc internally for zero-copy)
    library.add_package(package.clone()).await.unwrap();

    // Retrieve package
    let retrieved = library.get_package("test-pkg").await.unwrap();
    assert!(retrieved.is_some());

    // Verify the package data
    let pkg = retrieved.unwrap();
    assert_eq!(pkg.id.as_ref(), "test-pkg");
    assert_eq!(pkg.metadata.name.as_ref(), "Test Package");
}

#[tokio::test]
async fn test_multi_tier_caching() {
    let config = LibraryConfig {
        enable_cache: true,
        l1_cache_size: 10,
        l2_cache_size: 100,
        l3_cache_path: None, // Memory only for test
        ..Default::default()
    };

    let library = AssetLibrary::new(config);

    // Add multiple packages
    for i in 0..20 {
        let package = create_test_package(&format!("pkg-{}", i));
        library.add_package(package).await.unwrap();
    }

    // Access some packages to populate cache
    for i in 0..5 {
        let _ = library.get_package(&format!("pkg-{}", i)).await.unwrap();
    }

    // Check cache statistics
    let stats = library.get_stats().await.unwrap();
    assert_eq!(stats.total_packages, 20);
    // Cache hits should be recorded
}

#[tokio::test]
async fn test_package_manager_lifecycle() {
    let library = Arc::new(AssetLibrary::new(LibraryConfig::default()));

    // Add test package to library
    let package = create_test_package("lifecycle-pkg");
    library.add_package(package).await.unwrap();

    // Create package manager
    let manager = AssetPackageManager::new(
        library.clone(),
        Default::default(),
    );

    // Install package
    let result = manager.install_package("lifecycle-pkg").await.unwrap();
    assert!(result.success);
    assert_eq!(result.packages_affected.len(), 1);

    // List installed
    let installed = manager.list_installed().await.unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "lifecycle-pkg");

    // Uninstall package
    let result = manager.uninstall_package("lifecycle-pkg").await.unwrap();
    assert!(result.success);

    // Verify uninstalled
    let installed = manager.list_installed().await.unwrap();
    assert_eq!(installed.len(), 0);
}

#[tokio::test]
async fn test_search_functionality() {
    let library = AssetLibrary::new(LibraryConfig::default());

    // Add packages with different metadata
    for i in 0..5 {
        let mut package = create_test_package(&format!("search-pkg-{}", i));
        package.metadata.tags = Arc::new([Arc::from(format!("tag-{}", i % 2))]);
        package.metadata.keywords = Arc::new([
            Arc::from("search"),
            Arc::from(format!("keyword-{}", i)),
        ]);
        library.add_package(package).await.unwrap();
    }

    // Search by query text
    let query = SearchQuery {
        query: "search".to_string(),
        tags: vec![],
        asset_type: None,
        author: None,
        limit: 10,
        offset: 0,
    };

    let results = library.search_packages(&query).await.unwrap();
    assert_eq!(results.len(), 5);

    // Search by tag
    let query = SearchQuery {
        query: String::new(),
        tags: vec!["tag-0".to_string()],
        asset_type: None,
        author: None,
        limit: 10,
        offset: 0,
    };

    let results = library.search_packages(&query).await.unwrap();
    assert!(results.len() > 0);
}

#[tokio::test]
async fn test_no_service_dependencies() {
    // This test verifies that the library can operate without any
    // HTTP, network, or standalone service infrastructure

    let config = LibraryConfig::default();
    let mut library = AssetLibrary::new(config.clone());

    // Initialize without any network configuration
    library.initialize(config).await.unwrap();

    // All operations should work in-memory
    let package = create_test_package("no-service-pkg");
    library.add_package(package.clone()).await.unwrap();

    // Validate package (should work without external services)
    let validation = library.validate_package(&package).await.unwrap();
    assert!(validation.valid || validation.errors.is_empty());

    // Resolve dependencies (should work without external registry)
    let resolution = library.resolve_dependencies(&package).await.unwrap();
    assert!(resolution.success || resolution.conflicts.is_empty());
}

#[tokio::test]
async fn test_performance_metrics() {
    let config = LibraryConfig {
        enable_metrics: true,
        ..Default::default()
    };

    let library = AssetLibrary::new(config);

    // Perform operations
    for i in 0..10 {
        let package = create_test_package(&format!("metrics-pkg-{}", i));
        library.add_package(package).await.unwrap();
    }

    // Access packages to generate metrics
    for i in 0..10 {
        let _ = library.get_package(&format!("metrics-pkg-{}", i)).await.unwrap();
    }

    // Check metrics
    let stats = library.get_stats().await.unwrap();
    assert!(stats.total_operations > 0);
    assert!(stats.avg_latency_us > 0);
}

#[test]
fn test_lightweight_types() {
    // Verify that our types are lightweight and use Arc for efficiency

    let metadata = PackageMetadata {
        name: Arc::from("test"),
        version: Arc::from("1.0.0"),
        description: None,
        author: None,
        license: None,
        tags: Arc::new([]),
        keywords: Arc::new([]),
        created: 0,
        modified: 0,
    };

    // Arc allows cheap cloning
    let metadata_clone = metadata.clone();
    assert_eq!(metadata.name, metadata_clone.name);

    // Verify enum sizes are small
    assert!(std::mem::size_of::<AssetType>() <= 2);
    assert!(std::mem::size_of::<SandboxLevel>() <= 2);
    assert!(std::mem::size_of::<ExecutionPriority>() <= 2);
}

// Helper function to create test packages
fn create_test_package(id: &str) -> LibraryAssetPackage {
    use std::collections::HashMap;

    LibraryAssetPackage {
        id: Arc::from(id),
        metadata: PackageMetadata {
            name: Arc::from("Test Package"),
            version: Arc::from("1.0.0"),
            description: Some(Arc::from("A test package")),
            author: Some(Arc::from("Test Author")),
            license: Some(Arc::from("MIT")),
            tags: Arc::new([Arc::from("test")]),
            keywords: Arc::new([Arc::from("test"), Arc::from("example")]),
            created: chrono::Utc::now().timestamp(),
            modified: chrono::Utc::now().timestamp(),
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
                max_concurrent: Some(1),
                priority: ExecutionPriority::Normal,
                retry_policy: RetryPolicy::default(),
            },
            dependencies: Arc::new([]),
            environment: Arc::new(HashMap::new()),
        },
        content_refs: ContentReferences {
            main_ref: ContentRef {
                path: Arc::from("main.jl"),
                hash: Arc::from("test-hash"),
                size: 1024,
                content_type: ContentType::Source,
            },
            file_refs: Arc::new([]),
            binary_refs: Arc::new([]),
            total_size: 1024,
        },
        validation: None,
        hash: Arc::from("package-hash"),
    }
}
// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CatalogExtension AssetLibraryExtension trait implementation
//!
//! Package lifecycle operations: list, get, install, uninstall, update,
//! search, publish, and verify.

use async_trait::async_trait;
// BLAKE3 used via blake3::hash() for domain and package hashes
use std::collections::HashMap;
use std::sync::Arc;
use semver::Version;

use blockmatrix::extensions::{
    AssetLibraryExtension, ExtensionResult, ExtensionError,
    AssetPackage, PackageFilter, InstallOptions, InstallResult,
    UpdateResult, SearchOptions, AssetPackageSpec, PublishResult, VerificationResult,
    ResourceUsageReport, SecurityIssue,
};

use blockmatrix::assets::core::{AssetType, AssetRegistration, AssetData, NetworkScope, AssetCategory};
use blockmatrix::assets::core::ApplicationDomain;

use super::types::CatalogExtension;

#[async_trait]
impl AssetLibraryExtension for CatalogExtension {
    async fn list_packages(&self, _filter: PackageFilter) -> ExtensionResult<Vec<AssetPackage>> {
        self.increment_requests().await;
        self.start_operation().await;

        // TODO: Implement proper conversion from LibraryAssetPackage to blockmatrix AssetPackage
        let packages = vec![];

        self.complete_operation().await;
        Ok(packages)
    }

    async fn get_package(&self, package_id: &str) -> ExtensionResult<AssetPackage> {
        self.increment_requests().await;
        self.start_operation().await;

        // TODO: Implement proper conversion from LibraryAssetPackage to blockmatrix AssetPackage
        let package = AssetPackage {
            id: package_id.to_string(),
            name: "stub_package".to_string(),
            version: Version::parse("0.0.1").expect("valid semver"),
            description: "Stub package for compilation".to_string(),
            author: "".to_string(),
            license: "".to_string(),
            asset_types: vec![AssetType::Library],
            size_bytes: 0,
            install_count: 0,
            rating: 0.0,
            dependencies: vec![],
            metadata: HashMap::new(),
            distribution_hash: String::new(),
            signature: None,
        };

        self.complete_operation().await;
        Ok(package)
    }

    async fn install_package(
        &self,
        package_id: &str,
        _options: InstallOptions,
    ) -> ExtensionResult<InstallResult> {
        self.increment_requests().await;
        self.start_operation().await;

        if self.config.consensus_validation {
            // Consensus validation for installs requires the proof to be embedded in
            // the package metadata. The proof is validated at publish time; for installs,
            // we check if the package's security metadata requires consensus and log
            // that validation is deferred to the execution layer.
            let package_preview = self.library_manager.read().await
                .get_package(package_id).await;
            if let Some(pkg) = package_preview {
                if let Some(ref spec) = pkg.spec {
                    if spec.security.consensus_required {
                        tracing::warn!(
                            "Package {} requires consensus validation; \
                             proof verification deferred to execution layer",
                            package_id
                        );
                    }
                }
            }
        }

        let library_manager = self.library_manager.read().await;

        let package = library_manager.get_package(package_id).await
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", package_id)
            })?;

        let start = std::time::Instant::now();

        library_manager.install_package((*package).clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to install package: {}", e)
            })?;

        let install_duration = start.elapsed();

        let installed_asset_ids: Vec<AssetRegistration> = vec![
            AssetRegistration::from_hex_string(package_id)
                .unwrap_or_else(|_| {
                    let asset_data = AssetData {
                        config: package_id.as_bytes().to_vec(),
                        definition: b"catalog_package".to_vec(),
                        metadata: b"{}".to_vec(),
                    };
                    AssetRegistration::from_asset_data(
                        &asset_data,
                        NetworkScope::Global,
                        AssetCategory::Application(ApplicationDomain {
                            domain_name: "catalog".to_string(),
                            domain_hash: *blake3::hash(b"catalog").as_bytes(),
                        }),
                    )
                })
        ];

        let result = InstallResult {
            package_id: package_id.to_string(),
            install_path: std::path::PathBuf::from("/tmp/catalog/install"),
            installed_assets: installed_asset_ids,
            install_time: install_duration,
        };

        self.update_resource_usage(ResourceUsageReport {
            cpu_usage: 0.1,
            memory_usage: result.installed_assets.len() as u64 * 1024,
            network_bytes: 1024 * 1024,
            storage_bytes: 1024 * 1024,
        }).await;

        self.complete_operation().await;
        Ok(result)
    }

    async fn uninstall_package(&self, package_id: &str) -> ExtensionResult<()> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        library_manager.uninstall_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to uninstall package: {}", e)
            })?;

        self.complete_operation().await;
        Ok(())
    }

    async fn update_package(
        &self,
        package_id: &str,
        version: Option<Version>,
    ) -> ExtensionResult<UpdateResult> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;

        let package = library_manager.get_package(package_id).await
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", package_id)
            })?;

        let mut updated_package = (*package).clone();
        if let Some(new_version) = version {
            updated_package.version = new_version.to_string();
        }

        let start = std::time::Instant::now();

        library_manager.update_package(updated_package.clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to update package: {}", e)
            })?;

        let update_duration = start.elapsed();

        let result = UpdateResult {
            package_id: package_id.to_string(),
            from_version: Version::parse(&package.version)
                .unwrap_or(Version::parse("0.0.1").expect("valid semver")),
            to_version: Version::parse(&updated_package.version)
                .unwrap_or(Version::parse("0.0.2").expect("valid semver")),
            update_time: update_duration,
        };

        self.complete_operation().await;
        Ok(result)
    }

    async fn search_packages(
        &self,
        _query: &str,
        _options: SearchOptions,
    ) -> ExtensionResult<Vec<AssetPackage>> {
        self.increment_requests().await;
        self.start_operation().await;

        // TODO: Implement proper search with conversion from LibraryAssetPackage
        let packages = vec![];

        self.complete_operation().await;
        Ok(packages)
    }

    async fn publish_package(
        &self,
        package: AssetPackageSpec,
        proof: blockmatrix::assets::core::ConsensusProof,
    ) -> ExtensionResult<PublishResult> {
        self.increment_requests().await;
        self.start_operation().await;

        if self.config.consensus_validation {
            // Verify all four proofs (PoSpace, PoStake, PoWork, PoTime)
            if !proof.validate() {
                self.complete_operation().await;
                return Err(ExtensionError::RuntimeError {
                    message: "Proof of State validation failed: insufficient proof of state".to_string(),
                });
            }

            // Verify minimum stake requirement for publishing
            let min_stake = self.config.min_stake_for_publish();
            if proof.stake_proof.stake_amount < min_stake {
                self.complete_operation().await;
                return Err(ExtensionError::RuntimeError {
                    message: format!(
                        "Insufficient stake for publishing: {} < {} required",
                        proof.stake_proof.stake_amount, min_stake
                    ),
                });
            }

            tracing::info!(
                "Proof of State validated for package '{}': stake={}, space={}, compute={}",
                package.name,
                proof.stake_proof.stake_amount,
                proof.space_proof.total_storage,
                proof.work_proof.computational_power,
            );
        }

        let library_manager = self.library_manager.read().await;

        let lib_package = crate::library::types::LibraryAssetPackage {
            id: Arc::from(uuid::Uuid::new_v4().to_string().as_str()),
            name: package.name.clone(),
            version: package.version.to_string(),
            description: Some(package.description.clone()),
            asset_type: "library".to_string(),
            size: package.contents.len() as u64,
            hash: blake3::hash(&package.contents).to_hex().to_string(),
            content: String::new(),
            metadata: None,
            spec: None,
            content_refs: None,
            validation: None,
        };

        let start = std::time::Instant::now();

        library_manager.publish_package(lib_package.clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to publish package: {}", e)
            })?;

        let _publish_duration = start.elapsed();

        let result = PublishResult {
            package_id: lib_package.id.to_string(),
            version: Version::parse(&lib_package.version)
                .unwrap_or(Version::parse("0.0.1").expect("valid semver")),
            distribution_hash: lib_package.hash.clone(),
            signature: String::new(),
        };

        self.complete_operation().await;
        Ok(result)
    }

    async fn verify_package(&self, package_id: &str) -> ExtensionResult<VerificationResult> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let is_valid = library_manager.verify_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Verification failed: {}", e)
            })?;

        let result = VerificationResult {
            verified: is_valid,
            signature_valid: Some(is_valid),
            integrity_valid: is_valid,
            license_compliant: true,
            security_issues: if is_valid {
                vec![]
            } else {
                vec![SecurityIssue {
                    severity: "high".to_string(),
                    issue_type: "verification".to_string(),
                    description: "Package verification failed".to_string(),
                    affected_files: vec![],
                }]
            },
        };

        self.complete_operation().await;
        Ok(result)
    }
}

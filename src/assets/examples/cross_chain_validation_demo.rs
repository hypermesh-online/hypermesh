//! Cross-Chain Validation Demo
//! 
//! Demonstrates privacy-preserving cross-chain validation for real-world business workflows
//! such as vehicle purchasing across multiple entity blockchains.
//! 
//! This example shows how entities like Honda, Dealers, Banks, Insurance companies, and DMV
//! can validate asset information across their individual blockchains without exposing
//! sensitive business data.

use std::collections::HashMap;
use std::time::Duration;
use hypermesh_assets::{
    cross_chain::{
        CrossNetworkValidator, CrossChainValidationManager, BusinessWorkflowType,
        ZKProofStatement, ZKStatementType, PrivacyRequirements, NetworkValidationStep,
        ValidationCacheConfig, CacheInvalidationTrigger, CrossChainValidationRule,
        CrossChainValidationType,
    },
    matrix_blockchain::{
        MatrixBlockchainManager, EntityConfig, EntityType, MatrixCoordinate,
        GeographicDimension, OrganizationalDimension, AccessLevel, PrivacyPolicyConfig,
    },
    core::asset_id::{AssetId, AssetType},
    blockchain::AssetPrivacyLevel,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚗 Cross-Chain Validation Demo - Vehicle Purchase Workflow");
    println!("===========================================================");

    // Initialize matrix blockchain manager
    let mut matrix_manager = MatrixBlockchainManager::new();
    
    // Register entity blockchains
    register_entities(&mut matrix_manager).await?;
    
    // Create cross-chain validation manager
    let mut validator_manager = CrossChainValidationManager::new(matrix_manager);
    
    // Set up trust relationships
    setup_trust_relationships(&mut validator_manager).await?;
    
    // Register validation rules
    register_validation_rules(&mut validator_manager).await?;
    
    // Demonstrate vehicle purchase workflow
    let vehicle_asset_id = AssetId::new(AssetType::Container); // Vehicle as asset
    
    println!("\n🔍 Step 1: Performing Vehicle Purchase Cross-Chain Validation");
    println!("--------------------------------------------------------------");
    
    let participating_entities = vec![
        "honda.hypermesh.online".to_string(),
        "dealer.hypermesh.online".to_string(), 
        "bank.hypermesh.online".to_string(),
        "insurance.hypermesh.online".to_string(),
        "dmv.hypermesh.online".to_string(),
    ];
    
    match validator_manager.validate_business_workflow(
        BusinessWorkflowType::VehiclePurchase,
        vehicle_asset_id.clone(),
        participating_entities,
    ).await {
        Ok(result) => {
            println!("✅ Vehicle purchase validation completed!");
            println!("   Validation ID: {}", result.validation_id);
            println!("   Status: {:?}", result.validation_status);
            println!("   Networks validated: {}", result.network_results.len());
            
            // Display public confirmations (no sensitive data exposed)
            println!("\n📋 Public Confirmations:");
            for (key, value) in &result.public_confirmations {
                println!("   {} → {}", key, value);
            }
            
            // Display zero-knowledge proof results
            println!("\n🔐 Zero-Knowledge Proof Results:");
            for proof_result in &result.zk_proof_results {
                println!("   Statement: {} → Verified: {}", 
                    proof_result.statement_id, 
                    proof_result.verification_result
                );
            }
        }
        Err(e) => {
            println!("❌ Validation failed: {}", e);
        }
    }
    
    // Demonstrate direct cross-chain validation
    println!("\n🔍 Step 2: Direct Cross-Chain Validation Example");
    println!("--------------------------------------------------");
    
    let cross_validator = create_sample_validator(vehicle_asset_id)?;
    
    match validator_manager.validate_cross_chain(cross_validator).await {
        Ok(result) => {
            println!("✅ Direct validation completed!");
            println!("   Status: {:?}", result.validation_status);
            println!("   Cached until: {:?}", result.expires_at);
        }
        Err(e) => {
            println!("❌ Direct validation failed: {}", e);
        }
    }
    
    // Demonstrate privacy features
    demonstrate_privacy_features().await?;
    
    // Demonstrate zero-knowledge proofs
    demonstrate_zk_proofs().await?;
    
    println!("\n🎉 Cross-Chain Validation Demo Complete!");
    println!("==========================================");
    
    Ok(())
}

/// Register entity blockchains in the matrix
async fn register_entities(
    matrix_manager: &mut MatrixBlockchainManager
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 Registering Entity Blockchains");
    println!("----------------------------------");
    
    // Honda (Manufacturer)
    let honda_config = EntityConfig {
        network_domain: "honda.hypermesh.online".to_string(),
        entity_type: EntityType::Manufacturer,
        matrix_coordinate: MatrixCoordinate {
            geographic: GeographicDimension {
                region: "north-america".to_string(),
                country: "US".to_string(),
                state_province: "OH".to_string(),
                locality: "Marysville".to_string(),
                latitude: 40.2314,
                longitude: -83.3677,
            },
            organizational: OrganizationalDimension {
                network_id: "honda.hypermesh.online".to_string(),
                division_id: "manufacturing".to_string(),
                department_id: "assembly".to_string(),
                unit_id: "line-a".to_string(),
                hierarchy_level: 3,
            },
            access_level: AccessLevel::Administrative,
            temporal_index: 0,
            node_id: "honda-marysville".to_string(),
            cell_hash: [0u8; 32],
        },
        privacy_policies: PrivacyPolicyConfig {
            public_fields: vec![
                "vin".to_string(),
                "model".to_string(),
                "manufacturing_status".to_string()
            ],
            federated_fields: HashMap::from([
                ("dealer.hypermesh.online".to_string(), vec!["wholesale_price".to_string()]),
            ]),
            zk_proof_fields: vec!["manufacturing_cost".to_string()],
            default_privacy_level: AssetPrivacyLevel::PrivateNetwork,
        },
        trusted_partners: vec![
            "dealer.hypermesh.online".to_string(),
            "insurance.hypermesh.online".to_string(),
        ],
    };
    
    matrix_manager.register_entity(honda_config)?;
    println!("   ✓ Honda Manufacturing registered");
    
    // Dealer
    let dealer_config = EntityConfig {
        network_domain: "dealer.hypermesh.online".to_string(),
        entity_type: EntityType::Dealer,
        matrix_coordinate: MatrixCoordinate {
            geographic: GeographicDimension {
                region: "north-america".to_string(),
                country: "US".to_string(),
                state_province: "CA".to_string(),
                locality: "Los Angeles".to_string(),
                latitude: 34.0522,
                longitude: -118.2437,
            },
            organizational: OrganizationalDimension {
                network_id: "dealer.hypermesh.online".to_string(),
                division_id: "sales".to_string(),
                department_id: "new-vehicles".to_string(),
                unit_id: "showroom-1".to_string(),
                hierarchy_level: 3,
            },
            access_level: AccessLevel::Administrative,
            temporal_index: 0,
            node_id: "dealer-la-01".to_string(),
            cell_hash: [0u8; 32],
        },
        privacy_policies: PrivacyPolicyConfig {
            public_fields: vec![
                "availability_status".to_string(),
                "listed_price".to_string(),
            ],
            federated_fields: HashMap::from([
                ("bank.hypermesh.online".to_string(), vec!["invoice_price".to_string()]),
            ]),
            zk_proof_fields: vec!["profit_margin".to_string(), "dealer_cost".to_string()],
            default_privacy_level: AssetPrivacyLevel::PublicNetwork,
        },
        trusted_partners: vec![
            "honda.hypermesh.online".to_string(),
            "bank.hypermesh.online".to_string(),
            "insurance.hypermesh.online".to_string(),
        ],
    };
    
    matrix_manager.register_entity(dealer_config)?;
    println!("   ✓ Vehicle Dealer registered");
    
    // Bank
    let bank_config = EntityConfig {
        network_domain: "bank.hypermesh.online".to_string(),
        entity_type: EntityType::Bank,
        matrix_coordinate: MatrixCoordinate {
            geographic: GeographicDimension {
                region: "north-america".to_string(),
                country: "US".to_string(),
                state_province: "NY".to_string(),
                locality: "New York".to_string(),
                latitude: 40.7128,
                longitude: -74.0060,
            },
            organizational: OrganizationalDimension {
                network_id: "bank.hypermesh.online".to_string(),
                division_id: "consumer-lending".to_string(),
                department_id: "auto-loans".to_string(),
                unit_id: "underwriting".to_string(),
                hierarchy_level: 3,
            },
            access_level: AccessLevel::Administrative,
            temporal_index: 0,
            node_id: "bank-nyc-main".to_string(),
            cell_hash: [0u8; 32],
        },
        privacy_policies: PrivacyPolicyConfig {
            public_fields: vec![
                "loan_approval_status".to_string(),
            ],
            federated_fields: HashMap::from([
                ("dealer.hypermesh.online".to_string(), vec!["approved_amount".to_string()]),
                ("dmv.hypermesh.online".to_string(), vec!["lien_holder_info".to_string()]),
            ]),
            zk_proof_fields: vec![
                "credit_score".to_string(),
                "loan_amount".to_string(),
                "interest_rate".to_string()
            ],
            default_privacy_level: AssetPrivacyLevel::Private,
        },
        trusted_partners: vec![
            "dealer.hypermesh.online".to_string(),
            "insurance.hypermesh.online".to_string(),
            "dmv.hypermesh.online".to_string(),
        ],
    };
    
    matrix_manager.register_entity(bank_config)?;
    println!("   ✓ Financial Institution registered");
    
    // Insurance
    let insurance_config = EntityConfig {
        network_domain: "insurance.hypermesh.online".to_string(),
        entity_type: EntityType::Insurance,
        matrix_coordinate: MatrixCoordinate {
            geographic: GeographicDimension {
                region: "north-america".to_string(),
                country: "US".to_string(),
                state_province: "IL".to_string(),
                locality: "Chicago".to_string(),
                latitude: 41.8781,
                longitude: -87.6298,
            },
            organizational: OrganizationalDimension {
                network_id: "insurance.hypermesh.online".to_string(),
                division_id: "auto-insurance".to_string(),
                department_id: "underwriting".to_string(),
                unit_id: "risk-assessment".to_string(),
                hierarchy_level: 3,
            },
            access_level: AccessLevel::Administrative,
            temporal_index: 0,
            node_id: "insurance-chi-main".to_string(),
            cell_hash: [0u8; 32],
        },
        privacy_policies: PrivacyPolicyConfig {
            public_fields: vec![
                "policy_status".to_string(),
                "coverage_active".to_string(),
            ],
            federated_fields: HashMap::from([
                ("dmv.hypermesh.online".to_string(), vec!["policy_number".to_string()]),
            ]),
            zk_proof_fields: vec!["premium_amount".to_string(), "risk_score".to_string()],
            default_privacy_level: AssetPrivacyLevel::PrivateNetwork,
        },
        trusted_partners: vec![
            "honda.hypermesh.online".to_string(),
            "dealer.hypermesh.online".to_string(),
            "bank.hypermesh.online".to_string(),
            "dmv.hypermesh.online".to_string(),
        ],
    };
    
    matrix_manager.register_entity(insurance_config)?;
    println!("   ✓ Insurance Company registered");
    
    // DMV
    let dmv_config = EntityConfig {
        network_domain: "dmv.hypermesh.online".to_string(),
        entity_type: EntityType::DMV,
        matrix_coordinate: MatrixCoordinate {
            geographic: GeographicDimension {
                region: "north-america".to_string(),
                country: "US".to_string(),
                state_province: "CA".to_string(),
                locality: "Sacramento".to_string(),
                latitude: 38.5767,
                longitude: -121.4934,
            },
            organizational: OrganizationalDimension {
                network_id: "dmv.hypermesh.online".to_string(),
                division_id: "vehicle-registration".to_string(),
                department_id: "registration".to_string(),
                unit_id: "main-office".to_string(),
                hierarchy_level: 3,
            },
            access_level: AccessLevel::Administrative,
            temporal_index: 0,
            node_id: "dmv-ca-main".to_string(),
            cell_hash: [0u8; 32],
        },
        privacy_policies: PrivacyPolicyConfig {
            public_fields: vec![
                "registration_status".to_string(),
                "license_plate".to_string(),
            ],
            federated_fields: HashMap::from([
                ("insurance.hypermesh.online".to_string(), vec!["registration_date".to_string()]),
                ("bank.hypermesh.online".to_string(), vec!["lien_status".to_string()]),
            ]),
            zk_proof_fields: vec![],
            default_privacy_level: AssetPrivacyLevel::PublicNetwork,
        },
        trusted_partners: vec![
            "insurance.hypermesh.online".to_string(),
            "bank.hypermesh.online".to_string(),
        ],
    };
    
    matrix_manager.register_entity(dmv_config)?;
    println!("   ✓ Department of Motor Vehicles registered");
    
    Ok(())
}

/// Set up trust relationships between entities
async fn setup_trust_relationships(
    validator_manager: &mut CrossChainValidationManager
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🤝 Setting Up Trust Relationships");
    println!("----------------------------------");
    
    // Honda trusts dealers
    validator_manager.add_trust_relationship(
        "honda.hypermesh.online".to_string(),
        "dealer.hypermesh.online".to_string()
    );
    
    // Dealer trusts multiple entities
    validator_manager.add_trust_relationship(
        "dealer.hypermesh.online".to_string(),
        "bank.hypermesh.online".to_string()
    );
    validator_manager.add_trust_relationship(
        "dealer.hypermesh.online".to_string(),
        "insurance.hypermesh.online".to_string()
    );
    
    // Bank trusts DMV and Insurance
    validator_manager.add_trust_relationship(
        "bank.hypermesh.online".to_string(),
        "dmv.hypermesh.online".to_string()
    );
    validator_manager.add_trust_relationship(
        "bank.hypermesh.online".to_string(),
        "insurance.hypermesh.online".to_string()
    );
    
    // Insurance trusts DMV
    validator_manager.add_trust_relationship(
        "insurance.hypermesh.online".to_string(),
        "dmv.hypermesh.online".to_string()
    );
    
    println!("   ✓ Trust relationships established");
    
    Ok(())
}

/// Register validation rules for different business scenarios
async fn register_validation_rules(
    validator_manager: &mut CrossChainValidationManager
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Registering Validation Rules");
    println!("--------------------------------");
    
    // Vehicle purchase validation rule
    let vehicle_purchase_rule = CrossChainValidationRule {
        rule_id: "vehicle_purchase_workflow".to_string(),
        source_entity: EntityType::User,
        target_entity: EntityType::Dealer,
        validation_type: CrossChainValidationType::BusinessWorkflow {
            workflow_type: BusinessWorkflowType::VehiclePurchase,
            steps: vec![],
        },
        privacy_requirements: PrivacyRequirements {
            public_confirmable_fields: vec![
                "vehicle_available".to_string(),
                "financing_approved".to_string(),
                "insurance_active".to_string(),
                "registration_complete".to_string(),
            ],
            zk_proof_fields: vec![
                "loan_amount".to_string(),
                "vehicle_price".to_string(),
                "credit_score".to_string(),
            ],
            private_fields: vec![
                "manufacturing_cost".to_string(),
                "dealer_cost".to_string(),
                "profit_margin".to_string(),
                "personal_information".to_string(),
            ],
            trusted_entities: vec![
                "honda.hypermesh.online".to_string(),
                "dealer.hypermesh.online".to_string(),
                "bank.hypermesh.online".to_string(),
                "insurance.hypermesh.online".to_string(),
                "dmv.hypermesh.online".to_string(),
            ],
            max_retention_period: Duration::from_secs(86400 * 30), // 30 days
        },
        required_proofs: vec![
            hypermesh_assets::matrix_blockchain::ProofRequirement::ConsensusProof,
            hypermesh_assets::matrix_blockchain::ProofRequirement::DigitalSignature,
            hypermesh_assets::matrix_blockchain::ProofRequirement::ZeroKnowledgeProof,
        ],
    };
    
    validator_manager.register_validation_rule(vehicle_purchase_rule);
    println!("   ✓ Vehicle purchase workflow rule registered");
    
    Ok(())
}

/// Create a sample cross-network validator
fn create_sample_validator(vehicle_asset_id: AssetId) -> Result<CrossNetworkValidator, Box<dyn std::error::Error>> {
    let zk_statements = vec![
        ZKProofStatement {
            statement_id: "loan_amount_sufficient".to_string(),
            field_name: "loan_amount".to_string(),
            statement_type: ZKStatementType::GreaterThan {
                threshold: 25000.0, // Minimum loan amount
                field: "loan_amount".to_string(),
            },
            public_parameters: HashMap::from([
                ("threshold".to_string(), "25000".to_string()),
                ("currency".to_string(), "USD".to_string()),
            ]),
            verification_key: vec![1, 2, 3, 4], // Mock verification key
        },
        ZKProofStatement {
            statement_id: "credit_score_acceptable".to_string(),
            field_name: "credit_score".to_string(),
            statement_type: ZKStatementType::GreaterThan {
                threshold: 650.0, // Minimum credit score
                field: "credit_score".to_string(),
            },
            public_parameters: HashMap::from([
                ("threshold".to_string(), "650".to_string()),
                ("scale".to_string(), "FICO".to_string()),
            ]),
            verification_key: vec![5, 6, 7, 8], // Mock verification key
        },
    ];
    
    let validation_chain = vec![
        NetworkValidationStep {
            network_domain: "honda.hypermesh.online".to_string(),
            entity_type: EntityType::Manufacturer,
            validations: vec!["vehicle_exists".to_string(), "manufacturing_complete".to_string()],
            expected_confirmations: vec!["vehicle_manufactured".to_string()],
            step_order: 0,
            dependencies: vec![],
        },
        NetworkValidationStep {
            network_domain: "dealer.hypermesh.online".to_string(),
            entity_type: EntityType::Dealer,
            validations: vec!["vehicle_in_inventory".to_string(), "price_set".to_string()],
            expected_confirmations: vec!["available_for_sale".to_string()],
            step_order: 1,
            dependencies: vec![0],
        },
        NetworkValidationStep {
            network_domain: "bank.hypermesh.online".to_string(),
            entity_type: EntityType::Bank,
            validations: vec!["credit_approved".to_string(), "loan_terms_set".to_string()],
            expected_confirmations: vec!["financing_approved".to_string()],
            step_order: 2,
            dependencies: vec![1],
        },
    ];
    
    Ok(CrossNetworkValidator {
        source_network: "buyer.hypermesh.online".to_string(),
        asset_identifier: vehicle_asset_id,
        required_fields: vec![
            "vin".to_string(),
            "model".to_string(),
            "price".to_string(),
            "availability_status".to_string(),
            "financing_status".to_string(),
        ],
        validation_rules: vec![],
        zk_statements,
        validation_chain,
        cache_config: ValidationCacheConfig {
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_entries: 1000,
            invalidation_triggers: vec![
                CacheInvalidationTrigger::AssetStatusChange,
                CacheInvalidationTrigger::PrivacyPolicyUpdate,
                CacheInvalidationTrigger::TimeExpired,
            ],
        },
    })
}

/// Demonstrate privacy-preserving features
async fn demonstrate_privacy_features() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔐 Privacy Features Demonstration");
    println!("----------------------------------");
    
    println!("   🛡️  Privacy Levels:");
    println!("      • Private: No sharing, internal use only");
    println!("      • PrivateNetwork: Trusted partners only");
    println!("      • P2P: Verified peer-to-peer sharing");
    println!("      • PublicNetwork: Specific public networks");
    println!("      • FullPublic: Maximum rewards, full sharing");
    
    println!("\n   🔒 Field-Level Privacy:");
    println!("      • Public: VIN, model, availability status");
    println!("      • Federated: Wholesale prices, approved amounts");
    println!("      • Private: Manufacturing costs, profit margins");
    println!("      • ZK-Proof: Credit scores, loan amounts");
    
    println!("\n   📊 Business Benefits:");
    println!("      • Validation without data exposure");
    println!("      • Compliance with privacy regulations");
    println!("      • Reduced fraud through verification");
    println!("      • Streamlined business processes");
    
    Ok(())
}

/// Demonstrate zero-knowledge proof capabilities
async fn demonstrate_zk_proofs() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔬 Zero-Knowledge Proof Examples");
    println!("----------------------------------");
    
    println!("   🏦 Financial Validation:");
    println!("      • Prove loan amount > $25,000 without revealing exact amount");
    println!("      • Prove credit score > 650 without revealing actual score");
    println!("      • Prove income sufficiency without revealing salary");
    
    println!("\n   🚗 Vehicle Validation:");
    println!("      • Prove vehicle value within range without revealing exact value");
    println!("      • Prove manufacturing compliance without revealing processes");
    println!("      • Prove warranty coverage without revealing terms");
    
    println!("\n   📋 Regulatory Compliance:");
    println!("      • Prove age eligibility without revealing birthdate");
    println!("      • Prove residence without revealing exact address");
    println!("      • Prove insurance coverage without revealing premium");
    
    println!("\n   🔧 Technical Implementation:");
    println!("      • PLONK proof system for efficient verification");
    println!("      • Quantum-resistant security parameters");
    println!("      • Verification keys for statement validation");
    println!("      • Public parameters for threshold proofs");
    
    Ok(())
}
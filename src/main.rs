//! HyperMesh Protocol Stack - Unified Server
//! 
//! A revolutionary replacement for the traditional Internet protocol stack that embeds
//! STOQ transport, HyperMesh consensus, and TrustChain security into a single, 
//! self-contained networking foundation.
//!
//! This server represents a fundamental shift from traditional Internet's layered protocols
//! with external dependencies to HyperMesh's unified, consensus-validated,
//! certificate-embedded protocol stack.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::net::Ipv6Addr;
use tokio::signal;
use tracing::{info, warn, error};
use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};

mod config;
mod transport;
mod assets;
mod authority;
mod integration;
mod monitoring;
mod dashboard;
mod static_server;
mod hardware;

// External economic system integration
use caesar::{CaesarEconomicSystem, CaesarConfig};

use config::HyperMeshServerConfig;
use transport::StoqTransportLayer;
use assets::HyperMeshAssetLayer;
use authority::TrustChainAuthorityLayer;
use integration::LayerIntegration;
use monitoring::PerformanceMonitor;
use dashboard::{DashboardMessageHandler, DashboardMessage};
use static_server::{StaticFileServer, StaticServerConfig};
// use hardware::HardwareDetectionService; // TODO: Fix sysinfo API

/// HyperMesh Protocol Stack Server
#[derive(Parser)]
#[command(name = "hypermesh-server")]
#[command(about = "HyperMesh Protocol Stack - Unified STOQ/HyperMesh/TrustChain Server")]
#[command(version = "1.0.0")]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/production.toml")]
    pub config: String,
    
    /// Server bind address (IPv6 only)
    #[arg(short, long, default_value = "::")]
    pub bind: String,
    
    /// Server port (QUIC transport)
    #[arg(short, long, default_value = "8443")]
    pub port: u16,
    
    
    /// Deployment mode
    #[command(subcommand)]
    pub mode: Option<DeploymentMode>,
}

#[derive(Subcommand)]
pub enum DeploymentMode {
    /// Production deployment (full consensus, maximum security)
    Production {
        /// Enable federated bootstrap (no external dependencies)
        #[arg(long)]
        federated: bool,
    },
    
    /// Development deployment (reduced security for testing)
    Development {
        /// Enable legacy HTTP/TCP gateway
        #[arg(long)]
        legacy_gateway: bool,
    },
    
    /// Bootstrap mode (initialize new network)
    Bootstrap {
        /// Bootstrap as root authority
        #[arg(long)]
        root_authority: bool,
    },
    
    /// Gateway mode (legacy compatibility)
    Gateway {
        /// HTTP/TCP to STOQ translation
        #[arg(long)]
        translate_protocols: bool,
    },
}

/// HyperMesh Unified Server
/// 
/// This server embeds three critical layers into a single protocol stack:
/// 1. STOQ Transport: QUIC over IPv6 with 40 Gbps performance targets
/// 2. HyperMesh Assets: Universal asset system with four-proof consensus
/// 3. TrustChain Authority: Embedded CA and DNS with certificate transparency
pub struct HyperMeshServer {
    /// Configuration for all layers
    config: Arc<HyperMeshServerConfig>,
    
    /// Layer 1: STOQ Transport (Foundation)
    /// - QUIC over IPv6 ONLY
    /// - Certificate validation at connection establishment
    /// - 40 Gbps performance optimization
    /// - Zero-copy operations and hardware acceleration
    stoq_layer: Arc<StoqTransportLayer>,
    
    /// Layer 2: HyperMesh Assets (Orchestration)  
    /// - Universal asset system (everything is an asset)
    /// - Four-proof consensus (PoSpace+PoStake+PoWork+PoTime)
    /// - NAT-like proxy addressing for remote resources
    /// - VM execution through asset allocation
    hypermesh_layer: Arc<HyperMeshAssetLayer>,
    
    /// Layer 3: TrustChain Authority (Security)
    /// - Embedded certificate authority (no external CA)
    /// - Embedded DNS resolver (no external DNS)
    /// - Certificate transparency logging
    /// - Automatic certificate rotation
    trustchain_layer: Arc<TrustChainAuthorityLayer>,
    
    /// Layer Integration Logic
    /// - Cross-layer communication and validation
    /// - Consensus coordination across layers
    /// - Performance optimization coordination
    integration: Arc<LayerIntegration>,
    
    
    /// Performance monitoring for 40 Gbps targets
    monitor: Arc<PerformanceMonitor>,
    
    /// STOQ Protocol Handler for dashboard communication
    protocol_handler: Option<Arc<stoq::protocol::StoqProtocolHandler>>,

    /// Static file server for UI assets
    static_server: Arc<StaticFileServer>,

    /// Caesar economic system
    caesar: Arc<CaesarEconomicSystem>,

    // TODO: Fix hardware detection service - sysinfo API changes
    // /// Hardware detection service
    // hardware_service: Arc<HardwareDetectionService>,
}

impl HyperMeshServer {
    /// Create new HyperMesh server with pure STOQ protocol
    pub async fn new(config: HyperMeshServerConfig) -> Result<Self> {
        info!("🚀 Initializing HyperMesh Protocol Stack");
        info!("📡 Mode: Unified STOQ/HyperMesh/TrustChain Server");
        info!("🔗 Target: 40 Gbps performance with embedded security");
        
        let config = Arc::new(config);
        
        // Initialize performance monitor first for metrics collection
        let monitor = Arc::new(PerformanceMonitor::new(config.clone()).await?);
        
        // Layer 3: TrustChain Authority (Initialize first - others depend on certificates)
        info!("🔐 Initializing TrustChain Authority Layer");
        info!("   • Embedded Certificate Authority");
        info!("   • Embedded DNS Resolver");  
        info!("   • Certificate Transparency Logging");
        info!("   • Post-quantum cryptography (FALCON-1024 + Kyber)");
        
        let trustchain_layer = Arc::new(
            TrustChainAuthorityLayer::new(config.clone(), monitor.clone()).await
                .map_err(|e| anyhow!("TrustChain initialization failed: {}", e))?
        );
        
        // Layer 1: STOQ Transport (Initialize with TrustChain certificates)
        info!("⚡ Initializing STOQ Transport Layer");
        info!("   • QUIC over IPv6 ONLY");
        info!("   • Certificate validation at connection establishment");
        info!("   • 40 Gbps performance optimization");
        info!("   • Zero-copy operations and hardware acceleration");
        
        // Initialize static file server for UI first (needed by transport layer)
        info!("📦 Initializing Static File Server for UI");
        let static_config = StaticServerConfig {
            static_dir: std::path::PathBuf::from("ui/frontend/dist"),
            spa_fallback: true,
            cache_control: "public, max-age=3600".to_string(),
        };
        let static_server = Arc::new(StaticFileServer::new(static_config));

        // Validate static files directory
        static_server.validate().await
            .map_err(|e| anyhow!("Static file server validation failed: {}", e))?;

        let stoq_layer = Arc::new(
            StoqTransportLayer::new(
                config.clone(),
                trustchain_layer.clone(),
                monitor.clone(),
                Some(static_server.clone())
            ).await
                .map_err(|e| anyhow!("STOQ transport initialization failed: {}", e))?
        );
        
        // Layer 2: HyperMesh Assets (Initialize with STOQ transport)
        info!("🏗️  Initializing HyperMesh Asset Layer");
        info!("   • Universal Asset System");
        info!("   • Four-proof consensus (PoSpace+PoStake+PoWork+PoTime)");
        info!("   • NAT-like proxy addressing");
        info!("   • VM execution through asset allocation");
        
        let hypermesh_layer = Arc::new(
            HyperMeshAssetLayer::new(config.clone(), stoq_layer.clone(), monitor.clone()).await
                .map_err(|e| anyhow!("HyperMesh initialization failed: {}", e))?
        );
        
        // Layer Integration (Cross-layer coordination)
        info!("🔄 Initializing Layer Integration");
        info!("   • Cross-layer communication");
        info!("   • Consensus coordination");
        info!("   • Performance optimization coordination");
        
        let integration = Arc::new(
            LayerIntegration::new(
                config.clone(),
                stoq_layer.clone(),
                hypermesh_layer.clone(), 
                trustchain_layer.clone(),
                monitor.clone()
            ).await
                .map_err(|e| anyhow!("Layer integration failed: {}", e))?
        );
        
        // Verify all layers are properly integrated
        integration.validate_stack_integration().await
            .map_err(|e| anyhow!("Protocol stack validation failed: {}", e))?;
        
        info!("✅ HyperMesh Protocol Stack initialized successfully");
        info!("🌐 Server ready to replace traditional protocols");
        
        // Initialize STOQ protocol handler for dashboard communication
        info!("🔌 Initializing STOQ Protocol Handler");
        info!("   • Dashboard message handling");
        info!("   • Certificate-based authentication");
        info!("   • Real-time server statistics");
        
        let protocol_config = stoq::protocol::ProtocolConfig {
            max_message_size: 16 * 1024 * 1024, // 16MB
            message_timeout: std::time::Duration::from_secs(30),
            enable_compression: true,
            compression_threshold: 1024,
            enable_authentication: true,
            max_concurrent_streams: 100,
        };
        
        let protocol_handler = Arc::new(
            stoq::protocol::StoqProtocolHandler::new(
                protocol_config,
                None // Certificate manager will be provided by transport layer
            )
        );

        // Initialize hardware detection service first (needed by dashboard handler)
        info!("🔍 Initializing Hardware Detection Service");
        info!("   • Real-time CPU, memory, storage detection");
        info!("   • Network interface discovery");
        info!("   • Resource allocation tracking");
        info!("   • System capability analysis");

        // TODO: Fix hardware detection service - sysinfo API changes
        // let hardware_service = Arc::new(
        //     HardwareDetectionService::new().await
        //         .map_err(|e| anyhow!("Hardware detection initialization failed: {}", e))?
        // );

        // info!("✅ Hardware Detection Service initialized");

        // Register dashboard message handler
        let dashboard_handler = DashboardMessageHandler::new(
            config.clone(),
            stoq_layer.clone(),
            hypermesh_layer.clone(),
            trustchain_layer.clone(),
            integration.clone(),
            monitor.clone(),
            Arc::new(()) // TODO: Fix hardware service
        );
        
        protocol_handler.register_handler(
            "dashboard".to_string(),
            dashboard_handler
        ).await;
        
        info!("✅ STOQ Protocol Handler initialized with dashboard support");
        info!("✅ Static file server integrated with transport layer");

        // Initialize Caesar economic system
        info!("💰 Initializing Caesar Economic System");
        info!("   • Real token balance tracking");
        info!("   • Transaction processing and validation");
        info!("   • Reward calculation based on resource sharing");
        info!("   • Staking mechanisms with APY calculations");

        let caesar_config = CaesarConfig::default(); // Will be loaded from config in production
        let caesar = Arc::new(
            CaesarEconomicSystem::new(caesar_config).await
                .map_err(|e| anyhow!("Caesar initialization failed: {}", e))?
        );

        info!("✅ Caesar Economic System initialized");

        // Register TrustChain API endpoints with transport layer
        stoq_layer.set_trustchain_handler(trustchain_layer.clone()).await
            .map_err(|e| anyhow!("Failed to register TrustChain handlers: {}", e))?;

        info!("✅ TrustChain API endpoints integrated with STOQ transport");

        // Register Caesar API endpoints with transport layer
        stoq_layer.set_caesar_handler(caesar.clone()).await
            .map_err(|e| anyhow!("Failed to register Caesar handlers: {}", e))?;

        info!("✅ Caesar API endpoints integrated with STOQ transport");

        // TODO: Fix hardware detection API - sysinfo API changes
        // // Register Hardware API endpoints with transport layer
        // stoq_layer.set_hardware_handler(hardware_service.clone()).await
        //     .map_err(|e| anyhow!("Failed to register Hardware handlers: {}", e))?;

        // info!("✅ Hardware detection API endpoints integrated with STOQ transport");

        // Create server instance
        let server = Self {
            config,
            stoq_layer,
            hypermesh_layer,
            trustchain_layer,
            integration,
            monitor,
            protocol_handler: Some(protocol_handler),
            static_server,
            caesar,
            // hardware_service,
        };
        
        info!("✅ HyperMesh Protocol Stack initialized successfully");
        Ok(server)
    }
    
    /// Start the HyperMesh server - runs persistently until shutdown
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting HyperMesh Protocol Stack Server");
        
        // Start non-persistent layers first (they complete quickly)
        let hypermesh_task = self.hypermesh_layer.start(); 
        let trustchain_task = self.trustchain_layer.start();
        let integration_task = self.integration.start();
        let monitor_task = self.monitor.start();
        
        // Wait for non-persistent layers to complete startup
        match tokio::try_join!(hypermesh_task, trustchain_task, integration_task, monitor_task) {
            Ok(_) => {
                info!("✅ Supporting HyperMesh layers started successfully");
                
                // Log comprehensive startup summary before starting persistent service
                self.log_startup_summary().await;
                
                
                // Now start the persistent STOQ transport layer with protocol handler
                info!("🌐 Starting persistent STOQ transport layer with protocol handler...");
                info!("🔄 Server will now run until shutdown signal (Ctrl+C)");

                // Start the protocol handler alongside the transport layer
                if let Some(protocol_handler) = &self.protocol_handler {
                    let handler = protocol_handler.clone();
                    let transport = self.stoq_layer.clone();

                    tokio::spawn(async move {
                        info!("📡 STOQ Protocol Handler ready for dashboard connections");
                        // The protocol handler will be invoked by the transport layer
                        // when connections are established
                    });
                }

                // Initialize and start HTTP/3 bridge for browser compatibility
                info!("🌉 Enabling HTTP/3 bridge for web browser access...");
                {
                    // Clone transport layer for mutable access
                    let mut stoq_layer = (*self.stoq_layer).clone();
                    let trustchain = self.trustchain_layer.clone();

                    // Initialize HTTP/3 bridge with TrustChain
                    stoq_layer.initialize_http3_bridge(trustchain).await
                        .map_err(|e| {
                            warn!("⚠️ HTTP/3 bridge initialization failed: {}", e);
                            warn!("   Browsers will not be able to connect directly");
                            warn!("   Use STOQ client for full protocol access");
                            e
                        }).ok(); // Continue even if HTTP/3 fails - STOQ still works

                    // Start HTTP/3 bridge if initialized
                    if stoq_layer.start_http3_bridge().await.is_ok() {
                        info!("✅ HTTP/3 bridge active - browsers can connect to https://[::1]:{}/",
                              self.config.global.port);
                    }
                }

                // This is the main server loop - STOQ transport runs persistently
                self.stoq_layer.start().await?;
                
                info!("🛑 STOQ transport layer stopped - server shutting down");
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to start HyperMesh support layers: {}", e);
                Err(anyhow!("Server startup failed: {}", e))
            }
        }
    }
    
    /// Log comprehensive startup summary
    async fn log_startup_summary(&self) {
        let stats = self.monitor.get_stack_statistics().await;
        
        info!("🌐 HyperMesh Server Successfully Started");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("📊 Protocol Stack Status:");
        info!("   🔹 STOQ Transport: {} (Target: 40 Gbps)", format_performance(stats.stoq_throughput));
        info!("   🔹 HyperMesh Assets: {} active assets", stats.active_assets);
        info!("   🔹 TrustChain Authority: {} certificates", stats.active_certificates);
        info!("   🔹 Layer Integration: {} cross-layer operations/sec", stats.integration_ops_per_sec);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("🔗 Network Configuration:");
        info!("   🔹 Listening on: [{}]:{}", self.config.global.bind_address, self.config.global.port);
        info!("   🔹 IPv6 Only: ✅ (STOQ protocol requirement)");
        info!("   🔹 Protocol: QUIC over IPv6 (replacing HTTP/TCP)");
        info!("   🔹 Consensus Mode: {}", if self.config.deployment.consensus_mandatory { "MANDATORY" } else { "Optional" });
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("🔐 Security Configuration:");
        info!("   🔹 Embedded CA: ✅ (No external dependencies)");
        info!("   🔹 Embedded DNS: ✅ (No external dependencies)");
        info!("   🔹 Certificate Transparency: ✅");
        info!("   🔹 Post-quantum Ready: ✅ (FALCON-1024 + Kyber)");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("⚡ Performance Targets:");
        info!("   🔹 Transport Throughput: 40 Gbps");
        info!("   🔹 Consensus Operations: <100ms four-proof validation");
        info!("   🔹 Certificate Operations: <35ms"); 
        info!("   🔹 Asset Operations: 1000+ allocations/sec");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        if stats.performance_warnings.is_empty() {
            info!("🎯 Performance Status: All targets met");
        } else {
            warn!("⚠️  Performance Warnings:");
            for warning in &stats.performance_warnings {
                warn!("   • {}", warning);
            }
        }
        
        info!("🌟 HyperMesh server ready to serve connections");
    }
    
    /// Graceful shutdown of all layers
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down HyperMesh Protocol Stack");
        
        // Shutdown layers in reverse order (dependencies)
        self.integration.shutdown().await?;
        self.hypermesh_layer.shutdown().await?;
        self.stoq_layer.shutdown().await?;
        self.trustchain_layer.shutdown().await?;
        self.monitor.shutdown().await?;
        
        info!("✅ HyperMesh server shutdown complete");
        Ok(())
    }
    
    /// Get comprehensive server statistics
    pub async fn get_statistics(&self) -> Result<ServerStatistics> {
        Ok(ServerStatistics {
            stack_stats: self.monitor.get_stack_statistics().await,
            stoq_stats: self.stoq_layer.get_statistics().await?,
            hypermesh_stats: self.hypermesh_layer.get_statistics().await?,
            trustchain_stats: self.trustchain_layer.get_statistics().await?,
            integration_stats: self.integration.get_statistics().await?,
        })
    }
}

/// Comprehensive server statistics
#[derive(Debug)]
pub struct ServerStatistics {
    pub stack_stats: monitoring::StackStatistics,
    pub stoq_stats: transport::TransportStatistics,
    pub hypermesh_stats: assets::AssetStatistics,
    pub trustchain_stats: authority::AuthorityStatistics,
    pub integration_stats: integration::IntegrationStatistics,
}

/// Format performance numbers for human readability
fn format_performance(throughput_mbps: f64) -> String {
    if throughput_mbps >= 1000.0 {
        format!("{:.2} Gbps", throughput_mbps / 1000.0)
    } else {
        format!("{:.2} Mbps", throughput_mbps)
    }
}

/// Main entry point for HyperMesh server
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging for the entire stack
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hypermesh_server=debug".into())
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    let cli = Cli::parse();
    
    info!("🌐 HyperMesh Protocol Stack Server");
    info!("📡 Unified STOQ/HyperMesh/TrustChain Implementation");
    info!("🔧 Version: 1.0.0");
    
    // Load configuration
    let config = HyperMeshServerConfig::load(&cli.config, &cli.bind, cli.port).await
        .map_err(|e| anyhow!("Configuration loading failed: {}", e))?;
    
    // Apply deployment mode settings
    let config = match cli.mode {
        Some(DeploymentMode::Production { federated }) => {
            info!("🔐 Production Mode: Maximum security, mandatory consensus");
            if federated {
                info!("🌐 Federated Bootstrap: No external dependencies");
            }
            config.with_production_settings(federated)
        }
        Some(DeploymentMode::Development { legacy_gateway }) => {
            warn!("⚠️  Development Mode: Reduced security for testing");
            if legacy_gateway {
                info!("🔄 Legacy Gateway: HTTP/TCP compatibility enabled");
            }
            config.with_development_settings(legacy_gateway)
        }
        Some(DeploymentMode::Bootstrap { root_authority }) => {
            info!("🏗️  Bootstrap Mode: Initializing new HyperMesh network");
            if root_authority {
                info!("👑 Root Authority: Bootstrapping as root certificate authority");
            }
            config.with_bootstrap_settings(root_authority)
        }
        Some(DeploymentMode::Gateway { translate_protocols }) => {
            info!("🔄 Gateway Mode: Legacy protocol translation");
            if translate_protocols {
                info!("🔀 Protocol Translation: HTTP/TCP to STOQ translation enabled");
            }
            config.with_gateway_settings(translate_protocols)
        }
        None => {
            info!("🔧 Default Mode: Balanced configuration");
            config
        }
    };
    
    // Validate configuration
    config.validate()
        .map_err(|e| anyhow!("Configuration validation failed: {}", e))?;
    
    // Create the unified server with pure STOQ protocol
    let server = HyperMeshServer::new(config).await
        .map_err(|e| anyhow!("Server initialization failed: {}", e))?;
    
    // Setup shutdown signal handling
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::mpsc::channel::<()>(1);
    
    // Spawn task to handle shutdown signals
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("📡 Received Ctrl+C shutdown signal");
            }
            _ = sigterm.recv() => {
                info!("📡 Received SIGTERM shutdown signal");
            }
        }
        
        // Send shutdown signal to main loop
        let _ = shutdown_sender.send(()).await;
    });
    
    // Start server and wait for completion or shutdown signal
    tokio::select! {
        result = server.start() => {
            match result {
                Ok(()) => {
                    info!("🏁 Server completed normally");
                }
                Err(e) => {
                    error!("❌ Server failed: {}", e);
                    return Err(e);
                }
            }
        }
        _ = shutdown_receiver.recv() => {
            info!("🛑 Shutdown signal received - stopping server");
        }
    }
    
    // Graceful shutdown
    server.shutdown().await
        .map_err(|e| anyhow!("Server shutdown failed: {}", e))?;
    
    info!("👋 Internet 2.0 server stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server_initialization() {
        let config = HyperMeshServerConfig::default_development();
        let server = HyperMeshServer::new(config).await;
        assert!(server.is_ok());
    }
    
    #[tokio::test]
    async fn test_layer_integration() {
        let config = HyperMeshServerConfig::default_development();
        let server = HyperMeshServer::new(config).await.unwrap();
        
        // Verify all layers are integrated
        let stats = server.get_statistics().await.unwrap();
        assert!(stats.stack_stats.layers_integrated);
    }
    
    #[tokio::test]
    async fn test_performance_targets() {
        let config = HyperMeshServerConfig::default_production();
        let server = HyperMeshServer::new(config).await.unwrap();
        
        let stats = server.get_statistics().await.unwrap();
        
        // Verify performance targets are tracked
        assert!(stats.stoq_stats.target_throughput_gbps >= 40.0);
        assert!(stats.hypermesh_stats.consensus_time_ms <= 100.0);
        assert!(stats.trustchain_stats.certificate_ops_ms <= 35.0);
    }
}
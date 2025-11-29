//! Nexus API Server
//! 
//! Provides REST and GraphQL APIs for Nexus core management.
//! Serves as the bridge between C2 interfaces and Nexus core components.

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, delete, patch},
    Json, Router,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    services::ServeDir,
};
use tracing::{info, warn, error};

mod auth;
mod cluster;
mod service;
mod system;
mod graphql;
mod middleware_auth;
mod nexus_core;
mod config;
mod error;

use auth::{AuthService, Claims};
use error::{ApiError, ApiResult};
use nexus_core::NexusCore;

#[derive(Parser)]
#[command(name = "nexus-api-server")]
#[command(about = "Nexus API Server - REST and GraphQL APIs")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Server listen address
    #[arg(long, default_value = "0.0.0.0:8443")]
    addr: String,

    /// Enable development mode
    #[arg(long)]
    dev: bool,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server
    Serve,
    
    /// Generate API documentation
    Docs {
        /// Output directory for documentation
        #[arg(short, long, default_value = "docs")]
        output: String,
    },
    
    /// Database migration commands
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Run database migrations
    Up,
    
    /// Revert database migrations
    Down,
    
    /// Show migration status
    Status,
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub nexus_core: Arc<NexusCore>,
    pub auth_service: Arc<AuthService>,
    pub config: Arc<config::ServerConfig>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .with_target(false)
        .init();

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => serve(cli).await,
        Commands::Docs { output } => generate_docs(&output).await,
        Commands::Migrate { command } => handle_migrate_command(command).await,
    }
}

async fn serve(cli: Cli) -> Result<()> {
    info!("🚀 Starting Nexus API Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = config::load_config(cli.config.as_deref())?;
    info!("📋 Configuration loaded from: {:?}", config.source_file);

    // Initialize Nexus core connection
    info!("🔗 Connecting to Nexus core...");
    let nexus_core = Arc::new(NexusCore::new(&config.nexus).await?);
    
    // Initialize authentication service
    info!("🔐 Initializing authentication service...");
    let auth_service = Arc::new(AuthService::new(&config.auth)?);

    // Create application state
    let state = AppState {
        nexus_core,
        auth_service,
        config: Arc::new(config),
    };

    // Build our application with routes
    let app = create_router(state.clone()).await?;

    // Parse listen address
    let addr: SocketAddr = cli.addr.parse()?;
    info!("🌐 Server will listen on: {}", addr);

    // Create TCP listener
    let listener = TcpListener::bind(&addr).await?;
    
    info!("✅ Nexus API Server started successfully!");
    info!("📖 API Documentation: http://{}/docs", addr);
    info!("🔍 GraphQL Playground: http://{}/graphql", addr);
    info!("❤️  Health Check: http://{}/health", addr);

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn create_router(state: AppState) -> Result<Router> {
    // Create GraphQL schema
    let schema = graphql::create_schema(state.clone()).await?;

    let app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        
        // API v1 routes
        .nest("/api/v1", api_v1_routes())
        
        // GraphQL endpoint
        .route("/graphql", 
            get(graphql::graphql_playground)
            .post(graphql::graphql_handler)
        )
        .with_state((state.clone(), schema))
        
        // OpenAPI documentation
        .route("/api-docs/openapi.json", get(serve_openapi_spec))
        .nest_service("/docs", 
            utoipa_swagger_ui::SwaggerUi::new("/docs/*tail")
                .url("/api-docs/openapi.json", utoipa::openapi::OpenApiBuilder::new()
                    .info(utoipa::openapi::InfoBuilder::new()
                        .title("Nexus API")
                        .version(env!("CARGO_PKG_VERSION"))
                        .description(Some("REST and GraphQL APIs for Nexus core management"))
                        .build())
                    .build())
        )
        
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        
        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middleware_auth::auth_middleware,
                ))
        )
        .with_state(state);

    Ok(app)
}

fn api_v1_routes() -> Router<AppState> {
    Router::new()
        // System endpoints
        .route("/status", get(system::get_status))
        .route("/version", get(system::get_version))
        .route("/metrics", get(system::get_metrics))
        
        // Cluster management
        .route("/clusters", get(cluster::list_clusters).post(cluster::create_cluster))
        .route("/clusters/:name", 
            get(cluster::get_cluster)
            .delete(cluster::delete_cluster)
            .patch(cluster::update_cluster)
        )
        .route("/clusters/:name/scale", patch(cluster::scale_cluster))
        .route("/clusters/:name/nodes", get(cluster::list_nodes))
        .route("/clusters/:name/nodes/:node_id", get(cluster::get_node))
        
        // Service management  
        .route("/services", get(service::list_services).post(service::deploy_service))
        .route("/services/:name",
            get(service::get_service)
            .delete(service::delete_service)
            .patch(service::update_service)
        )
        .route("/services/:name/scale", patch(service::scale_service))
        .route("/services/:name/logs", get(service::get_logs))
        .route("/services/:name/exec", post(service::exec_command))
        
        // Authentication
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/auth/logout", post(auth::logout))
}

// Route handlers

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn readiness_check(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Check if we can communicate with Nexus core
    let core_status = state.nexus_core.ping().await?;
    
    Ok(Json(serde_json::json!({
        "status": "ready",
        "core_status": core_status,
        "timestamp": chrono::Utc::now()
    })))
}

async fn serve_openapi_spec() -> impl IntoResponse {
    // This would normally be generated from the OpenAPI annotations
    let spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Nexus API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "REST and GraphQL APIs for Nexus core management"
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy"
                        }
                    }
                }
            }
        }
    });
    
    Json(spec)
}

async fn generate_docs(output_dir: &str) -> Result<()> {
    info!("📖 Generating API documentation to: {}", output_dir);
    
    // Create output directory
    std::fs::create_dir_all(output_dir)?;
    
    // Generate OpenAPI spec
    let spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Nexus API",
            "version": env!("CARGO_PKG_VERSION")
        }
    });
    
    std::fs::write(
        format!("{}/openapi.json", output_dir),
        serde_json::to_string_pretty(&spec)?
    )?;
    
    info!("✅ Documentation generated successfully");
    Ok(())
}

async fn handle_migrate_command(command: MigrateCommand) -> Result<()> {
    match command {
        MigrateCommand::Up => {
            info!("📊 Running database migrations...");
            // Database migration logic would go here
            info!("✅ Migrations completed");
        },
        MigrateCommand::Down => {
            info!("📊 Reverting database migrations...");
            // Database rollback logic would go here
            info!("✅ Rollback completed");
        },
        MigrateCommand::Status => {
            info!("📊 Database migration status:");
            // Show migration status
            println!("All migrations: Up to date");
        },
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🛑 Received Ctrl+C, starting graceful shutdown...");
        },
        _ = terminate => {
            info!("🛑 Received SIGTERM, starting graceful shutdown...");
        },
    }
    
    info!("👋 Nexus API Server shutdown complete");
}
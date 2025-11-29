//! Dashboard web server implementation

use std::sync::{Arc, RwLock};
use tracing::info;
use nexus_shared::Result;

use super::config::WebServerConfig;
use super::metrics::MetricsStorage;

/// Dashboard web server
pub struct DashboardWebServer {
    pub config: WebServerConfig,
    pub metrics_storage: Arc<RwLock<MetricsStorage>>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl DashboardWebServer {
    pub fn new(config: &WebServerConfig, metrics_storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            config: config.clone(),
            metrics_storage,
            server_handle: None,
        }
    }

    pub async fn start_server(&self) -> Result<()> {
        info!("Starting dashboard web server on {}:{}", self.config.bind_address, self.config.port);
        Ok(())
    }
}

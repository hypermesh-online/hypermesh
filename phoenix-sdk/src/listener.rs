//! Phoenix Listener Management
//!
//! Provides server-side connection acceptance and handling.

use std::sync::Arc;
use std::net::SocketAddr;
use parking_lot::RwLock;
use tracing::{info, debug, warn};
use stoq::{StoqTransport, Listener as StoqListener};
use trustchain::TrustChain;

use crate::{
    config::PhoenixConfig,
    connection::PhoenixConnection,
    metrics::MetricsCollector,
    security::SecurityManager,
    errors::{PhoenixError, Result},
};

/// Phoenix listener for accepting incoming connections
pub struct PhoenixListener {
    /// Listener ID
    id: String,
    /// Local address
    local_addr: SocketAddr,
    /// Underlying STOQ listener
    inner: Arc<dyn StoqListener>,
    /// Listener state
    state: Arc<RwLock<ListenerState>>,
    /// Configuration
    config: Arc<PhoenixConfig>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Security manager
    security: Arc<SecurityManager>,
}

/// Listener state
#[derive(Debug, Clone, PartialEq)]
pub enum ListenerState {
    /// Listener is starting
    Starting,
    /// Listener is active
    Active,
    /// Listener is stopping
    Stopping,
    /// Listener is stopped
    Stopped,
}

impl PhoenixListener {
    /// Create new Phoenix listener
    pub(crate) fn new(
        inner: Arc<dyn StoqListener>,
        local_addr: SocketAddr,
        config: Arc<PhoenixConfig>,
        metrics: Arc<MetricsCollector>,
        security: Arc<SecurityManager>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();

        Self {
            id,
            local_addr,
            inner,
            state: Arc::new(RwLock::new(ListenerState::Active)),
            config,
            metrics,
            security,
        }
    }

    /// Accept incoming connection
    ///
    /// # Example
    /// ```rust
    /// let listener = phoenix.listen(8080).await?;
    ///
    /// loop {
    ///     match listener.accept().await {
    ///         Ok(conn) => {
    ///             tokio::spawn(async move {
    ///                 handle_connection(conn).await;
    ///             });
    ///         }
    ///         Err(e) => eprintln!("Accept error: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn accept(&self) -> Result<PhoenixConnection> {
        // Check listener state
        if !self.is_active() {
            return Err(PhoenixError::Other("Listener is not active".to_string()));
        }

        // Accept STOQ connection
        let stoq_conn = self.inner.accept().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        let remote_addr = stoq_conn.remote_address()
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        // Create Phoenix connection
        let conn = PhoenixConnection::new(
            stoq_conn,
            remote_addr,
            &self.config,
            self.metrics.clone(),
            self.security.clone(),
        );

        // Update metrics
        self.metrics.increment_accepted_connections();

        info!("Accepted connection from {}", remote_addr);
        Ok(conn)
    }

    /// Handle connections with async closure
    ///
    /// # Example
    /// ```rust
    /// listener.handle(|conn| async move {
    ///     while let Ok(msg) = conn.receive::<String>().await {
    ///         println!("Received: {}", msg);
    ///         conn.send(&format!("Echo: {}", msg)).await?;
    ///     }
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn handle<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(PhoenixConnection) -> Fut + Send + Sync + 'static,
        F: Clone,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        info!("Starting connection handler on {}", self.local_addr);

        while self.is_active() {
            match self.accept().await {
                Ok(conn) => {
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler(conn).await {
                            warn!("Connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    warn!("Accept error: {}", e);
                    // Continue accepting unless listener is stopped
                    if !self.is_active() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get listener ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Check if listener is active
    pub fn is_active(&self) -> bool {
        matches!(*self.state.read(), ListenerState::Active)
    }

    /// Stop listener
    pub async fn stop(&self) -> Result<()> {
        *self.state.write() = ListenerState::Stopping;
        // Listener will stop accepting on next iteration
        *self.state.write() = ListenerState::Stopped;
        info!("Listener {} stopped", self.id);
        Ok(())
    }
}

/// Listener manager for handling multiple listeners
pub(crate) struct ListenerManager {
    transport: Arc<StoqTransport>,
    trustchain: Arc<TrustChain>,
    listeners: Arc<dashmap::DashMap<String, Arc<PhoenixListener>>>,
}

impl ListenerManager {
    pub fn new(
        transport: Arc<StoqTransport>,
        trustchain: Arc<TrustChain>,
    ) -> Self {
        Self {
            transport,
            trustchain,
            listeners: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub async fn create_listener(
        &self,
        addr: SocketAddr,
        config: &PhoenixConfig,
        metrics: Arc<MetricsCollector>,
        security: Arc<SecurityManager>,
    ) -> Result<PhoenixListener> {
        // Create STOQ listener
        let stoq_listener = self.transport.listen(addr).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        // Create Phoenix listener
        let listener = PhoenixListener::new(
            Arc::new(stoq_listener),
            addr,
            Arc::new(config.clone()),
            metrics,
            security,
        );

        // Track listener
        self.listeners.insert(listener.id.clone(), Arc::new(listener.clone()));

        Ok(listener)
    }

    pub async fn shutdown(&self) {
        for entry in self.listeners.iter() {
            if let Err(e) = entry.value().stop().await {
                warn!("Error stopping listener {}: {}", entry.key(), e);
            }
        }
        self.listeners.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listener_state() {
        let state = ListenerState::Starting;
        assert_eq!(state, ListenerState::Starting);

        let state = ListenerState::Active;
        assert_eq!(state, ListenerState::Active);
    }
}
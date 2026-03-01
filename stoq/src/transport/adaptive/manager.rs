// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use quinn::Connection as QuinnConnection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::warn;

use super::connection::{AdaptationStats, AdaptiveConnection};

pub struct AdaptationManager {
    connections: Arc<dashmap::DashMap<String, Arc<AdaptiveConnection>>>,
    enabled: AtomicBool,
    adaptation_interval: Duration,
}

impl AdaptationManager {
    pub fn new(adaptation_interval: Duration) -> Self {
        Self {
            connections: Arc::new(dashmap::DashMap::new()),
            enabled: AtomicBool::new(true),
            adaptation_interval,
        }
    }

    pub fn register_connection(&self, id: String, connection: Arc<QuinnConnection>) {
        let adaptive = Arc::new(AdaptiveConnection::new(connection));
        self.connections.insert(id, adaptive);
    }

    pub fn unregister_connection(&self, id: &str) {
        self.connections.remove(id);
    }

    pub async fn start(self: Arc<Self>) {
        let mut ticker = interval(self.adaptation_interval);

        loop {
            ticker.tick().await;

            if !self.enabled.load(Ordering::Relaxed) {
                continue;
            }

            for entry in self.connections.iter() {
                let connection = entry.value().clone();

                tokio::spawn(async move {
                    if let Err(e) = connection.adapt().await {
                        warn!("Failed to adapt connection: {}", e);
                    }
                });
            }
        }
    }

    pub fn get_connection(&self, id: &str) -> Option<Arc<AdaptiveConnection>> {
        self.connections.get(id).map(|entry| entry.clone())
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn connection_ids(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn all_stats(&self) -> Vec<(String, AdaptationStats)> {
        self.connections
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().adaptation_stats()))
            .collect()
    }
}

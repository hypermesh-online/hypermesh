//! Event system for system-wide notifications and monitoring

use nexus_shared::NodeId;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream};

/// System-wide event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SystemEvent {
    /// System lifecycle events
    SystemStarted {
        node_id: NodeId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    SystemStopped {
        node_id: NodeId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Service lifecycle events
    ServiceDeployed {
        service_name: String,
        replicas: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ServiceReady {
        service_name: String,
        endpoints: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ServiceScaled {
        service_name: String,
        old_replicas: u32,
        new_replicas: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ServiceDeleted {
        service_name: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Cluster events
    NodeJoined {
        node_id: NodeId,
        cluster_id: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    NodeLeft {
        node_id: NodeId,
        cluster_id: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    LeaderElected {
        new_leader: NodeId,
        cluster_id: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Health events
    ComponentHealthChanged {
        component: String,
        old_status: String,
        new_status: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Resource events
    ResourceAlert {
        resource_type: ResourceType,
        threshold: f64,
        current_value: f64,
        node_id: Option<NodeId>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Security events
    AuthenticationFailed {
        source_ip: String,
        user_id: Option<String>,
        reason: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    CertificateRotated {
        node_id: NodeId,
        certificate_type: String,
        expires_at: chrono::DateTime<chrono::Utc>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Network events
    NetworkPartition {
        affected_nodes: Vec<NodeId>,
        duration_seconds: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ConnectionEstablished {
        source_node: NodeId,
        target_node: NodeId,
        protocol: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    CPU,
    Memory,
    Disk,
    Network,
    FileDescriptors,
}

/// Event stream wrapper
pub struct EventStream {
    receiver: BroadcastStream<SystemEvent>,
}

impl EventStream {
    pub fn new(receiver: broadcast::Receiver<SystemEvent>) -> Self {
        Self {
            receiver: BroadcastStream::new(receiver),
        }
    }
    
    /// Convert to a stream
    pub fn into_stream(self) -> impl Stream<Item = Result<SystemEvent, broadcast::error::RecvError>> {
        self.receiver
    }
}

/// Event filter for subscribing to specific event types
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub service_events: bool,
    pub cluster_events: bool,
    pub health_events: bool,
    pub resource_events: bool,
    pub security_events: bool,
    pub network_events: bool,
    pub service_names: Option<Vec<String>>,
    pub node_ids: Option<Vec<NodeId>>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            service_events: true,
            cluster_events: true,
            health_events: true,
            resource_events: true,
            security_events: true,
            network_events: true,
            service_names: None,
            node_ids: None,
        }
    }
}

impl EventFilter {
    /// Check if an event matches this filter
    pub fn matches(&self, event: &SystemEvent) -> bool {
        match event {
            SystemEvent::ServiceDeployed { service_name, .. } |
            SystemEvent::ServiceReady { service_name, .. } |
            SystemEvent::ServiceScaled { service_name, .. } |
            SystemEvent::ServiceDeleted { service_name, .. } => {
                self.service_events && 
                self.service_names.as_ref()
                    .map_or(true, |names| names.contains(service_name))
            },

            SystemEvent::NodeJoined { node_id, .. } |
            SystemEvent::NodeLeft { node_id, .. } |
            SystemEvent::LeaderElected { new_leader: node_id, .. } => {
                self.cluster_events && 
                self.node_ids.as_ref()
                    .map_or(true, |ids| ids.contains(node_id))
            },

            SystemEvent::ComponentHealthChanged { .. } => self.health_events,

            SystemEvent::ResourceAlert { node_id, .. } => {
                self.resource_events && 
                node_id.as_ref().map_or(true, |id| {
                    self.node_ids.as_ref().map_or(true, |ids| ids.contains(id))
                })
            },

            SystemEvent::AuthenticationFailed { .. } |
            SystemEvent::CertificateRotated { .. } => self.security_events,

            SystemEvent::NetworkPartition { .. } |
            SystemEvent::ConnectionEstablished { .. } => self.network_events,

            SystemEvent::SystemStarted { .. } |
            SystemEvent::SystemStopped { .. } => true, // Always pass system events
        }
    }
}

/// Event aggregator for metrics and monitoring
pub struct EventAggregator {
    events: Vec<SystemEvent>,
    max_events: usize,
}

impl EventAggregator {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events),
            max_events,
        }
    }

    pub fn add_event(&mut self, event: SystemEvent) {
        self.events.push(event);
        
        // Keep only the most recent events
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    pub fn events(&self) -> &[SystemEvent] {
        &self.events
    }

    /// Get events within a time range
    pub fn events_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&SystemEvent> {
        self.events
            .iter()
            .filter(|event| {
                let timestamp = match event {
                    SystemEvent::SystemStarted { timestamp, .. } |
                    SystemEvent::SystemStopped { timestamp, .. } |
                    SystemEvent::ServiceDeployed { timestamp, .. } |
                    SystemEvent::ServiceReady { timestamp, .. } |
                    SystemEvent::ServiceScaled { timestamp, .. } |
                    SystemEvent::ServiceDeleted { timestamp, .. } |
                    SystemEvent::NodeJoined { timestamp, .. } |
                    SystemEvent::NodeLeft { timestamp, .. } |
                    SystemEvent::LeaderElected { timestamp, .. } |
                    SystemEvent::ComponentHealthChanged { timestamp, .. } |
                    SystemEvent::ResourceAlert { timestamp, .. } |
                    SystemEvent::AuthenticationFailed { timestamp, .. } |
                    SystemEvent::CertificateRotated { timestamp, .. } |
                    SystemEvent::NetworkPartition { timestamp, .. } |
                    SystemEvent::ConnectionEstablished { timestamp, .. } => timestamp,
                };
                *timestamp >= start && *timestamp <= end
            })
            .collect()
    }

    /// Count events by type
    pub fn count_by_type(&self) -> std::collections::HashMap<String, u32> {
        let mut counts = std::collections::HashMap::new();
        
        for event in &self.events {
            let event_type = match event {
                SystemEvent::SystemStarted { .. } => "system_started",
                SystemEvent::SystemStopped { .. } => "system_stopped",
                SystemEvent::ServiceDeployed { .. } => "service_deployed",
                SystemEvent::ServiceReady { .. } => "service_ready",
                SystemEvent::ServiceScaled { .. } => "service_scaled",
                SystemEvent::ServiceDeleted { .. } => "service_deleted",
                SystemEvent::NodeJoined { .. } => "node_joined",
                SystemEvent::NodeLeft { .. } => "node_left",
                SystemEvent::LeaderElected { .. } => "leader_elected",
                SystemEvent::ComponentHealthChanged { .. } => "component_health_changed",
                SystemEvent::ResourceAlert { .. } => "resource_alert",
                SystemEvent::AuthenticationFailed { .. } => "authentication_failed",
                SystemEvent::CertificateRotated { .. } => "certificate_rotated",
                SystemEvent::NetworkPartition { .. } => "network_partition",
                SystemEvent::ConnectionEstablished { .. } => "connection_established",
            };
            
            *counts.entry(event_type.to_string()).or_insert(0) += 1;
        }
        
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_filter_service_names() {
        let filter = EventFilter {
            service_names: Some(vec!["nginx".to_string()]),
            ..Default::default()
        };
        
        let event = SystemEvent::ServiceDeployed {
            service_name: "nginx".to_string(),
            replicas: 3,
            timestamp: chrono::Utc::now(),
        };
        
        assert!(filter.matches(&event));
        
        let event2 = SystemEvent::ServiceDeployed {
            service_name: "redis".to_string(),
            replicas: 1,
            timestamp: chrono::Utc::now(),
        };
        
        assert!(!filter.matches(&event2));
    }
    
    #[test]
    fn test_event_aggregator() {
        let mut aggregator = EventAggregator::new(10);
        
        let event = SystemEvent::SystemStarted {
            node_id: NodeId::random(),
            timestamp: chrono::Utc::now(),
        };
        
        aggregator.add_event(event);
        assert_eq!(aggregator.events().len(), 1);
        
        let counts = aggregator.count_by_type();
        assert_eq!(counts.get("system_started"), Some(&1));
    }
}
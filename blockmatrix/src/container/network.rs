//! Container networking implementation

use crate::ContainerId;
use super::error::{Result, ContainerError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use tracing::{info, debug};

/// Network configuration for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable networking
    pub enabled: bool,
    /// Network mode (bridge, host, none)
    pub mode: String,
    /// Port mappings
    pub port_mappings: Vec<PortMapping>,
    /// DNS servers
    pub dns_servers: Vec<IpAddr>,
    /// Hostname
    pub hostname: String,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,
    /// Container port
    pub container_port: u16,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Network namespace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNamespace {
    pub id: ContainerId,
    pub ip_address: IpAddr,
    pub gateway: IpAddr,
    pub netmask: u8,
    pub interface_name: String,
    pub bridge_name: String,
}

/// Container network trait
#[async_trait]
pub trait ContainerNetwork: Send + Sync {
    async fn create_network_namespace(&self, id: ContainerId, config: &NetworkConfig) -> Result<NetworkNamespace>;
    async fn delete_network_namespace(&self, id: ContainerId) -> Result<()>;
    async fn get_namespace(&self, id: ContainerId) -> Result<NetworkNamespace>;
    async fn configure_port_forwarding(&self, id: ContainerId, host_port: u16, container_port: u16) -> Result<()>;
}

/// Default container network implementation
pub struct DefaultContainerNetwork {
    namespaces: std::sync::Arc<tokio::sync::RwLock<HashMap<ContainerId, NetworkNamespace>>>,
    ip_allocator: std::sync::Arc<tokio::sync::Mutex<u32>>,
}

impl DefaultContainerNetwork {
    pub fn new() -> Self {
        Self {
            namespaces: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ip_allocator: std::sync::Arc::new(tokio::sync::Mutex::new(2)), // Start from .2
        }
    }
}

#[async_trait]
impl ContainerNetwork for DefaultContainerNetwork {
    async fn create_network_namespace(&self, id: ContainerId, _config: &NetworkConfig) -> Result<NetworkNamespace> {
        let mut ip_allocator = self.ip_allocator.lock().await;
        let ip_suffix = *ip_allocator;
        *ip_allocator += 1;
        drop(ip_allocator);
        
        let namespace = NetworkNamespace {
            id,
            ip_address: IpAddr::V4(Ipv4Addr::new(172, 17, 0, ip_suffix as u8)),
            gateway: IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
            netmask: 16,
            interface_name: format!("veth{}", id.as_uuid().simple()),
            bridge_name: "hypermesh0".to_string(),
        };
        
        let mut namespaces = self.namespaces.write().await;
        namespaces.insert(id, namespace.clone());
        
        info!("Created network namespace for container {} with IP {}", id, namespace.ip_address);
        Ok(namespace)
    }
    
    async fn delete_network_namespace(&self, id: ContainerId) -> Result<()> {
        let mut namespaces = self.namespaces.write().await;
        namespaces.remove(&id);
        info!("Deleted network namespace for container {}", id);
        Ok(())
    }
    
    async fn get_namespace(&self, id: ContainerId) -> Result<NetworkNamespace> {
        let namespaces = self.namespaces.read().await;
        namespaces.get(&id).cloned()
            .ok_or_else(|| ContainerError::network("Network namespace not found"))
    }
    
    async fn configure_port_forwarding(&self, id: ContainerId, host_port: u16, container_port: u16) -> Result<()> {
        debug!("Configuring port forwarding for container {}: {}:{}", id, host_port, container_port);
        Ok(())
    }
}

impl Default for DefaultContainerNetwork {
    fn default() -> Self {
        Self::new()
    }
}
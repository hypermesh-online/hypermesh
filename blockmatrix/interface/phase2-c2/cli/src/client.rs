//! Nexus API client for communication with core components

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Client for communicating with Nexus core components
pub struct NexusClient {
    http_client: Client,
    base_url: Url,
    token: Option<String>,
}

impl NexusClient {
    /// Create a new Nexus client
    pub fn new(api_url: Option<String>, token: Option<String>) -> Result<Self> {
        let base_url = api_url
            .unwrap_or_else(|| "https://localhost:8443".to_string());
        
        let base_url = Url::parse(&base_url)?;
        
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("nexus-cli/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        
        Ok(Self {
            http_client,
            base_url,
            token,
        })
    }
    
    /// Get system status from Nexus core
    pub async fn get_system_status(&self) -> Result<SystemStatusResponse> {
        let url = self.base_url.join("/api/v1/status")?;
        
        let mut request = self.http_client.get(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "API request failed with status: {}",
                response.status()
            ));
        }
        
        let status = response.json().await?;
        Ok(status)
    }
    
    /// Get cluster information
    pub async fn get_cluster(&self, name: &str) -> Result<ClusterResponse> {
        let url = self.base_url.join(&format!("/api/v1/clusters/{}", name))?;
        
        let mut request = self.http_client.get(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get cluster '{}': {}",
                name,
                response.status()
            ));
        }
        
        let cluster = response.json().await?;
        Ok(cluster)
    }
    
    /// List all clusters
    pub async fn list_clusters(&self) -> Result<ClustersResponse> {
        let url = self.base_url.join("/api/v1/clusters")?;
        
        let mut request = self.http_client.get(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to list clusters: {}",
                response.status()
            ));
        }
        
        let clusters = response.json().await?;
        Ok(clusters)
    }
    
    /// Create a new cluster
    pub async fn create_cluster(&self, spec: &ClusterCreateRequest) -> Result<ClusterResponse> {
        let url = self.base_url.join("/api/v1/clusters")?;
        
        let mut request = self.http_client.post(url)
            .json(spec);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to create cluster: {}",
                response.status()
            ));
        }
        
        let cluster = response.json().await?;
        Ok(cluster)
    }
    
    /// Delete a cluster
    pub async fn delete_cluster(&self, name: &str) -> Result<()> {
        let url = self.base_url.join(&format!("/api/v1/clusters/{}", name))?;
        
        let mut request = self.http_client.delete(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to delete cluster '{}': {}",
                name,
                response.status()
            ));
        }
        
        Ok(())
    }
    
    /// Get service information
    pub async fn get_service(&self, name: &str) -> Result<ServiceResponse> {
        let url = self.base_url.join(&format!("/api/v1/services/{}", name))?;
        
        let mut request = self.http_client.get(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get service '{}': {}",
                name,
                response.status()
            ));
        }
        
        let service = response.json().await?;
        Ok(service)
    }
    
    /// List all services
    pub async fn list_services(&self, namespace: Option<&str>) -> Result<ServicesResponse> {
        let mut url = self.base_url.join("/api/v1/services")?;
        
        if let Some(ns) = namespace {
            url.query_pairs_mut().append_pair("namespace", ns);
        }
        
        let mut request = self.http_client.get(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to list services: {}",
                response.status()
            ));
        }
        
        let services = response.json().await?;
        Ok(services)
    }
    
    /// Deploy a new service
    pub async fn deploy_service(&self, spec: &ServiceDeployRequest) -> Result<ServiceResponse> {
        let url = self.base_url.join("/api/v1/services")?;
        
        let mut request = self.http_client.post(url)
            .json(spec);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to deploy service: {}",
                response.status()
            ));
        }
        
        let service = response.json().await?;
        Ok(service)
    }
    
    /// Delete a service
    pub async fn delete_service(&self, name: &str) -> Result<()> {
        let url = self.base_url.join(&format!("/api/v1/services/{}", name))?;
        
        let mut request = self.http_client.delete(url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to delete service '{}': {}",
                name,
                response.status()
            ));
        }
        
        Ok(())
    }
    
    /// Scale a service
    pub async fn scale_service(&self, name: &str, replicas: u32) -> Result<ServiceResponse> {
        let url = self.base_url.join(&format!("/api/v1/services/{}/scale", name))?;
        
        let request_body = ServiceScaleRequest { replicas };
        
        let mut request = self.http_client.patch(url)
            .json(&request_body);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to scale service '{}': {}",
                name,
                response.status()
            ));
        }
        
        let service = response.json().await?;
        Ok(service)
    }
}

// API Response Types

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub cluster_health: String,
    pub node_count: u32,
    pub service_count: u32,
    pub active_connections: u32,
    pub uptime_seconds: u64,
    pub version: String,
    pub components: Vec<ComponentStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub status: String,
    pub health: String,
    pub connections: u32,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterResponse {
    pub name: String,
    pub status: String,
    pub node_count: u32,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub endpoint: String,
    pub high_availability: bool,
    pub nodes: Vec<NodeStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatus {
    pub id: String,
    pub status: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_tx: u64,
    pub network_rx: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClustersResponse {
    pub clusters: Vec<ClusterSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub name: String,
    pub status: String,
    pub node_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub high_availability: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub name: String,
    pub image: String,
    pub status: String,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub endpoint: Option<String>,
    pub environment: std::collections::HashMap<String, String>,
    pub resources: ServiceResources,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResources {
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<String>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_tx: u64,
    pub network_rx: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServicesResponse {
    pub services: Vec<ServiceSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceSummary {
    pub name: String,
    pub image: String,
    pub status: String,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// API Request Types

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterCreateRequest {
    pub name: String,
    pub node_count: u32,
    pub node_size: String,
    pub high_availability: bool,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceDeployRequest {
    pub name: String,
    pub image: String,
    pub replicas: u32,
    pub environment: std::collections::HashMap<String, String>,
    pub resources: ServiceResourceRequests,
    pub ports: Vec<ServicePort>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResourceRequests {
    pub cpu: Option<f64>,
    pub memory: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServicePort {
    pub name: String,
    pub port: u16,
    pub target_port: u16,
    pub protocol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceScaleRequest {
    pub replicas: u32,
}
//! Capability-based security system

use crate::{Principal, Resource, Operation, error::{Result, SecurityError}};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Capability identifier
pub type CapabilityId = String;

/// Permission set for capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub delete: bool,
    pub modify_permissions: bool,
    pub delegate: bool,
}

/// Security capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub resource: Resource,
    pub permissions: PermissionSet,
    pub expiry: Option<SystemTime>,
    pub delegation_depth: u8,
    pub signature: String, // Cryptographic signature
}

/// Capability system implementation
pub struct CapabilitySystem {
    capabilities: RwLock<HashMap<Principal, Vec<Capability>>>,
    capability_store: RwLock<HashMap<CapabilityId, Capability>>,
}

impl CapabilitySystem {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
            capability_store: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn grant_capability(&self, principal: Principal, capability: Capability) -> Result<()> {
        let mut capabilities = self.capabilities.write().await;
        let mut store = self.capability_store.write().await;
        
        store.insert(capability.id.clone(), capability.clone());
        capabilities.entry(principal).or_insert_with(Vec::new).push(capability);
        
        Ok(())
    }
    
    pub async fn check_permission(&self, principal: &Principal, resource: &Resource, operation: &Operation) -> Result<bool> {
        let capabilities = self.capabilities.read().await;
        
        if let Some(caps) = capabilities.get(principal) {
            for cap in caps {
                if self.capability_matches(cap, resource, operation) && !self.is_expired(cap) {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    fn capability_matches(&self, cap: &Capability, resource: &Resource, operation: &Operation) -> bool {
        // Simplified matching logic
        match operation {
            Operation::Read => cap.permissions.read,
            Operation::Write => cap.permissions.write,
            Operation::Execute => cap.permissions.execute,
            Operation::Delete => cap.permissions.delete,
            _ => false,
        }
    }
    
    fn is_expired(&self, cap: &Capability) -> bool {
        cap.expiry.map_or(false, |expiry| SystemTime::now() > expiry)
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            delete: false,
            modify_permissions: false,
            delegate: false,
        }
    }
}
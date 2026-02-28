// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GlobalAddress implementation - address creation, conversion, and parsing

use std::net::{Ipv6Addr, SocketAddrV6};
use std::time::SystemTime;
use blake3;

use crate::assets::core::{AssetRegistration, AssetResult, AssetError};
use super::types::{GlobalAddress, GlobalAddressType};

impl GlobalAddress {
    /// Create new global address
    pub fn new(
        network_prefix: [u8; 8],
        node_id: [u8; 8],
        asset_id: &AssetRegistration,
        service_port: u16,
        address_type: GlobalAddressType,
    ) -> Self {
        let mut asset_bytes = [0u8; 16];
        asset_bytes.copy_from_slice(&asset_id.content_hash[..16]);

        Self {
            network_prefix,
            node_id,
            asset_id: asset_bytes,
            service_port,
            address_type,
            created_at: SystemTime::now(),
        }
    }

    /// Convert to IPv6 address representation for network compatibility
    pub fn to_ipv6(&self) -> Ipv6Addr {
        let mut ipv6_bytes = [0u8; 16];
        ipv6_bytes[0..8].copy_from_slice(&self.network_prefix);
        ipv6_bytes[8..16].copy_from_slice(&self.node_id);
        Ipv6Addr::from(ipv6_bytes)
    }

    /// Convert to socket address
    pub fn to_socket_addr(&self) -> SocketAddrV6 {
        SocketAddrV6::new(self.to_ipv6(), self.service_port, 0, 0)
    }

    /// Get string representation
    pub fn to_string(&self) -> String {
        format!(
            "hypermesh://{}/{}/{}:{}",
            hex::encode(self.network_prefix),
            hex::encode(self.node_id),
            hex::encode(self.asset_id),
            self.service_port
        )
    }

    /// Parse from string representation
    pub fn from_string(s: &str) -> AssetResult<Self> {
        if !s.starts_with("hypermesh://") {
            return Err(AssetError::AdapterError {
                message: "Invalid global address scheme".to_string()
            });
        }

        let addr_part = &s[12..];
        let parts: Vec<&str> = addr_part.split('/').collect();

        if parts.len() != 3 {
            return Err(AssetError::AdapterError {
                message: "Invalid global address format".to_string()
            });
        }

        let network_bytes = hex::decode(parts[0])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid network prefix".to_string()
            })?;
        if network_bytes.len() != 8 {
            return Err(AssetError::AdapterError {
                message: "Network prefix must be 8 bytes".to_string()
            });
        }
        let mut network_prefix = [0u8; 8];
        network_prefix.copy_from_slice(&network_bytes);

        let node_bytes = hex::decode(parts[1])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid node ID".to_string()
            })?;
        if node_bytes.len() != 8 {
            return Err(AssetError::AdapterError {
                message: "Node ID must be 8 bytes".to_string()
            });
        }
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&node_bytes);

        let asset_port: Vec<&str> = parts[2].split(':').collect();
        if asset_port.len() != 2 {
            return Err(AssetError::AdapterError {
                message: "Invalid asset:port format".to_string()
            });
        }

        let asset_bytes = hex::decode(asset_port[0])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid asset ID".to_string()
            })?;
        if asset_bytes.len() != 16 {
            return Err(AssetError::AdapterError {
                message: "Asset ID must be 16 bytes".to_string()
            });
        }
        let mut asset_id = [0u8; 16];
        asset_id.copy_from_slice(&asset_bytes);

        let service_port: u16 = asset_port[1].parse()
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid service port".to_string()
            })?;

        Ok(Self {
            network_prefix,
            node_id,
            asset_id,
            service_port,
            address_type: GlobalAddressType::Memory,
            created_at: SystemTime::now(),
        })
    }

    /// Generate address hash for validation
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.network_prefix);
        hasher.update(&self.node_id);
        hasher.update(&self.asset_id);
        hasher.update(&self.service_port.to_le_bytes());
        hasher.update(&format!("{:?}", self.address_type).as_bytes());

        *hasher.finalize().as_bytes()
    }
}

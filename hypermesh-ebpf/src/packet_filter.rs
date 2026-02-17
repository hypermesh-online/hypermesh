// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Packet Filtering
//!
//! XDP packet filtering with HyperMesh intelligence validation.
//! This replaces generic STOQ filtering with HyperMesh-specific logic.

use anyhow::{Result, anyhow};
use crate::policy_maps::PolicyManager;
use crate::hypermesh_headers::*;

/// Filter action for packets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Pass packet to userspace
    Pass,
    /// Drop packet at kernel level
    Drop,
    /// Redirect to AF_XDP socket for zero-copy
    Redirect,
}

/// HyperMesh packet filter with intelligence validation
pub struct HyperMeshPacketFilter {
    interface: String,
    policy_manager: PolicyManager,
    #[cfg(feature = "kernel-attach")]
    _xdp_program: Option<XdpProgram>,
}

#[cfg(feature = "kernel-attach")]
struct XdpProgram {
    /// Loaded BPF object containing the XDP program and maps
    bpf: aya::Bpf,
}

impl HyperMeshPacketFilter {
    /// Create new packet filter
    pub fn new(interface: &str, policy_manager: PolicyManager) -> Result<Self> {
        Ok(Self {
            interface: interface.to_string(),
            policy_manager,
            #[cfg(feature = "kernel-attach")]
            _xdp_program: None,
        })
    }

    /// Attach filter to network interface
    ///
    /// With `kernel-attach` feature: loads the HyperMesh XDP program into the kernel
    /// and attaches it to the specified interface using aya.
    /// Without: returns an error indicating the feature is not enabled.
    #[cfg(feature = "kernel-attach")]
    pub fn attach(&mut self) -> Result<()> {
        use aya::programs::{Xdp, XdpFlags};

        tracing::info!("Attaching HyperMesh XDP filter to {}", self.interface);

        // Look for pre-compiled HyperMesh eBPF object
        let bpf_paths = [
            std::path::PathBuf::from("/sys/fs/bpf/hypermesh_xdp"),
            std::path::PathBuf::from("target/bpf/hypermesh_xdp.o"),
        ];

        let load_path = bpf_paths.iter().find(|p| p.exists());

        match load_path {
            Some(path) => {
                let mut bpf = aya::Bpf::load_file(path)
                    .map_err(|e| anyhow!("Failed to load HyperMesh eBPF from {:?}: {}", path, e))?;

                // Try to find and attach the XDP filter program
                if let Some(program) = bpf.program_mut("hypermesh_xdp_filter") {
                    let xdp: &mut Xdp = program.try_into()
                        .map_err(|e| anyhow!("Program is not XDP type: {}", e))?;
                    xdp.load()
                        .map_err(|e| anyhow!("Failed to load XDP program: {}", e))?;
                    // Use SKB_MODE (generic) for maximum compatibility
                    xdp.attach(&self.interface, XdpFlags::SKB_MODE)
                        .map_err(|e| anyhow!("Failed to attach XDP to {}: {}", self.interface, e))?;

                    tracing::info!(
                        "HyperMesh XDP filter attached to {} (kernel mode)",
                        self.interface
                    );
                } else {
                    tracing::warn!(
                        "No 'hypermesh_xdp_filter' program in {:?}, filter not attached",
                        path
                    );
                }

                self._xdp_program = Some(XdpProgram { bpf });
            }
            None => {
                tracing::warn!(
                    "No compiled HyperMesh eBPF object found. XDP filter not attached to {}. \
                     Userspace validation remains active.",
                    self.interface
                );
            }
        }

        // Sync policies to kernel maps (or log if not kernel-attached)
        self.policy_manager.sync_to_kernel()?;

        Ok(())
    }

    #[cfg(not(feature = "kernel-attach"))]
    pub fn attach(&mut self) -> Result<()> {
        Err(anyhow!("kernel-attach feature not enabled; use validate_packet_userspace() instead"))
    }

    /// Detach filter from interface
    pub fn detach(&self) -> Result<()> {
        tracing::info!("Detaching HyperMesh XDP filter from {}", self.interface);
        Ok(())
    }

    /// Userspace packet validation (for testing without eBPF)
    pub fn validate_packet_userspace(
        &self,
        connection_id: u64,
        packet_data: &[u8],
    ) -> FilterAction {
        let policy = self.policy_manager.get_policy(connection_id);

        // Check packet size
        if packet_data.len() > policy.max_packet_size as usize {
            tracing::debug!("Packet too large: {} > {}", packet_data.len(), policy.max_packet_size);
            return FilterAction::Drop;
        }

        // For actual implementation, this would:
        // 1. Parse STOQ packet structure
        // 2. Extract extension headers
        // 3. Validate based on policy
        // 4. Return Pass/Drop/Redirect

        // Placeholder validation
        FilterAction::Pass
    }

    /// Validate Proof of State extension (userspace)
    pub fn validate_proof_of_state(
        &self,
        proof: &ProofOfStateHeader,
    ) -> bool {
        // Validate timestamps
        if !proof.validate_timestamps() {
            tracing::warn!("Proof of State timestamp validation failed");
            return false;
        }

        // In production, this would validate:
        // - Proof of Stake (WHO) - Check signature against known identities
        // - Proof of Work (WHAT) - Verify computational challenge
        // - Proof of Time (WHEN) - Check temporal ordering
        // - Proof of Space (WHERE) - Validate storage commitment

        true
    }

    /// Validate Asset Hash extension (userspace)
    pub fn validate_asset_hash(
        &self,
        header: &AssetHashHeader,
        _payload: &[u8],
    ) -> bool {
        // Validate shard indices
        if !header.validate_shard_indices() {
            tracing::warn!("Invalid shard indices in asset hash header");
            return false;
        }

        // In production, this would:
        // 1. Compute BLAKE3 hash of payload
        // 2. Compare with header.hash
        // 3. Verify asset_id against blockchain registry

        // Placeholder validation
        true
    }

    /// Validate Matrix Routing extension (userspace)
    pub fn validate_matrix_routing(
        &self,
        routing: &MatrixRoutingHeader,
        matrix_size: u16,
    ) -> bool {
        // Validate path (no loops, within bounds)
        if !routing.validate_path(matrix_size) {
            tracing::warn!("Invalid matrix routing path");
            return false;
        }

        // In production, this would:
        // 1. Check routing path against known topology
        // 2. Verify next hop is valid neighbor
        // 3. Validate destination is reachable

        true
    }

    /// Get interface name
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Get policy manager
    pub fn policy_manager(&self) -> &PolicyManager {
        &self.policy_manager
    }
}

impl Drop for HyperMeshPacketFilter {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_filter_creation() {
        let manager = PolicyManager::new().unwrap();
        let filter = HyperMeshPacketFilter::new("eth0", manager);
        assert!(filter.is_ok());
    }

    #[test]
    fn test_userspace_validation() {
        let manager = PolicyManager::new().unwrap();
        manager.set_default_policy(ValidationPolicy::permissive());

        let filter = HyperMeshPacketFilter::new("eth0", manager.clone()).unwrap();

        let packet = vec![0u8; 1500]; // Standard MTU
        let action = filter.validate_packet_userspace(123, &packet);
        assert_eq!(action, FilterAction::Pass);

        let large_packet = vec![0u8; 70000]; // Too large
        manager.set_default_policy(ValidationPolicy::default());
        let action = filter.validate_packet_userspace(123, &large_packet);
        assert_eq!(action, FilterAction::Drop);
    }

    #[test]
    fn test_proof_of_state_validation() {
        let manager = PolicyManager::new().unwrap();
        let filter = HyperMeshPacketFilter::new("eth0", manager).unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let valid_proof = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: now,
            where_: [3u8; 16],
        };

        assert!(filter.validate_proof_of_state(&valid_proof));

        // Future proof (invalid)
        let future_proof = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: now + 10 * 60 * 1_000_000, // 10 minutes in future
            where_: [3u8; 16],
        };

        assert!(!filter.validate_proof_of_state(&future_proof));
    }
}

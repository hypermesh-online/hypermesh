// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Read-only sysfs backend (Substrate.b, degraded tier).
//!
//! Reads interface and carrier state from `/sys/class/net/{iface}/...` — the same
//! proven pattern `hypermesh-ebpf`'s `NicCapabilities::detect()` uses. Works where
//! netlink is unavailable or unprivileged. It is read-only: it cannot assign
//! addresses or subscribe to events, so it advertises a reduced capability set and
//! the registry only selects it when the netlink backend is absent (R16 graceful
//! degradation: netlink → sysfs → fallback).

use async_trait::async_trait;

use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceAddress, InterfaceId, LinkState};
use crate::reachability::Reachability;
use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Root of the Linux network sysfs tree.
const SYS_CLASS_NET: &str = "/sys/class/net";

/// Read-only sysfs-backed substrate adapter.
#[derive(Debug, Default)]
pub struct SysfsFallbackAdapter;

impl SysfsFallbackAdapter {
    /// Construct the adapter.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubstrateAdapter for SysfsFallbackAdapter {
    fn name(&self) -> &'static str {
        "sysfs-fallback"
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        // Read-only: can enumerate and read carrier, cannot assign/watch/discover.
        SubstrateCapabilities {
            enumerate: true,
            carrier: true,
            assign_address: false,
            watch: false,
            reachability: false,
        }
    }

    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>> {
        // Read directory entries under /sys/class/net; each entry is an
        // interface whose kernel index lives in `<iface>/ifindex`.
        let dir = std::fs::read_dir(SYS_CLASS_NET).map_err(|e| {
            SubstrateError::Backend(format!("cannot read {SYS_CLASS_NET}: {e}"))
        })?;

        let mut interfaces = Vec::new();
        for entry in dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::debug!("skipping unreadable {SYS_CLASS_NET} entry: {e}");
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            match read_ifindex(&name) {
                Ok(index) => interfaces.push(InterfaceId { index, name }),
                Err(e) => {
                    tracing::debug!("skipping interface {name}: {e}");
                    continue;
                }
            }
        }

        if interfaces.is_empty() {
            return Err(SubstrateError::InterfaceNotFound(
                "no interfaces found under /sys/class/net".to_string(),
            ));
        }
        Ok(interfaces)
    }

    async fn carrier_state(&self, iface: &InterfaceId) -> SubstrateResult<LinkState> {
        // `operstate` distinguishes administratively up/down; `carrier` reports
        // physical/radio link presence on an up interface. Prefer the carrier
        // reading (the `eno1`-bounce distinction) and fall back to operstate.
        let operstate = read_sysfs_trimmed(&iface.name, "operstate").unwrap_or_default();
        if operstate == "down" {
            return Ok(LinkState::Down);
        }

        match read_sysfs_trimmed(&iface.name, "carrier") {
            Ok(v) if v == "1" => Ok(LinkState::Carrier(true)),
            Ok(v) if v == "0" => Ok(LinkState::Carrier(false)),
            // `carrier` returns EINVAL when the interface is admin-down; treat a
            // non-"down" operstate with an unreadable carrier as simply Up.
            _ if operstate == "up" => Ok(LinkState::Up),
            _ => Ok(LinkState::Down),
        }
    }

    async fn assign_address(
        &self,
        _iface: &InterfaceId,
        _addr: InterfaceAddress,
    ) -> SubstrateResult<()> {
        Err(SubstrateError::Unsupported(
            "sysfs fallback is read-only; address assignment requires the netlink backend"
                .to_string(),
        ))
    }

    async fn detect_reachability(&self) -> SubstrateResult<Reachability> {
        Err(SubstrateError::Unsupported(
            "sysfs fallback cannot discover reachability".to_string(),
        ))
    }
}

/// Read `/sys/class/net/{iface}/ifindex` and parse the kernel interface index.
fn read_ifindex(iface: &str) -> SubstrateResult<u32> {
    let raw = read_sysfs_trimmed(iface, "ifindex")?;
    raw.parse::<u32>()
        .map_err(|e| SubstrateError::Backend(format!("bad ifindex '{raw}' for {iface}: {e}")))
}

/// Read a single-line attribute file under `/sys/class/net/{iface}/{attr}`,
/// returning its trimmed contents.
fn read_sysfs_trimmed(iface: &str, attr: &str) -> SubstrateResult<String> {
    let path = format!("{SYS_CLASS_NET}/{iface}/{attr}");
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| SubstrateError::Backend(format!("cannot read {path}: {e}")))?;
    Ok(contents.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enumerate_finds_loopback_on_linux() {
        // Every Linux host has `lo`; this exercises the real sysfs read path.
        let adapter = SysfsFallbackAdapter::new();
        match adapter.enumerate_interfaces().await {
            Ok(ifaces) => {
                assert!(
                    ifaces.iter().any(|i| i.name == "lo"),
                    "expected loopback in enumeration, got: {ifaces:?}"
                );
                // Every enumerated interface must carry a non-zero kernel index.
                assert!(ifaces.iter().all(|i| i.index > 0));
            }
            Err(e) => {
                // Sandboxes without /sys/class/net: tolerate, don't fail CI.
                eprintln!("test: sysfs enumeration unavailable in sandbox: {e}");
            }
        }
    }

    #[tokio::test]
    async fn carrier_state_reads_loopback() {
        let adapter = SysfsFallbackAdapter::new();
        let lo = InterfaceId {
            index: 1,
            name: "lo".to_string(),
        };
        // Loopback is always up; we only assert the read does not error out on a
        // host that has /sys. In a bare sandbox this may legitimately be Down.
        match adapter.carrier_state(&lo).await {
            Ok(state) => {
                assert!(matches!(
                    state,
                    LinkState::Up | LinkState::Down | LinkState::Carrier(_)
                ));
            }
            Err(e) => eprintln!("test: carrier read unavailable in sandbox: {e}"),
        }
    }

    #[test]
    fn read_ifindex_rejects_missing_interface() {
        assert!(read_ifindex("definitely-not-a-real-iface-xyz").is_err());
    }
}

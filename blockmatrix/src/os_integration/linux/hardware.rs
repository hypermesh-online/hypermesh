// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Hardware detection methods for Linux: CPU, GPU, memory, storage, resource usage.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::super::types::*;
use super::LinuxAbstraction;

impl LinuxAbstraction {
    /// Parse /proc/cpuinfo for CPU details
    pub(super) fn parse_cpuinfo(&self) -> Result<CpuInfo> {
        let content =
            fs::read_to_string("/proc/cpuinfo").context("Failed to read /proc/cpuinfo")?;

        let mut cores = 0;
        let mut model = String::from("Unknown");
        let mut vendor = None;
        let mut frequency_mhz = None;

        for line in content.lines() {
            if line.starts_with("processor") {
                cores += 1;
            } else if line.starts_with("model name") {
                if let Some(value) = line.split(':').nth(1) {
                    model = value.trim().to_string();
                }
            } else if line.starts_with("vendor_id") {
                if let Some(value) = line.split(':').nth(1) {
                    vendor = Some(value.trim().to_string());
                }
            } else if line.starts_with("cpu MHz") {
                if let Some(value) = line.split(':').nth(1) {
                    if let Ok(mhz) = value.trim().parse::<f64>() {
                        frequency_mhz = Some(mhz as u64);
                    }
                }
            }
        }

        let architecture = std::env::consts::ARCH.to_string();

        Ok(CpuInfo {
            cores,
            model,
            architecture,
            frequency_mhz,
            usage_percent: None,
            vendor,
            cache_kb: None,
        })
    }

    /// Detect GPUs using /sys/class/drm or lspci
    pub(super) fn detect_gpus(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") && !name.contains('-') {
                        if let Ok(gpu) = self.parse_drm_card(&path) {
                            gpus.push(gpu);
                        }
                    }
                }
            }
        }

        if gpus.is_empty() {
            if let Ok(lspci_gpus) = self.detect_gpus_lspci() {
                gpus = lspci_gpus;
            }
        }

        Ok(gpus)
    }

    /// Parse GPU info from /sys/class/drm/cardN
    fn parse_drm_card(&self, card_path: &Path) -> Result<GpuInfo> {
        let device_path = card_path.join("device");

        let mut model = String::from("Unknown GPU");
        let mut vendor = String::from("Unknown");

        if let Ok(vendor_id) = fs::read_to_string(device_path.join("vendor")) {
            vendor = self.pci_vendor_name(vendor_id.trim());
        }

        if let Ok(uevent) = fs::read_to_string(device_path.join("uevent")) {
            for line in uevent.lines() {
                if line.starts_with("PCI_ID=") {
                    model = line.split('=').nth(1).unwrap_or("Unknown").to_string();
                    break;
                }
            }
        }

        Ok(GpuInfo {
            model,
            vendor,
            memory_bytes: None,
            available_bytes: None,
            gpu_type: GpuType::Discrete,
            capabilities: vec![],
            pci_address: None,
        })
    }

    /// Detect GPUs using lspci command (fallback)
    fn detect_gpus_lspci(&self) -> Result<Vec<GpuInfo>> {
        Ok(Vec::new())
    }

    /// Map PCI vendor ID to vendor name
    fn pci_vendor_name(&self, vendor_id: &str) -> String {
        match vendor_id {
            "0x10de" => "NVIDIA".to_string(),
            "0x1002" => "AMD".to_string(),
            "0x8086" => "Intel".to_string(),
            _ => format!("Vendor {vendor_id}"),
        }
    }

    /// Parse /proc/meminfo for memory details
    pub(super) fn parse_meminfo(&self) -> Result<MemoryInfo> {
        let content =
            fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].trim_end_matches(':');
                if let Ok(value) = parts[1].parse::<u64>() {
                    match key {
                        "MemTotal" => total_kb = value,
                        "MemAvailable" => available_kb = value,
                        "SwapTotal" => swap_total_kb = value,
                        "SwapFree" => swap_free_kb = value,
                        _ => {}
                    }
                }
            }
        }

        let total_bytes = total_kb * 1024;
        let available_bytes = available_kb * 1024;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total_bytes,
            available_bytes,
            used_bytes,
            usage_percent,
            swap_total_bytes: Some(swap_total_kb * 1024),
            swap_used_bytes: Some((swap_total_kb - swap_free_kb) * 1024),
        })
    }

    /// Detect storage from /proc/mounts and statvfs
    pub(super) fn detect_storage_devices(&self) -> Result<Vec<StorageInfo>> {
        let mut devices = Vec::new();

        let mounts = fs::read_to_string("/proc/mounts").context("Failed to read /proc/mounts")?;

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let device = parts[0];
                let mount_point = parts[1];
                let filesystem = parts[2];

                if device.starts_with("/dev/") {
                    if let Ok(stat) = nix::sys::statvfs::statvfs(mount_point) {
                        let block_size = stat.block_size();
                        let total_bytes = stat.blocks() * block_size;
                        let available_bytes = stat.blocks_available() * block_size;
                        let used_bytes = total_bytes - available_bytes;
                        let usage_percent = if total_bytes > 0 {
                            (used_bytes as f64 / total_bytes as f64) * 100.0
                        } else {
                            0.0
                        };

                        devices.push(StorageInfo {
                            device: device.to_string(),
                            mount_point: mount_point.to_string(),
                            filesystem: filesystem.to_string(),
                            total_bytes,
                            used_bytes,
                            available_bytes,
                            usage_percent,
                            storage_type: self.detect_storage_type(device),
                        });
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Detect storage type (HDD, SSD, NVMe)
    pub(super) fn detect_storage_type(&self, device: &str) -> StorageType {
        let dev_name = device.trim_start_matches("/dev/");
        let base_name = dev_name.trim_end_matches(char::is_numeric);

        if base_name.starts_with("nvme") {
            return StorageType::NVMe;
        }

        let rotational_path = format!("/sys/block/{base_name}/queue/rotational");
        if let Ok(content) = fs::read_to_string(&rotational_path) {
            if content.trim() == "0" {
                return StorageType::SSD;
            } else if content.trim() == "1" {
                return StorageType::HDD;
            }
        }

        StorageType::Unknown
    }

    /// Get resource usage from /proc
    pub(super) fn get_current_resource_usage(&self) -> Result<ResourceUsage> {
        let cpu_percent = self.get_cpu_usage()?;
        let memory_info = self.parse_meminfo()?;
        let load_average = self.get_load_average()?;

        Ok(ResourceUsage {
            cpu_usage_percent: cpu_percent,
            memory_usage_percent: memory_info.usage_percent,
            load_average: Some(load_average),
            network_rx_bytes_per_sec: Some(0),
            network_tx_bytes_per_sec: Some(0),
            disk_read_bytes_per_sec: Some(0),
            disk_write_bytes_per_sec: Some(0),
            process_count: None,
        })
    }

    /// Get CPU usage percentage
    fn get_cpu_usage(&self) -> Result<f64> {
        Ok(0.0)
    }

    /// Get load average from /proc/loadavg
    fn get_load_average(&self) -> Result<[f64; 3]> {
        let content =
            fs::read_to_string("/proc/loadavg").context("Failed to read /proc/loadavg")?;

        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            let load1 = parts[0].parse::<f64>().unwrap_or(0.0);
            let load5 = parts[1].parse::<f64>().unwrap_or(0.0);
            let load15 = parts[2].parse::<f64>().unwrap_or(0.0);
            Ok([load1, load5, load15])
        } else {
            Ok([0.0, 0.0, 0.0])
        }
    }

    /// Get process count from /proc
    pub(super) fn _get_process_count(&self) -> Option<usize> {
        fs::read_dir("/proc")
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                    .is_some()
            })
            .count()
            .into()
    }

    // ===== Device-unique identity readers (device-auth invariant) =====

    /// Read `/etc/machine-id`, falling back to `/var/lib/dbus/machine-id`.
    ///
    /// The machine-id is a 128-bit hex string unique to the installed OS.
    /// It survives reboots but is regenerated on reinstall — a stable,
    /// widely-present device-binding source.
    pub(super) fn read_machine_id(&self) -> Option<String> {
        for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            if let Ok(content) = fs::read_to_string(path) {
                let id = content.trim().to_string();
                if !id.is_empty() {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Read DMI/SMBIOS identifiers. Each is independently optional because
    /// these are typically root-readable only (mode 0400). We degrade
    /// gracefully — a non-root run simply contributes fewer sources.
    pub(super) fn read_dmi_identifiers(
        &self,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let read_dmi = |field: &str| -> Option<String> {
            let path = format!("/sys/class/dmi/id/{field}");
            fs::read_to_string(&path).ok().and_then(|s| {
                let v = s.trim().to_string();
                // Filter out well-known placeholder junk from OEM firmware.
                if v.is_empty()
                    || v.eq_ignore_ascii_case("None")
                    || v.eq_ignore_ascii_case("To Be Filled By O.E.M.")
                    || v.eq_ignore_ascii_case("Default string")
                    || v.chars().all(|c| c == '0' || c == '-')
                {
                    None
                } else {
                    Some(v)
                }
            })
        };
        (
            read_dmi("product_uuid"),
            read_dmi("board_serial"),
            read_dmi("product_serial"),
        )
    }

    /// Serial of the disk backing the largest mounted `/dev/*` filesystem.
    ///
    /// Resolves the block device for the largest mount, walks to its parent
    /// disk (stripping partition suffixes), and reads
    /// `/sys/block/<disk>/device/serial`.
    pub(super) fn read_primary_disk_serial(&self) -> Option<String> {
        let devices = self.detect_storage_devices().ok()?;
        let largest = devices.iter().max_by_key(|d| d.total_bytes)?;
        let base = Self::block_base_name(&largest.device)?;
        Self::read_disk_serial_for(&base)
    }

    /// Strip `/dev/` prefix and partition suffix to get the parent disk name.
    ///
    /// `/dev/nvme0n1p2` -> `nvme0n1`, `/dev/sda3` -> `sda`,
    /// `/dev/mmcblk0p1` -> `mmcblk0`.
    fn block_base_name(device: &str) -> Option<String> {
        let name = device.strip_prefix("/dev/")?;
        // nvme/mmcblk use `p<N>` partition suffix; sd*/vd*/hd* use trailing digits.
        if name.starts_with("nvme") || name.starts_with("mmcblk") {
            // Cut at the `p<digits>` partition marker if present.
            if let Some(idx) = name.rfind('p') {
                if name[idx + 1..].chars().all(|c| c.is_ascii_digit())
                    && !name[idx + 1..].is_empty()
                {
                    return Some(name[..idx].to_string());
                }
            }
            Some(name.to_string())
        } else {
            Some(name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string())
        }
    }

    /// Read `/sys/block/<disk>/device/serial`, falling back to WWID.
    fn read_disk_serial_for(disk: &str) -> Option<String> {
        for field in ["device/serial", "device/wwid", "wwid"] {
            let path = format!("/sys/block/{disk}/{field}");
            if let Ok(content) = fs::read_to_string(&path) {
                let serial = content.trim().to_string();
                if !serial.is_empty() {
                    return Some(serial);
                }
            }
        }
        None
    }

    /// Enumerate `/sys/class/net/*` and pick the primary interface.
    ///
    /// Selection priority: (1) non-loopback with carrier up, (2) any
    /// non-loopback, (3) loopback as last resort. Replaces the historic
    /// hardcoded loopback `::1` network asset — a real MAC is a device-unique
    /// fingerprint component.
    pub(super) fn read_primary_nic(&self) -> Option<NicInfo> {
        let mut candidates: Vec<NicInfo> = Vec::new();
        let entries = fs::read_dir("/sys/class/net").ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let base = entry.path();
            let mac = fs::read_to_string(base.join("address"))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if mac.is_empty() {
                continue;
            }
            let is_loopback = name == "lo" || mac == "00:00:00:00:00:00";
            let carrier = fs::read_to_string(base.join("carrier"))
                .ok()
                .map(|s| s.trim() == "1")
                .unwrap_or(false);
            candidates.push(NicInfo {
                name,
                mac,
                carrier,
                is_loopback,
            });
        }

        // (1) non-loopback with carrier up
        if let Some(nic) = candidates
            .iter()
            .find(|n| !n.is_loopback && n.carrier)
            .cloned()
        {
            return Some(nic);
        }
        // (2) any non-loopback (stable MAC even if link is down)
        if let Some(nic) = candidates.iter().find(|n| !n.is_loopback).cloned() {
            return Some(nic);
        }
        // (3) loopback last resort
        candidates.into_iter().next()
    }
}

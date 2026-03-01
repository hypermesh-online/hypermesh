// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GPU hardware detection and specification estimation.

use std::collections::HashMap;

use crate::os_integration::create_os_abstraction;

use super::types::*;

impl GpuAssetAdapter {
    /// Detect system GPU configuration using OS abstraction layer
    // STUB: GPU detection returns simulated devices, not actual hardware
    pub(crate) async fn detect_gpu_configuration() -> (u32, HashMap<u32, GpuDevice>) {
        // Use OS abstraction for real hardware detection
        match create_os_abstraction() {
            Ok(os) => {
                if let Ok(gpu_infos) = os.detect_gpu() {
                    if !gpu_infos.is_empty() {
                        let total_devices = gpu_infos.len() as u32;
                        let mut gpu_devices = HashMap::new();

                        for (device_id, gpu_info) in gpu_infos.iter().enumerate() {
                            let device_id = device_id as u32;
                            let compute_capability = Self::detect_compute_capability(
                                &gpu_info.model,
                                &gpu_info.capabilities,
                            );
                            let (vulkan_compute_units, base_clock, memory_clock) =
                                Self::estimate_gpu_specs(&gpu_info.model);
                            let (temperature, power) =
                                Self::read_gpu_sensors(&gpu_info.pci_address);

                            gpu_devices.insert(
                                device_id,
                                GpuDevice {
                                    device_id,
                                    device_name: gpu_info.model.clone(),
                                    compute_capability,
                                    total_memory_bytes: gpu_info.memory_bytes.unwrap_or(0),
                                    available_memory_bytes: gpu_info
                                        .available_bytes
                                        .unwrap_or(gpu_info.memory_bytes.unwrap_or(0)),
                                    vulkan_compute_units,
                                    nova_execution_units: vulkan_compute_units / 128,
                                    base_clock_mhz: base_clock,
                                    memory_clock_mhz: memory_clock,
                                    pci_bus_id: gpu_info
                                        .pci_address
                                        .clone()
                                        .unwrap_or_else(|| format!("Unknown:{device_id}")),
                                    status: GpuStatus::Available,
                                    allocated_to: None,
                                    temperature_celsius: temperature,
                                    power_watts: power,
                                },
                            );
                        }

                        tracing::info!(
                            "Detected {} GPU(s) via OS abstraction: {}",
                            total_devices,
                            gpu_infos
                                .iter()
                                .map(|g| g.model.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );

                        return (total_devices, gpu_devices);
                    } else {
                        tracing::info!("No GPUs detected via OS abstraction");
                        return (0, HashMap::new());
                    }
                } else {
                    tracing::warn!("Failed to detect GPUs via OS abstraction, using fallback");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create OS abstraction: {}, using fallback", e);
            }
        }

        // Fallback: report zero GPUs when detection fails (no fake hardware)
        tracing::info!("No GPUs detected (OS abstraction unavailable), reporting zero devices");
        (0, HashMap::new())
    }

    /// Detect compute capability from GPU model and capabilities
    pub(crate) fn detect_compute_capability(model: &str, capabilities: &[String]) -> String {
        let model_lower = model.to_lowercase();

        // NVIDIA GPUs
        if model_lower.contains("nvidia")
            || model_lower.contains("geforce")
            || model_lower.contains("rtx")
            || model_lower.contains("gtx")
        {
            if model_lower.contains("rtx 40")
                || model_lower.contains("4090")
                || model_lower.contains("4080")
            {
                return "8.9".to_string();
            }
            if model_lower.contains("rtx 30")
                || model_lower.contains("3090")
                || model_lower.contains("3080")
            {
                return "8.6".to_string();
            }
            if model_lower.contains("rtx 20")
                || model_lower.contains("2080")
                || model_lower.contains("2070")
            {
                return "7.5".to_string();
            }
            if model_lower.contains("gtx 10")
                || model_lower.contains("1080")
                || model_lower.contains("1070")
            {
                return "6.1".to_string();
            }
            return "7.0".to_string();
        }

        // AMD GPUs
        if model_lower.contains("amd")
            || model_lower.contains("radeon")
            || model_lower.contains("rx ")
        {
            if model_lower.contains("rx 7") {
                return "RDNA 3".to_string();
            }
            if model_lower.contains("rx 6") {
                return "RDNA 2".to_string();
            }
            if model_lower.contains("rx 5") {
                return "RDNA 1".to_string();
            }
            return "GCN 5".to_string();
        }

        // Intel GPUs
        if model_lower.contains("intel") || model_lower.contains("arc") {
            if model_lower.contains("arc") {
                return "Xe-HPG".to_string();
            }
            return "Xe".to_string();
        }

        // Check capabilities for OpenCL/Vulkan version
        for cap in capabilities {
            if cap.starts_with("opencl") || cap.starts_with("vulkan") {
                return cap.clone();
            }
        }

        "Unknown".to_string()
    }

    /// Estimate GPU specifications from model string
    pub(crate) fn estimate_gpu_specs(model: &str) -> (u32, u32, u32) {
        let model_lower = model.to_lowercase();

        if model_lower.contains("4080") {
            return (9728, 2505, 11400);
        }
        if model_lower.contains("4090") {
            return (16384, 2520, 10080);
        }
        if model_lower.contains("3090") {
            return (10496, 1695, 9750);
        }
        if model_lower.contains("3080") {
            return (8704, 1710, 9500);
        }
        if model_lower.contains("7900 xtx") {
            return (6144, 2500, 10000);
        }
        if model_lower.contains("6900 xt") {
            return (5120, 2250, 8000);
        }
        if model_lower.contains("arc a770") {
            return (4096, 2400, 8560);
        }

        // Default reasonable specs
        (4096, 1800, 7000)
    }

    /// Read GPU temperature and power consumption from sensors
    pub(crate) fn read_gpu_sensors(pci_address: &Option<String>) -> (Option<f32>, Option<f32>) {
        #[cfg(target_os = "linux")]
        {
            if let Some(_pci_addr) = pci_address {
                if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
                    for entry in entries.flatten() {
                        let card_path = entry.path();
                        let hwmon_path = card_path.join("device/hwmon");

                        if let Ok(hwmon_entries) = std::fs::read_dir(&hwmon_path) {
                            for hwmon_entry in hwmon_entries.flatten() {
                                let temp_file = hwmon_entry.path().join("temp1_input");
                                let temperature = std::fs::read_to_string(&temp_file)
                                    .ok()
                                    .and_then(|s| s.trim().parse::<i32>().ok())
                                    .map(|millidegrees| millidegrees as f32 / 1000.0);

                                let power_file = hwmon_entry.path().join("power1_average");
                                let power = std::fs::read_to_string(&power_file)
                                    .ok()
                                    .and_then(|s| s.trim().parse::<i32>().ok())
                                    .map(|microwatts| microwatts as f32 / 1000000.0);

                                if temperature.is_some() || power.is_some() {
                                    return (temperature, power);
                                }
                            }
                        }
                    }
                }
            }
        }

        (None, None)
    }
}

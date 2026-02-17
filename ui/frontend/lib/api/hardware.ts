// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Hardware Detection API Client

import { apiRequest } from '../api';

// Hardware capabilities types
export interface CpuInfo {
  physical_cores: number;
  logical_cores: number;
  model_name: string;
  frequency_mhz: number;
  vendor: string;
  architecture: string;
  usage_percent: number;
  temperature_celsius?: number;
}

export interface MemoryInfo {
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
  speed_mhz?: number;
  memory_type?: string;
}

export interface StorageInfo {
  mount_point: string;
  device_name: string;
  filesystem_type: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  is_ssd: boolean;
  is_removable: boolean;
}

export interface NetworkInterface {
  name: string;
  mac_address: string;
  ip_addresses: string[];
  speed_mbps: number;
  is_wireless: boolean;
  is_virtual: boolean;
  bytes_received: number;
  bytes_transmitted: number;
  packets_received: number;
  packets_transmitted: number;
  current_bandwidth_mbps: number;
}

export interface SystemInfo {
  os_name: string;
  os_version: string;
  kernel_version: string;
  hostname: string;
  uptime_seconds: number;
  boot_time: number;
  process_count: number;
}

export interface HardwareCapabilities {
  cpu: CpuInfo;
  memory: MemoryInfo;
  storage: StorageInfo[];
  network: NetworkInterface[];
  system: SystemInfo;
  detected_at: number;
}

// Resource allocation types
export interface ResourceAllocationDetail {
  total: number;
  allocated: number;
  used: number;
  available: number;
  allocation_percent: number;
  usage_percent: number;
}

export interface ResourceAllocation {
  cpu: ResourceAllocationDetail;
  memory: ResourceAllocationDetail;
  storage: ResourceAllocationDetail;
  network: ResourceAllocationDetail;
}

// Sharing capabilities types
export interface ResourceLimits {
  max_cpu_cores: number;
  max_memory_bytes: number;
  max_storage_bytes: number;
  max_network_mbps: number;
}

export interface SharingMode {
  name: string;
  description: string;
  is_active: boolean;
  resource_limits: ResourceLimits;
}

export interface SharingCapabilities {
  max_cpu_cores: number;
  max_memory_bytes: number;
  max_storage_bytes: number;
  max_network_mbps: number;
  recommended_cpu_cores: number;
  recommended_memory_bytes: number;
  recommended_storage_bytes: number;
  recommended_network_mbps: number;
  available_modes: SharingMode[];
}

// API response wrapper
export interface HardwareApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: number;
}

// Hardware detection API client
export class HardwareApi {
  private baseUrl: string;

  constructor(baseUrl: string = '') {
    this.baseUrl = baseUrl;
  }

  /**
   * Get hardware capabilities
   */
  async getHardwareCapabilities(): Promise<HardwareCapabilities> {
    const response = await apiRequest<HardwareApiResponse<HardwareCapabilities>>(
      `${this.baseUrl}/api/v1/system/hardware`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );

    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to get hardware capabilities');
    }

    return response.data;
  }

  /**
   * Get network capabilities
   */
  async getNetworkCapabilities(): Promise<{ interfaces: NetworkInterface[]; detected_at: number }> {
    const response = await apiRequest<HardwareApiResponse<{ interfaces: NetworkInterface[]; detected_at: number }>>(
      `${this.baseUrl}/api/v1/system/network`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );

    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to get network capabilities');
    }

    return response.data;
  }

  /**
   * Get resource allocation status
   */
  async getResourceAllocation(): Promise<ResourceAllocation> {
    const response = await apiRequest<HardwareApiResponse<ResourceAllocation>>(
      `${this.baseUrl}/api/v1/system/allocation`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );

    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to get resource allocation');
    }

    return response.data;
  }

  /**
   * Get sharing capabilities
   */
  async getSharingCapabilities(): Promise<SharingCapabilities> {
    const response = await apiRequest<HardwareApiResponse<SharingCapabilities>>(
      `${this.baseUrl}/api/v1/system/capabilities`,
      {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );

    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to get sharing capabilities');
    }

    return response.data;
  }

  /**
   * Refresh hardware detection
   */
  async refreshHardware(): Promise<{ success: boolean; message: string; timestamp: number }> {
    const response = await apiRequest<{ success: boolean; message: string; timestamp: number }>(
      `${this.baseUrl}/api/v1/system/refresh`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );

    return response;
  }
}

// Default export for convenience
export const hardwareApi = new HardwareApi();

// Helper functions for formatting
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatBandwidth(mbps: number): string {
  if (mbps >= 1000) {
    return `${(mbps / 1000).toFixed(1)} Gbps`;
  }
  return `${mbps} Mbps`;
}
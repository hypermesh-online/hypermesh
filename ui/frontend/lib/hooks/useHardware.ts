// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Hardware Detection React Hook

import { useState, useEffect, useCallback } from 'react';
import {
  hardwareApi,
  HardwareCapabilities,
  ResourceAllocation,
  SharingCapabilities,
  formatBytes,
  formatBandwidth
} from '../api/hardware';

export interface HardwareData {
  capabilities?: HardwareCapabilities;
  allocation?: ResourceAllocation;
  sharing?: SharingCapabilities;
  isLoading: boolean;
  error?: string;
  lastUpdate?: Date;
}

export interface FormattedHardwareData {
  cpu: {
    cores: number;
    model: string;
    usage: number;
  };
  memory: {
    total: string;
    used: string;
    available: string;
    usage: number;
  };
  storage: {
    total: string;
    used: string;
    available: string;
    usage: number;
    devices: number;
  };
  network: {
    interfaces: number;
    totalBandwidth: string;
    activeInterfaces: number;
  };
  system: {
    os: string;
    hostname: string;
    uptime: string;
  };
}

export function useHardware(refreshInterval?: number) {
  const [data, setData] = useState<HardwareData>({
    isLoading: true,
  });

  const fetchHardwareData = useCallback(async () => {
    try {
      const [capabilities, allocation, sharing] = await Promise.all([
        hardwareApi.getHardwareCapabilities(),
        hardwareApi.getResourceAllocation(),
        hardwareApi.getSharingCapabilities(),
      ]);

      setData({
        capabilities,
        allocation,
        sharing,
        isLoading: false,
        lastUpdate: new Date(),
      });
    } catch (error) {
      console.error('Failed to fetch hardware data:', error);
      setData((prev) => ({
        ...prev,
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch hardware data',
      }));
    }
  }, []);

  const refresh = useCallback(async () => {
    setData((prev) => ({ ...prev, isLoading: true }));
    await hardwareApi.refreshHardware();
    await fetchHardwareData();
  }, [fetchHardwareData]);

  useEffect(() => {
    fetchHardwareData();

    if (refreshInterval) {
      const interval = setInterval(fetchHardwareData, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchHardwareData, refreshInterval]);

  // Format hardware data for display
  const formatted: FormattedHardwareData | undefined = data.capabilities
    ? {
        cpu: {
          cores: data.capabilities.cpu.logical_cores,
          model: data.capabilities.cpu.model_name,
          usage: data.capabilities.cpu.usage_percent,
        },
        memory: {
          total: formatBytes(data.capabilities.memory.total_bytes),
          used: formatBytes(data.capabilities.memory.used_bytes),
          available: formatBytes(data.capabilities.memory.available_bytes),
          usage: data.capabilities.memory.usage_percent,
        },
        storage: {
          total: formatBytes(
            data.capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0)
          ),
          used: formatBytes(
            data.capabilities.storage.reduce((sum, disk) => sum + disk.used_bytes, 0)
          ),
          available: formatBytes(
            data.capabilities.storage.reduce((sum, disk) => sum + disk.available_bytes, 0)
          ),
          usage:
            (data.capabilities.storage.reduce((sum, disk) => sum + disk.used_bytes, 0) /
              data.capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0)) *
            100,
          devices: data.capabilities.storage.length,
        },
        network: {
          interfaces: data.capabilities.network.length,
          totalBandwidth: formatBandwidth(
            data.capabilities.network.reduce((sum, iface) => sum + iface.speed_mbps, 0)
          ),
          activeInterfaces: data.capabilities.network.filter(
            (iface) => iface.bytes_received > 0 || iface.bytes_transmitted > 0
          ).length,
        },
        system: {
          os: `${data.capabilities.system.os_name} ${data.capabilities.system.os_version}`,
          hostname: data.capabilities.system.hostname,
          uptime: formatUptime(data.capabilities.system.uptime_seconds),
        },
      }
    : undefined;

  return {
    ...data,
    formatted,
    refresh,
  };
}

// Helper function to format uptime
function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  const parts = [];
  if (days > 0) parts.push(`${days}d`);
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);

  return parts.length > 0 ? parts.join(' ') : 'Just started';
}

// Hook for real-time resource monitoring
export function useResourceMonitor(interval: number = 2000) {
  const [allocation, setAllocation] = useState<ResourceAllocation | undefined>();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();

  useEffect(() => {
    const fetchAllocation = async () => {
      try {
        const data = await hardwareApi.getResourceAllocation();
        setAllocation(data);
        setIsLoading(false);
      } catch (err) {
        console.error('Failed to fetch resource allocation:', err);
        setError(err instanceof Error ? err.message : 'Failed to fetch allocation');
        setIsLoading(false);
      }
    };

    fetchAllocation();
    const timer = setInterval(fetchAllocation, interval);

    return () => clearInterval(timer);
  }, [interval]);

  return { allocation, isLoading, error };
}

// Hook for sharing capabilities
export function useSharingCapabilities() {
  const [capabilities, setCapabilities] = useState<SharingCapabilities | undefined>();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();

  useEffect(() => {
    hardwareApi
      .getSharingCapabilities()
      .then((data) => {
        setCapabilities(data);
        setIsLoading(false);
      })
      .catch((err) => {
        console.error('Failed to fetch sharing capabilities:', err);
        setError(err instanceof Error ? err.message : 'Failed to fetch capabilities');
        setIsLoading(false);
      });
  }, []);

  return { capabilities, isLoading, error };
}
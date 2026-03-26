// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useEffect, useState, useCallback } from 'react';
import { hyperMeshAPI } from '../services/HyperMeshAPI';
import { stoqAPI } from '../services/STOQAPI';
import type { PerformanceMetrics as StoqPerformanceMetrics } from '../services/STOQAPI';

// Local type definitions (previously from StoqDataProvider)
export interface SystemStatus {
  services: {
    [key: string]: {
      status: 'healthy' | 'degraded' | 'offline';
      uptime: number;
      lastHealthCheck: string;
    };
  };
  overall: 'healthy' | 'degraded' | 'critical';
}

export interface PerformanceMetrics {
  throughput: {
    download: number;
    upload: number;
    efficiency: number;
  };
  latency: {
    rtt: number;
    packetLoss: number;
  };
  timestamp: string;
}

export interface Asset {
  id: string;
  type: 'CPU' | 'GPU' | 'Memory' | 'Storage';
  status: 'available' | 'allocated' | 'maintenance';
  proxyAddress: string;
  stateProof?: string;
}

export interface AssetAllocation {
  id: string;
  assetId: string;
  status: 'active' | 'pending' | 'completed';
  allocatedAt: string;
}

export interface ByzantineDetection {
  nodeId: string;
  behaviour: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  status: 'detected' | 'investigating' | 'resolved';
  detectedAt: string;
}

export interface QUICConnection {
  id: string;
  status: 'active' | 'idle' | 'closed';
  throughput: number;
  latency: number;
  createdAt: string;
}

/**
 * Hook for system status via API polling
 */
export function useSystemStatus(autoRefresh = false) {
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const [hypermeshHealth, stoqHealth] = await Promise.allSettled([
        hyperMeshAPI.getSystemStatus(),
        stoqAPI.getSystemHealth()
      ]);

      const services: SystemStatus['services'] = {};

      if (hypermeshHealth.status === 'fulfilled') {
        services['hypermesh'] = {
          status: hypermeshHealth.value.status === 'healthy' ? 'healthy' : 'degraded',
          uptime: hypermeshHealth.value.uptime,
          lastHealthCheck: new Date().toISOString()
        };
      }

      if (stoqHealth.status === 'fulfilled') {
        services['stoq'] = {
          status: stoqHealth.value.status === 'optimal' ? 'healthy' : stoqHealth.value.status === 'good' ? 'healthy' : 'degraded',
          uptime: stoqHealth.value.uptime,
          lastHealthCheck: new Date().toISOString()
        };
      }

      const statuses = Object.values(services).map(s => s.status);
      const overall = statuses.includes('offline') ? 'critical' as const
        : statuses.includes('degraded') ? 'degraded' as const
        : 'healthy' as const;

      setSystemStatus({ services, overall });
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get system status');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    if (autoRefresh) {
      const interval = setInterval(fetchStatus, 30000);
      return () => clearInterval(interval);
    }
  }, [fetchStatus, autoRefresh]);

  return {
    systemStatus,
    isLoading,
    error,
    isHealthy: systemStatus?.overall === 'healthy',
    hasWarnings: systemStatus?.overall === 'degraded',
    isCritical: systemStatus?.overall === 'critical',
    refetch: fetchStatus
  };
}

/**
 * Hook for performance metrics via API polling
 */
export function usePerformanceMetrics(serviceType?: string, timeRange?: string, autoRefresh = false) {
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchMetrics = useCallback(async () => {
    try {
      const results = await stoqAPI.getPerformanceMetrics();
      if (results.length > 0) {
        const latest = results[results.length - 1];
        setMetrics({
          throughput: {
            download: latest.throughput.download,
            upload: latest.throughput.upload,
            efficiency: latest.throughput.efficiency
          },
          latency: {
            rtt: latest.latency.rtt,
            packetLoss: latest.latency.packetLoss
          },
          timestamp: latest.timestamp
        });
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get performance metrics');
    } finally {
      setIsLoading(false);
    }
  }, [serviceType, timeRange]);

  useEffect(() => {
    fetchMetrics();
    if (autoRefresh) {
      const interval = setInterval(fetchMetrics, 10000);
      return () => clearInterval(interval);
    }
  }, [fetchMetrics, autoRefresh]);

  const throughputAchievement = metrics ? (metrics.throughput.download / 40000) * 100 : 0;
  const performanceGrade = throughputAchievement >= 90 ? 'A+' :
                          throughputAchievement >= 80 ? 'A' :
                          throughputAchievement >= 70 ? 'B' :
                          throughputAchievement >= 60 ? 'C' : 'D';

  const bottlenecks = metrics && metrics.latency.rtt > 100 ? ['High Latency'] : [];

  return {
    latestMetrics: metrics,
    throughputAchievement,
    performanceGrade,
    bottlenecks,
    isLoading,
    error,
    refetch: fetchMetrics
  };
}

/**
 * Hook for assets via API polling
 */
export function useAssets() {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAssets = useCallback(async () => {
    try {
      const result = await hyperMeshAPI.getAssets();
      const mapped: Asset[] = result.map(a => ({
        id: a.id,
        type: (a.type as Asset['type']) || 'CPU',
        status: (a.status as Asset['status']) || 'available',
        proxyAddress: a.location?.address || '',
        stateProof: undefined
      }));
      setAssets(mapped);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get assets');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => { fetchAssets(); }, [fetchAssets]);

  return {
    assets,
    availableAssets: assets.filter(a => a.status === 'available'),
    allocatedAssets: assets.filter(a => a.status === 'allocated'),
    isLoading,
    error,
    refetch: fetchAssets
  };
}

/**
 * Hook for allocations via API polling
 */
export function useAllocations() {
  const [allocations, setAllocations] = useState<AssetAllocation[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAllocations = useCallback(async () => {
    try {
      const result = await hyperMeshAPI.getAllocations();
      const mapped: AssetAllocation[] = result.map(a => ({
        id: a.id,
        assetId: a.assetId,
        status: (a.status as AssetAllocation['status']) || 'active',
        allocatedAt: a.startTime || new Date().toISOString()
      }));
      setAllocations(mapped);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get allocations');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => { fetchAllocations(); }, [fetchAllocations]);

  return {
    allocations,
    activeAllocations: allocations.filter(a => a.status === 'active'),
    isLoading,
    error,
    refetch: fetchAllocations
  };
}

/**
 * Hook for Byzantine detections via API polling
 */
export function useByzantineDetections() {
  const [detections, setDetections] = useState<ByzantineDetection[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchDetections = useCallback(async () => {
    try {
      const result = await hyperMeshAPI.getByzantineDetections();
      const mapped: ByzantineDetection[] = result.map(d => ({
        nodeId: d.nodeId,
        behaviour: d.behaviour,
        severity: (d.severity as ByzantineDetection['severity']) || 'low',
        status: (d.status as ByzantineDetection['status']) || 'detected',
        detectedAt: d.detectedAt
      }));
      setDetections(mapped);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get Byzantine detections');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => { fetchDetections(); }, [fetchDetections]);

  return {
    detections,
    criticalDetections: detections.filter(d => d.severity === 'critical'),
    unresolved: detections.filter(d => d.status !== 'resolved'),
    isLoading,
    error,
    refetch: fetchDetections
  };
}

/**
 * Hook for QUIC connections via API polling
 */
export function useQUICConnections() {
  const [connections, setConnections] = useState<QUICConnection[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchConnections = useCallback(async () => {
    try {
      const result = await stoqAPI.getConnections();
      const mapped: QUICConnection[] = result.map(c => ({
        id: c.id,
        status: c.status === 'connected' ? 'active' as const : c.status === 'disconnected' ? 'closed' as const : 'idle' as const,
        throughput: 0,
        latency: 0,
        createdAt: c.establishedAt || new Date().toISOString()
      }));
      setConnections(mapped);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get QUIC connections');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => { fetchConnections(); }, [fetchConnections]);

  return {
    connections,
    activeConnections: connections.filter(c => c.status === 'active'),
    isLoading,
    error,
    refetch: fetchConnections
  };
}

/**
 * No-op hook (STOQ Data Provider removed)
 */
export function useStoqDataProvider() {
  return {
    isConnected: false,
    isInitializing: false,
    error: 'STOQ Data Provider removed - use HTTP API hooks instead',
    initialize: async (_cert: string) => {},
    disconnect: async () => {}
  };
}

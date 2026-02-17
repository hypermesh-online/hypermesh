// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useEffect, useState } from 'react';
import { stoqDataProvider, SystemStatus, PerformanceMetrics, Asset, AssetAllocation, ByzantineDetection, QUICConnection } from '../StoqDataProvider';

/**
 * Hook for real-time system status via STOQ protocol
 */
export function useSystemStatus(autoRefresh = false) {
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.systemStatus$.subscribe({
      next: (status) => {
        setSystemStatus(status);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get system status');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestSystemStatus();
    }

    return () => subscription.unsubscribe();
  }, []);

  const isHealthy = systemStatus?.overall === 'healthy';
  const hasWarnings = systemStatus?.overall === 'degraded';
  const isCritical = systemStatus?.overall === 'critical';

  return {
    systemStatus,
    isLoading,
    error,
    isHealthy,
    hasWarnings,
    isCritical,
    refetch: () => stoqDataProvider.requestSystemStatus()
  };
}

/**
 * Hook for real-time performance metrics via STOQ protocol
 */
export function usePerformanceMetrics(serviceType?: string, timeRange?: string, autoRefresh = false) {
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.performanceMetrics$.subscribe({
      next: (performanceMetrics) => {
        setMetrics(performanceMetrics);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get performance metrics');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestPerformanceMetrics();
    }

    return () => subscription.unsubscribe();
  }, [serviceType, timeRange]);

  // Calculate derived metrics
  const latestMetrics = metrics;
  const throughputAchievement = metrics ? (metrics.throughput.download / 40000) * 100 : 0;
  const performanceGrade = throughputAchievement >= 90 ? 'A+' :
                          throughputAchievement >= 80 ? 'A' :
                          throughputAchievement >= 70 ? 'B' :
                          throughputAchievement >= 60 ? 'C' : 'D';
  
  const bottlenecks = metrics && metrics.latency.rtt > 100 ? ['High Latency'] : [];

  return {
    latestMetrics,
    throughputAchievement,
    performanceGrade,
    bottlenecks,
    isLoading,
    error,
    refetch: () => stoqDataProvider.requestPerformanceMetrics()
  };
}

/**
 * Hook for real-time assets via STOQ protocol
 */
export function useAssets() {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.assets$.subscribe({
      next: (assetsData) => {
        setAssets(assetsData);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get assets');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestAssets();
    }

    return () => subscription.unsubscribe();
  }, []);

  const availableAssets = assets.filter(asset => asset.status === 'available');
  const allocatedAssets = assets.filter(asset => asset.status === 'allocated');

  return {
    assets,
    availableAssets,
    allocatedAssets,
    isLoading,
    error,
    refetch: () => stoqDataProvider.requestAssets()
  };
}

/**
 * Hook for real-time allocations via STOQ protocol
 */
export function useAllocations() {
  const [allocations, setAllocations] = useState<AssetAllocation[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.allocations$.subscribe({
      next: (allocationsData) => {
        setAllocations(allocationsData);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get allocations');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestAllocations();
    }

    return () => subscription.unsubscribe();
  }, []);

  const activeAllocations = allocations.filter(allocation => allocation.status === 'active');

  return {
    allocations,
    activeAllocations,
    isLoading,
    error,
    refetch: () => stoqDataProvider.requestAllocations()
  };
}

/**
 * Hook for real-time Byzantine detections via STOQ protocol
 */
export function useByzantineDetections() {
  const [detections, setDetections] = useState<ByzantineDetection[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.byzantineDetections$.subscribe({
      next: (detectionsData) => {
        setDetections(detectionsData);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get Byzantine detections');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestByzantineDetections();
    }

    return () => subscription.unsubscribe();
  }, []);

  const criticalDetections = detections.filter(detection => detection.severity === 'critical');
  const unresolved = detections.filter(detection => detection.status !== 'resolved');

  return {
    detections,
    criticalDetections,
    unresolved,
    isLoading,
    error,
    refetch: () => stoqDataProvider.requestByzantineDetections()
  };
}

/**
 * Hook for real-time QUIC connections via STOQ protocol
 */
export function useQUICConnections() {
  const [connections, setConnections] = useState<QUICConnection[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subscription = stoqDataProvider.quicConnections$.subscribe({
      next: (connectionsData) => {
        setConnections(connectionsData);
        setIsLoading(false);
        setError(null);
      },
      error: (err) => {
        setError(err.message || 'Failed to get QUIC connections');
        setIsLoading(false);
      }
    });

    // Get initial data
    if (stoqDataProvider.isConnected()) {
      stoqDataProvider.requestQUICConnections();
    }

    return () => subscription.unsubscribe();
  }, []);

  const activeConnections = connections.filter(connection => connection.status === 'active');

  return {
    connections,
    activeConnections,
    isLoading,
    error,
    refetch: () => stoqDataProvider.requestQUICConnections()
  };
}

/**
 * Hook to initialize STOQ data provider with certificate
 */
export function useStoqDataProvider() {
  const [isConnected, setIsConnected] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const initialize = async (certificatePem: string) => {
    if (isInitializing) return;
    
    setIsInitializing(true);
    setError(null);

    try {
      await stoqDataProvider.initialize(certificatePem);
      setIsConnected(true);
      console.log('✅ STOQ Data Provider ready for dashboard streaming');
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to initialize STOQ data provider';
      setError(errorMessage);
      console.error('❌ STOQ Data Provider initialization failed:', errorMessage);
    } finally {
      setIsInitializing(false);
    }
  };

  const disconnect = async () => {
    try {
      await stoqDataProvider.disconnect();
      setIsConnected(false);
    } catch (err) {
      console.error('Error disconnecting STOQ data provider:', err);
    }
  };

  // Check connection status periodically
  useEffect(() => {
    const interval = setInterval(() => {
      const connected = stoqDataProvider.isConnected();
      if (connected !== isConnected) {
        setIsConnected(connected);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [isConnected]);

  return {
    isConnected,
    isInitializing,
    error,
    initialize,
    disconnect
  };
}
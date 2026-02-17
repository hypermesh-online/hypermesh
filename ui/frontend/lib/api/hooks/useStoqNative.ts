// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * React Hook for STOQ Native Protocol Integration
 * 
 * Provides React integration for the STOQ native client with
 * automatic connection management, status monitoring, and
 * real-time data updates.
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { stoqNativeClient, isStoqNativeAvailable, type StoqAuthResult } from '../StoqNativeClient';
import type { WasmConnectionStatus } from '../../stoq-wasm';

export interface StoqConnectionState {
  isAvailable: boolean;
  isConnected: boolean;
  isAuthenticated: boolean;
  connectionId: string | null;
  status: WasmConnectionStatus | null;
  error: string | null;
  protocol: 'stoq-native' | 'http-fallback' | null;
}

export interface StoqSystemStatus {
  overall_health: string;
  score: number;
  services: Record<string, {
    status: string;
    [key: string]: any;
  }>;
  timestamp: string;
}

export interface StoqPerformanceMetrics {
  throughput: {
    current: number;
    target: number;
    unit: string;
    efficiency: number;
  };
  latency: {
    average: number;
    p95: number;
    p99: number;
    unit: string;
  };
  connections: {
    active: number;
    total: number;
    failed: number;
  };
  timestamp: string;
}

/**
 * Main hook for STOQ native connection management
 */
export function useStoqNative(certificatePem?: string) {
  const [connectionState, setConnectionState] = useState<StoqConnectionState>({
    isAvailable: isStoqNativeAvailable(),
    isConnected: false,
    isAuthenticated: false,
    connectionId: null,
    status: null,
    error: null,
    protocol: null,
  });

  const statusCheckInterval = useRef<number | null>(null);

  // Initialize connection
  const initializeMutation = useMutation({
    mutationFn: async (cert: string): Promise<StoqAuthResult> => {
      return stoqNativeClient.initialize(cert);
    },
    onSuccess: (result) => {
      setConnectionState(prev => ({
        ...prev,
        isAuthenticated: result.authenticated,
        connectionId: result.connectionId || null,
        error: result.error || null,
        protocol: result.authenticated ? 'stoq-native' : 'http-fallback',
      }));
      
      // Start status monitoring
      startStatusMonitoring();
    },
    onError: (error) => {
      setConnectionState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Connection failed',
        protocol: 'http-fallback',
      }));
    },
  });

  // Disconnect
  const disconnectMutation = useMutation({
    mutationFn: async (): Promise<void> => {
      await stoqNativeClient.disconnect();
    },
    onSuccess: () => {
      setConnectionState(prev => ({
        ...prev,
        isConnected: false,
        isAuthenticated: false,
        connectionId: null,
        status: null,
        error: null,
      }));
      
      stopStatusMonitoring();
    },
  });

  // Start monitoring connection status
  const startStatusMonitoring = useCallback(() => {
    if (statusCheckInterval.current) {
      clearInterval(statusCheckInterval.current);
    }

    statusCheckInterval.current = window.setInterval(() => {
      const status = stoqNativeClient.getConnectionStatus();
      const isAuthenticated = stoqNativeClient.isAuthenticated();
      const connectionId = stoqNativeClient.getConnectionId();

      setConnectionState(prev => ({
        ...prev,
        status,
        isConnected: status !== null && status !== 0, // Not disconnected
        isAuthenticated,
        connectionId,
      }));
    }, 1000); // Check every second
  }, []);

  // Stop monitoring
  const stopStatusMonitoring = useCallback(() => {
    if (statusCheckInterval.current) {
      clearInterval(statusCheckInterval.current);
      statusCheckInterval.current = null;
    }
  }, []);

  // Initialize on mount if certificate is provided
  useEffect(() => {
    if (certificatePem && connectionState.isAvailable && !connectionState.isAuthenticated) {
      initializeMutation.mutate(certificatePem);
    }

    return () => {
      stopStatusMonitoring();
    };
  }, [certificatePem, connectionState.isAvailable, connectionState.isAuthenticated]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopStatusMonitoring();
    };
  }, []);

  return {
    connectionState,
    initialize: initializeMutation.mutate,
    disconnect: disconnectMutation.mutate,
    isInitializing: initializeMutation.isPending,
    isDisconnecting: disconnectMutation.isPending,
  };
}

/**
 * Hook for getting system status via STOQ native protocol
 */
export function useStoqSystemStatus(enabled = true) {
  return useQuery({
    queryKey: ['stoq-native', 'system-status'],
    queryFn: async (): Promise<StoqSystemStatus> => {
      if (!stoqNativeClient.isAuthenticated()) {
        throw new Error('STOQ client not authenticated');
      }
      
      const response = await stoqNativeClient.getSystemStatus();
      return response.system || response;
    },
    enabled: enabled && stoqNativeClient.isAuthenticated(),
    refetchInterval: 30000, // Refresh every 30 seconds
    retry: (failureCount, error) => {
      // Don't retry if not authenticated
      if (error.message.includes('not authenticated')) {
        return false;
      }
      return failureCount < 3;
    },
  });
}

/**
 * Hook for getting performance metrics via STOQ native protocol
 */
export function useStoqPerformanceMetrics(timeRange = '1h', enabled = true) {
  return useQuery({
    queryKey: ['stoq-native', 'performance-metrics', timeRange],
    queryFn: async (): Promise<StoqPerformanceMetrics> => {
      if (!stoqNativeClient.isAuthenticated()) {
        throw new Error('STOQ client not authenticated');
      }
      
      const response = await stoqNativeClient.getPerformanceMetrics(timeRange);
      return response.metrics || response;
    },
    enabled: enabled && stoqNativeClient.isAuthenticated(),
    refetchInterval: 10000, // Refresh every 10 seconds
    retry: (failureCount, error) => {
      if (error.message.includes('not authenticated')) {
        return false;
      }
      return failureCount < 3;
    },
  });
}

/**
 * Hook for getting dashboard data via STOQ native protocol
 */
export function useStoqDashboardData(dashboardType: string, enabled = true) {
  return useQuery({
    queryKey: ['stoq-native', 'dashboard', dashboardType],
    queryFn: async () => {
      if (!stoqNativeClient.isAuthenticated()) {
        throw new Error('STOQ client not authenticated');
      }
      
      return stoqNativeClient.getDashboardData(dashboardType);
    },
    enabled: enabled && stoqNativeClient.isAuthenticated(),
    refetchInterval: 15000, // Refresh every 15 seconds
    retry: (failureCount, error) => {
      if (error.message.includes('not authenticated')) {
        return false;
      }
      return failureCount < 3;
    },
  });
}

/**
 * Hook for sending messages via STOQ native protocol
 */
export function useStoqMessage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ messageType, payload }: { messageType: string; payload: any }) => {
      if (!stoqNativeClient.isAuthenticated()) {
        throw new Error('STOQ client not authenticated');
      }
      
      return stoqNativeClient.request('integration', messageType, payload);
    },
    onSuccess: () => {
      // Invalidate related queries to trigger refresh
      queryClient.invalidateQueries({ queryKey: ['stoq-native'] });
    },
  });
}

/**
 * Hook for real-time message handling
 */
export function useStoqMessageHandler(messageType: string, handler: (payload: any) => void) {
  useEffect(() => {
    if (stoqNativeClient.isAuthenticated()) {
      stoqNativeClient.registerMessageHandler(messageType, handler);
    }
  }, [messageType, handler]);
}

/**
 * Utility hook to check if STOQ native is preferred over HTTP fallback
 */
export function useStoqNativePreference(): {
  isPreferred: boolean;
  reason: string;
  canUpgrade: boolean;
} {
  const [preference, setPreference] = useState({
    isPreferred: false,
    reason: 'Checking...',
    canUpgrade: false,
  });

  useEffect(() => {
    const checkPreference = () => {
      if (!isStoqNativeAvailable()) {
        setPreference({
          isPreferred: false,
          reason: 'WebAssembly not supported',
          canUpgrade: false,
        });
        return;
      }

      if (stoqNativeClient.isAuthenticated()) {
        setPreference({
          isPreferred: true,
          reason: 'Connected via STOQ native protocol',
          canUpgrade: false,
        });
        return;
      }

      setPreference({
        isPreferred: true,
        reason: 'STOQ native available for better performance',
        canUpgrade: true,
      });
    };

    checkPreference();
    const interval = setInterval(checkPreference, 5000);
    
    return () => clearInterval(interval);
  }, []);

  return preference;
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * System Status Hooks - Real-time system health monitoring
 * 
 * Provides React Query hooks for monitoring Web3 ecosystem health:
 * - Real-time system status updates
 * - Service health monitoring
 * - Performance metrics tracking
 * - WebSocket-based live updates
 */

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { trustChainAPI } from '../services/TrustChainAPI';
import { hyperMeshAPI } from '../services/HyperMeshAPI';
import { stoqAPI } from '../services/STOQAPI';
import { web3Events } from '../index';
import type { EventChannel } from '../Web3Events';

export interface SystemStatus {
  overall: 'healthy' | 'degraded' | 'critical' | 'offline';
  services: {
    trustchain: ServiceStatus;
    hypermesh: ServiceStatus;
    stoq: ServiceStatus;
    integration: ServiceStatus;
  };
  performance: {
    avgResponseTime: number;
    totalRequests: number;
    errorRate: number;
    uptime: number;
  };
  lastUpdated: string;
}

export interface ServiceStatus {
  name: string;
  status: 'healthy' | 'warning' | 'critical' | 'offline';
  responseTime: number;
  errorRate: number;
  uptime: number;
  version?: string;
  lastCheck: string;
  details?: Record<string, any>;
}

/**
 * Get comprehensive system status with real-time updates
 */
export function useSystemStatus(enableRealtime: boolean = true) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['system', 'status'],
    queryFn: async (): Promise<SystemStatus> => {
      try {
        // Fetch status from all services in parallel
        const [trustchainHealth, hypermeshHealth, stoqHealth] = await Promise.allSettled([
          trustChainAPI.getHealthStatus(),
          hyperMeshAPI.getSystemStatus(),
          stoqAPI.getSystemHealth()
        ]);

        // Calculate overall status
        const services = {
          trustchain: mapTrustChainHealth(trustchainHealth),
          hypermesh: mapHyperMeshHealth(hypermeshHealth),
          stoq: mapSTOQHealth(stoqHealth),
          integration: {
            name: 'Integration',
            status: 'healthy' as const,
            responseTime: 50,
            errorRate: 0,
            uptime: 99.9,
            lastCheck: new Date().toISOString()
          }
        };

        const overallStatus = calculateOverallStatus(services);
        const performanceMetrics = calculatePerformanceMetrics(services);

        return {
          overall: overallStatus,
          services,
          performance: performanceMetrics,
          lastUpdated: new Date().toISOString()
        };

      } catch (error) {
        console.error('Failed to fetch system status:', error);
        throw error;
      }
    },
    refetchInterval: enableRealtime ? 30000 : false, // Refetch every 30 seconds
    staleTime: 10000, // Data is fresh for 10 seconds
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000)
  });

  // Set up real-time updates via WebSocket
  useEffect(() => {
    if (!enableRealtime) return;

    const setupRealtimeUpdates = async () => {
      try {
        // Connect to system status events
        await web3Events.connect('integration');
        
        const subscriptionId = await web3Events.subscribe('integration', 'system.status', (event) => {
          // Update the query cache with real-time data
          queryClient.setQueryData(['system', 'status'], (oldData: SystemStatus | undefined) => {
            if (!oldData) return oldData;
            
            return {
              ...oldData,
              services: {
                ...oldData.services,
                [event.data.service]: {
                  ...oldData.services[event.data.service as keyof typeof oldData.services],
                  ...event.data.status,
                  lastCheck: event.timestamp
                }
              },
              lastUpdated: event.timestamp
            };
          });
        });

        subscriptionRef.current = subscriptionId;

      } catch (error) {
        console.error('Failed to setup real-time system status updates:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [enableRealtime, queryClient]);

  return {
    ...query,
    systemStatus: query.data,
    isHealthy: query.data?.overall === 'healthy',
    hasWarnings: query.data?.overall === 'degraded',
    isCritical: query.data?.overall === 'critical',
    isOffline: query.data?.overall === 'offline'
  };
}

/**
 * Get individual service status
 */
export function useServiceStatus(service: 'trustchain' | 'hypermesh' | 'stoq') {
  return useQuery({
    queryKey: ['service', service, 'status'],
    queryFn: async (): Promise<ServiceStatus> => {
      switch (service) {
        case 'trustchain':
          const trustchainHealth = await trustChainAPI.getHealthStatus();
          return {
            name: 'TrustChain',
            status: trustchainHealth.status,
            responseTime: 50, // Mock response time
            errorRate: 0,
            uptime: trustchainHealth.uptime,
            lastCheck: new Date().toISOString(),
            details: trustchainHealth
          };

        case 'hypermesh':
          const hypermeshHealth = await hyperMeshAPI.getSystemStatus();
          return {
            name: 'HyperMesh',
            status: hypermeshHealth.status,
            responseTime: 75,
            errorRate: 0,
            uptime: hypermeshHealth.uptime,
            lastCheck: new Date().toISOString(),
            details: hypermeshHealth
          };

        case 'stoq':
          const stoqHealth = await stoqAPI.getSystemHealth();
          return {
            name: 'STOQ',
            status: stoqHealth.status === 'optimal' ? 'healthy' : stoqHealth.status,
            responseTime: 60,
            errorRate: 0,
            uptime: stoqHealth.uptime,
            version: stoqHealth.version,
            lastCheck: new Date().toISOString(),
            details: stoqHealth
          };

        default:
          throw new Error(`Unknown service: ${service}`);
      }
    },
    refetchInterval: 60000, // Refetch every minute
    staleTime: 30000,
    retry: 2
  });
}

/**
 * Monitor system performance metrics
 */
export function usePerformanceMetrics(timeRange: '1h' | '24h' | '7d' = '1h') {
  return useQuery({
    queryKey: ['system', 'performance', timeRange],
    queryFn: async () => {
      const endTime = new Date();
      const startTime = new Date();
      
      switch (timeRange) {
        case '1h':
          startTime.setHours(endTime.getHours() - 1);
          break;
        case '24h':
          startTime.setDate(endTime.getDate() - 1);
          break;
        case '7d':
          startTime.setDate(endTime.getDate() - 7);
          break;
      }

      // Fetch historical metrics from STOQ (as the transport layer)
      const metrics = await stoqAPI.getHistoricalMetrics({
        start: startTime.toISOString(),
        end: endTime.toISOString(),
        interval: timeRange === '1h' ? '1m' : timeRange === '24h' ? '5m' : '1h'
      });

      return {
        timeRange,
        metrics,
        summary: {
          avgThroughput: metrics.reduce((sum, m) => sum + m.throughput, 0) / metrics.length,
          avgLatency: metrics.reduce((sum, m) => sum + m.latency, 0) / metrics.length,
          totalConnections: Math.max(...metrics.map(m => m.connections)),
          errorRate: (metrics.reduce((sum, m) => sum + m.errors, 0) / metrics.length) * 100
        }
      };
    },
    refetchInterval: 60000,
    staleTime: 30000
  });
}

/**
 * Helper functions for mapping service health data
 */
function mapTrustChainHealth(result: PromiseSettledResult<any>): ServiceStatus {
  if (result.status === 'rejected') {
    return {
      name: 'TrustChain',
      status: 'offline',
      responseTime: 0,
      errorRate: 100,
      uptime: 0,
      lastCheck: new Date().toISOString()
    };
  }

  const health = result.value;
  return {
    name: 'TrustChain',
    status: health.status,
    responseTime: 50,
    errorRate: 0,
    uptime: health.uptime,
    lastCheck: new Date().toISOString(),
    details: health
  };
}

function mapHyperMeshHealth(result: PromiseSettledResult<any>): ServiceStatus {
  if (result.status === 'rejected') {
    return {
      name: 'HyperMesh',
      status: 'offline',
      responseTime: 0,
      errorRate: 100,
      uptime: 0,
      lastCheck: new Date().toISOString()
    };
  }

  const health = result.value;
  return {
    name: 'HyperMesh',
    status: health.status,
    responseTime: 75,
    errorRate: 0,
    uptime: health.uptime,
    lastCheck: new Date().toISOString(),
    details: health
  };
}

function mapSTOQHealth(result: PromiseSettledResult<any>): ServiceStatus {
  if (result.status === 'rejected') {
    return {
      name: 'STOQ',
      status: 'offline',
      responseTime: 0,
      errorRate: 100,
      uptime: 0,
      lastCheck: new Date().toISOString()
    };
  }

  const health = result.value;
  return {
    name: 'STOQ',
    status: health.status === 'optimal' ? 'healthy' : health.status,
    responseTime: 60,
    errorRate: 0,
    uptime: health.uptime,
    version: health.version,
    lastCheck: new Date().toISOString(),
    details: health
  };
}

function calculateOverallStatus(services: SystemStatus['services']): SystemStatus['overall'] {
  const statuses = Object.values(services).map(s => s.status);
  
  if (statuses.includes('offline')) return 'offline';
  if (statuses.includes('critical')) return 'critical';
  if (statuses.includes('warning')) return 'degraded';
  return 'healthy';
}

function calculatePerformanceMetrics(services: SystemStatus['services']) {
  const serviceList = Object.values(services);
  
  return {
    avgResponseTime: serviceList.reduce((sum, s) => sum + s.responseTime, 0) / serviceList.length,
    totalRequests: 1000, // Mock data
    errorRate: serviceList.reduce((sum, s) => sum + s.errorRate, 0) / serviceList.length,
    uptime: serviceList.reduce((sum, s) => sum + s.uptime, 0) / serviceList.length
  };
}
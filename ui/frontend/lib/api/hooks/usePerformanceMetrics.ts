// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Performance Metrics Hooks - STOQ monitoring and optimization
 * 
 * Provides React Query hooks for STOQ performance monitoring:
 * - Real-time QUIC connection monitoring
 * - Performance metrics tracking (targeting 40 Gbps)
 * - Network quality analysis
 * - Transport optimization recommendations
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  stoqAPI,
  QUICConnection,
  PerformanceMetrics,
  NetworkQuality,
  TransportOptimization,
  ConnectionPool,
  StreamAnalytics
} from '../services/STOQAPI';

/**
 * Get QUIC connections with real-time updates
 */
export function useQUICConnections() {
  const query = useQuery({
    queryKey: ['stoq', 'connections'],
    queryFn: () => stoqAPI.getConnections(),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });

  return {
    ...query,
    connections: Array.isArray(query.data) ? query.data : [],
    activeConnections: Array.isArray(query.data) ? query.data.filter(conn => conn.status === 'connected') : [],
    connectingConnections: Array.isArray(query.data) ? query.data.filter(conn => conn.status === 'connecting') : [],
    errorConnections: Array.isArray(query.data) ? query.data.filter(conn => conn.status === 'error') : []
  };
}

/**
 * Get specific QUIC connection details
 */
export function useQUICConnection(connectionId: string) {
  return useQuery({
    queryKey: ['stoq', 'connection', connectionId],
    queryFn: () => stoqAPI.getConnection(connectionId),
    enabled: !!connectionId,
    staleTime: 15000,
    refetchInterval: 30000,
    retry: 2
  });
}

/**
 * Create new QUIC connection
 */
export function useCreateConnection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: {
      remoteAddress: string;
      port: number;
      serverName?: string;
      alpn?: string[];
      initialMaxStreams?: number;
    }) => stoqAPI.createConnection(config),
    onSuccess: (newConnection) => {
      // Update connections list
      queryClient.setQueryData(['stoq', 'connections'], (oldData: QUICConnection[] | undefined) => {
        return Array.isArray(oldData) ? [...oldData, newConnection] : [newConnection];
      });
    }
  });
}

/**
 * Close QUIC connection
 */
export function useCloseConnection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ connectionId, reason }: { connectionId: string; reason?: string }) =>
      stoqAPI.closeConnection(connectionId, reason),
    onSuccess: (_, variables) => {
      // Remove connection from cache
      queryClient.setQueryData(['stoq', 'connections'], (oldData: QUICConnection[] | undefined) => {
        return Array.isArray(oldData) ? oldData.filter(conn => conn.id !== variables.connectionId) : [];
      });
      
      // Remove specific connection cache
      queryClient.removeQueries({ queryKey: ['stoq', 'connection', variables.connectionId] });
    }
  });
}

/**
 * Get real-time performance metrics with high-frequency updates
 */
export function usePerformanceMetrics(
  connectionId?: string,
  timeRange?: { start: string; end: string },
  enableRealtime: boolean = true
) {
  const query = useQuery({
    queryKey: ['stoq', 'performance', connectionId, timeRange],
    queryFn: () => stoqAPI.getPerformanceMetrics(connectionId, timeRange),
    staleTime: enableRealtime ? 5000 : 30000,
    refetchInterval: enableRealtime ? 10000 : 60000,
    retry: 2
  });

  const metricsArray = Array.isArray(query.data) ? query.data : [];
  const latestMetrics = metricsArray.length > 0 ? metricsArray[metricsArray.length - 1] : undefined;
  const throughputTarget = 40000; // 40 Gbps in Mbps

  return {
    ...query,
    metrics: metricsArray,
    latestMetrics,
    throughputAchievement: latestMetrics ? (latestMetrics.throughput.download / throughputTarget) * 100 : 0,
    performanceGrade: calculatePerformanceGrade(latestMetrics, throughputTarget),
    bottlenecks: identifyBottlenecks(latestMetrics),
    trends: calculateTrends(metricsArray)
  };
}

/**
 * Get network quality assessment
 */
export function useNetworkQuality(connectionId?: string) {
  return useQuery({
    queryKey: ['stoq', 'quality', connectionId],
    queryFn: () => stoqAPI.getNetworkQuality(connectionId),
    staleTime: 60000, // 1 minute
    refetchInterval: 120000, // 2 minutes
    retry: 2
  });
}

/**
 * Get transport optimization suggestions
 */
export function useTransportOptimizations(connectionId: string) {
  return useQuery({
    queryKey: ['stoq', 'optimizations', connectionId],
    queryFn: () => stoqAPI.getOptimizations(connectionId),
    enabled: !!connectionId,
    staleTime: 300000, // 5 minutes
    retry: 2
  });
}

/**
 * Apply optimization settings
 */
export function useApplyOptimization() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ connectionId, optimization }: {
      connectionId: string;
      optimization: { type: string; settings: Record<string, any> };
    }) => stoqAPI.applyOptimization(connectionId, optimization),
    onSuccess: (result, variables) => {
      if (result.applied) {
        // Invalidate optimizations and performance metrics
        queryClient.invalidateQueries({ 
          queryKey: ['stoq', 'optimizations', variables.connectionId] 
        });
        queryClient.invalidateQueries({ 
          queryKey: ['stoq', 'performance', variables.connectionId] 
        });
      }
    }
  });
}

/**
 * Get connection pools
 */
export function useConnectionPools() {
  return useQuery({
    queryKey: ['stoq', 'pools'],
    queryFn: () => stoqAPI.getConnectionPools(),
    staleTime: 60000,
    refetchInterval: 300000, // 5 minutes
    retry: 2
  });
}

/**
 * Create connection pool
 */
export function useCreateConnectionPool() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: {
      name: string;
      maxConnections: number;
      strategy: ConnectionPool['strategy'];
      targets: Array<{
        address: string;
        port: number;
        weight?: number;
      }>;
    }) => stoqAPI.createConnectionPool(config),
    onSuccess: (newPool) => {
      queryClient.setQueryData(['stoq', 'pools'], (oldData: ConnectionPool[] | undefined) => {
        return Array.isArray(oldData) ? [...oldData, newPool] : [newPool];
      });
    }
  });
}

/**
 * Get stream analytics
 */
export function useStreamAnalytics(connectionId?: string, streamId?: string) {
  return useQuery({
    queryKey: ['stoq', 'streams', connectionId, streamId],
    queryFn: () => stoqAPI.getStreamAnalytics(connectionId, streamId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });
}

/**
 * Get historical performance data
 */
export function useHistoricalMetrics(timeRange: {
  start: string;
  end: string;
  interval: '1m' | '5m' | '15m' | '1h' | '1d';
}) {
  return useQuery({
    queryKey: ['stoq', 'historical', timeRange],
    queryFn: () => stoqAPI.getHistoricalMetrics(timeRange),
    staleTime: 300000, // 5 minutes
    retry: 2
  });
}

/**
 * Run connection diagnostics
 */
export function useRunDiagnostics() {
  return useMutation({
    mutationFn: (connectionId: string) => stoqAPI.runDiagnostics(connectionId)
  });
}

/**
 * Run performance benchmark
 */
export function useRunBenchmark() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (test: {
      type: 'throughput' | 'latency' | 'stream_multiplexing' | 'connection_establishment';
      duration: number;
      targets?: string[];
      parameters?: Record<string, any>;
    }) => stoqAPI.runBenchmark(test),
    onSuccess: (result) => {
      // Cache benchmark result
      queryClient.setQueryData(['stoq', 'benchmark', result.testId], result);
    }
  });
}

/**
 * Get benchmark results
 */
export function useBenchmarkResult(testId: string) {
  return useQuery({
    queryKey: ['stoq', 'benchmark', testId],
    queryFn: () => stoqAPI.getBenchmarkResult(testId),
    enabled: !!testId,
    refetchInterval: (query) => {
      // Poll while test is running
      return query.state.data?.status === 'running' ? 5000 : false;
    },
    retry: 2
  });
}

/**
 * Update transport settings
 */
export function useUpdateTransportSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: {
      maxConcurrentStreams?: number;
      initialMaxData?: number;
      initialMaxStreamData?: number;
      idleTimeout?: number;
      keepAlive?: boolean;
      congestionControl?: 'bbr' | 'cubic' | 'reno';
    }) => stoqAPI.updateTransportSettings(settings),
    onSuccess: () => {
      // Invalidate transport config and connections
      queryClient.invalidateQueries({ queryKey: ['stoq', 'config'] });
      queryClient.invalidateQueries({ queryKey: ['stoq', 'connections'] });
    }
  });
}

/**
 * Get current transport configuration
 */
export function useTransportSettings() {
  return useQuery({
    queryKey: ['stoq', 'config', 'transport'],
    queryFn: () => stoqAPI.getTransportSettings(),
    staleTime: 300000, // 5 minutes
    retry: 2
  });
}

/**
 * Helper functions for performance analysis
 */
function calculatePerformanceGrade(metrics: PerformanceMetrics | undefined, target: number): string {
  if (!metrics) return 'N/A';
  
  const throughputPercent = (metrics.throughput.download / target) * 100;
  const latencyScore = Math.max(0, 100 - metrics.latency.rtt); // Lower latency is better
  const reliabilityScore = Math.max(0, 100 - (metrics.latency.packetLoss * 10)); // Lower packet loss is better
  
  const overallScore = (throughputPercent * 0.5) + (latencyScore * 0.3) + (reliabilityScore * 0.2);
  
  if (overallScore >= 90) return 'A+';
  if (overallScore >= 80) return 'A';
  if (overallScore >= 70) return 'B';
  if (overallScore >= 60) return 'C';
  if (overallScore >= 50) return 'D';
  return 'F';
}

function identifyBottlenecks(metrics: PerformanceMetrics | undefined): string[] {
  if (!metrics) return [];
  
  const bottlenecks: string[] = [];
  
  if (metrics.throughput.efficiency < 25) {
    bottlenecks.push('Low throughput efficiency - check network capacity');
  }
  
  if (metrics.latency.rtt > 100) {
    bottlenecks.push('High round-trip time - network latency issue');
  }
  
  if (metrics.latency.packetLoss > 1) {
    bottlenecks.push('Packet loss detected - network reliability issue');
  }
  
  if (metrics.congestion.retransmissions > metrics.congestion.inFlight * 0.1) {
    bottlenecks.push('High retransmission rate - congestion control issue');
  }
  
  if (metrics.streams.activeStreams < metrics.streams.maxStreams * 0.1) {
    bottlenecks.push('Low stream utilization - consider stream multiplexing optimization');
  }
  
  return bottlenecks;
}

function calculateTrends(metrics: PerformanceMetrics[]) {
  if (metrics.length < 2) return null;
  
  const recent = metrics.slice(-10); // Last 10 measurements
  const older = metrics.slice(-20, -10); // Previous 10 measurements
  
  if (recent.length === 0 || older.length === 0) return null;
  
  const recentAvg = {
    throughput: recent.reduce((sum, m) => sum + m.throughput.download, 0) / recent.length,
    latency: recent.reduce((sum, m) => sum + m.latency.rtt, 0) / recent.length,
    packetLoss: recent.reduce((sum, m) => sum + m.latency.packetLoss, 0) / recent.length
  };
  
  const olderAvg = {
    throughput: older.reduce((sum, m) => sum + m.throughput.download, 0) / older.length,
    latency: older.reduce((sum, m) => sum + m.latency.rtt, 0) / older.length,
    packetLoss: older.reduce((sum, m) => sum + m.latency.packetLoss, 0) / older.length
  };
  
  return {
    throughput: ((recentAvg.throughput - olderAvg.throughput) / olderAvg.throughput) * 100,
    latency: ((recentAvg.latency - olderAvg.latency) / olderAvg.latency) * 100,
    packetLoss: ((recentAvg.packetLoss - olderAvg.packetLoss) / Math.max(olderAvg.packetLoss, 0.001)) * 100
  };
}
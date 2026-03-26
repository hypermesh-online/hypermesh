// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * STOQ API - QUIC transport performance monitoring and connection management
 * 
 * Provides typed interface for STOQ service operations:
 * - QUIC connection management and monitoring
 * - Real-time performance metrics (targeting 40 Gbps)
 * - Transport layer optimization
 * - Network quality analysis
 */

import { get, post, put } from '../../api';

export interface QUICConnection {
  id: string;
  localAddress: string;
  remoteAddress: string;
  status: 'connecting' | 'connected' | 'disconnecting' | 'disconnected' | 'error';
  protocol: 'QUIC/HTTP3';
  version: string;
  establishedAt?: string;
  disconnectedAt?: string;
  lastActivity: string;
  streams: {
    total: number;
    active: number;
    closed: number;
  };
  encryption: {
    cipher: string;
    keyExchange: string;
    certificateFingerprint: string;
  };
}

export interface PerformanceMetrics {
  connectionId: string;
  timestamp: string;
  throughput: {
    upload: number;    // Mbps
    download: number;  // Mbps
    target: number;    // Target: 40 Gbps = 40,000 Mbps
    efficiency: number; // Percentage of target achieved
  };
  latency: {
    rtt: number;       // Round-trip time in ms
    jitter: number;    // Jitter in ms
    packetLoss: number; // Percentage
  };
  congestion: {
    windowSize: number;
    inFlight: number;
    retransmissions: number;
    congestionEvents: number;
  };
  streams: {
    activeStreams: number;
    maxStreams: number;
    streamCreationRate: number;
    streamCompletionRate: number;
  };
}

export interface NetworkQuality {
  overall: 'excellent' | 'good' | 'fair' | 'poor' | 'critical';
  score: number; // 0-100
  factors: {
    bandwidth: number;
    latency: number;
    stability: number;
    errorRate: number;
  };
  recommendations: string[];
  bottlenecks: Array<{
    component: string;
    severity: 'low' | 'medium' | 'high' | 'critical';
    description: string;
    mitigation?: string;
  }>;
}

export interface TransportOptimization {
  connectionId: string;
  optimizations: Array<{
    type: 'congestion_control' | 'flow_control' | 'stream_multiplexing' | 'connection_migration';
    applied: boolean;
    impact: number; // Performance improvement percentage
    timestamp: string;
  }>;
  currentSettings: {
    maxStreams: number;
    initialWindowSize: number;
    maxDatagramSize: number;
    idleTimeout: number;
    keepAliveInterval: number;
  };
  recommendations: Array<{
    setting: string;
    currentValue: any;
    recommendedValue: any;
    expectedImprovement: number;
  }>;
}

export interface ConnectionPool {
  id: string;
  name: string;
  maxConnections: number;
  activeConnections: number;
  queuedRequests: number;
  strategy: 'round_robin' | 'least_connections' | 'weighted' | 'latency_based';
  health: {
    healthy: number;
    degraded: number;
    failed: number;
  };
  performance: {
    averageThroughput: number;
    averageLatency: number;
    successRate: number;
  };
}

export interface StreamAnalytics {
  streamId: string;
  connectionId: string;
  type: 'unidirectional' | 'bidirectional';
  status: 'active' | 'completed' | 'reset' | 'failed';
  startTime: string;
  endTime?: string;
  bytesTransferred: {
    sent: number;
    received: number;
  };
  performance: {
    throughput: number;
    duration: number;
    efficiency: number;
  };
  errors: Array<{
    code: string;
    message: string;
    timestamp: string;
  }>;
}

export interface STOQSystemHealth {
  status: 'optimal' | 'good' | 'degraded' | 'critical';
  version: string;
  uptime: number;
  performance: {
    globalThroughput: number; // Current global throughput
    targetThroughput: number; // 40 Gbps target
    achievementPercentage: number;
    bottlenecks: string[];
  };
  connections: {
    total: number;
    active: number;
    failed: number;
    averagePerformance: number;
  };
  resources: {
    cpuUsage: number;
    memoryUsage: number;
    networkUtilization: number;
    diskIo: number;
  };
  alerts: Array<{
    level: 'info' | 'warning' | 'error' | 'critical';
    message: string;
    timestamp: string;
    acknowledged: boolean;
  }>;
}

export class STOQAPI {
  /**
   * Get all QUIC connections
   */
  async getConnections(): Promise<QUICConnection[]> {
    return get<QUICConnection[]>('/api/v1/stoq/connections');
  }

  /**
   * Get specific connection details
   */
  async getConnection(connectionId: string): Promise<QUICConnection> {
    return get<QUICConnection>(`/api/v1/stoq/connections/${connectionId}`);
  }

  /**
   * Establish new QUIC connection
   */
  async createConnection(config: {
    remoteAddress: string;
    port: number;
    serverName?: string;
    alpn?: string[];
    initialMaxStreams?: number;
  }): Promise<QUICConnection> {
    return post<QUICConnection>('/api/v1/stoq/connections', config);
  }

  /**
   * Close connection
   */
  async closeConnection(connectionId: string, reason?: string): Promise<void> {
    await post(`/api/v1/stoq/connections/${connectionId}/close`, { reason });
  }

  /**
   * Get real-time performance metrics
   */
  async getPerformanceMetrics(connectionId?: string, timeRange?: {
    start: string;
    end: string;
  }): Promise<PerformanceMetrics[]> {
    const params = new URLSearchParams();
    if (connectionId) params.append('connectionId', connectionId);
    if (timeRange) {
      params.append('start', timeRange.start);
      params.append('end', timeRange.end);
    }

    const endpoint = params.toString() ? `/api/v1/stoq/metrics/performance?${params}` : '/api/v1/stoq/metrics/performance';
    return get<PerformanceMetrics[]>(endpoint);
  }

  /**
   * Get network quality assessment
   */
  async getNetworkQuality(connectionId?: string): Promise<NetworkQuality> {
    const endpoint = connectionId
      ? `/api/v1/stoq/analysis/quality?connectionId=${connectionId}`
      : '/api/v1/stoq/analysis/quality';
    return get<NetworkQuality>(endpoint);
  }

  /**
   * Get transport optimization suggestions
   */
  async getOptimizations(connectionId: string): Promise<TransportOptimization> {
    return get<TransportOptimization>(`/api/v1/stoq/optimization/${connectionId}`);
  }

  /**
   * Apply optimization settings
   */
  async applyOptimization(connectionId: string, optimization: {
    type: string;
    settings: Record<string, any>;
  }): Promise<{ applied: boolean; impact?: number; error?: string }> {
    return post(`/api/v1/stoq/optimization/${connectionId}/apply`, optimization);
  }

  /**
   * Get connection pools
   */
  async getConnectionPools(): Promise<ConnectionPool[]> {
    return get<ConnectionPool[]>('/api/v1/stoq/pools');
  }

  /**
   * Create connection pool
   */
  async createConnectionPool(config: {
    name: string;
    maxConnections: number;
    strategy: ConnectionPool['strategy'];
    targets: Array<{
      address: string;
      port: number;
      weight?: number;
    }>;
  }): Promise<ConnectionPool> {
    return post<ConnectionPool>('/api/v1/stoq/pools', config);
  }

  /**
   * Update connection pool
   */
  async updateConnectionPool(poolId: string, updates: Partial<ConnectionPool>): Promise<ConnectionPool> {
    return put<ConnectionPool>(`/api/v1/stoq/pools/${poolId}`, updates);
  }

  /**
   * Get stream analytics
   */
  async getStreamAnalytics(connectionId?: string, streamId?: string): Promise<StreamAnalytics[]> {
    const params = new URLSearchParams();
    if (connectionId) params.append('connectionId', connectionId);
    if (streamId) params.append('streamId', streamId);

    const endpoint = params.toString() ? `/api/v1/stoq/analytics/streams?${params}` : '/api/v1/stoq/analytics/streams';
    return get<StreamAnalytics[]>(endpoint);
  }

  /**
   * Get historical performance data
   */
  async getHistoricalMetrics(timeRange: {
    start: string;
    end: string;
    interval: '1m' | '5m' | '15m' | '1h' | '1d';
  }): Promise<Array<{
    timestamp: string;
    throughput: number;
    latency: number;
    connections: number;
    errors: number;
  }>> {
    return post('/api/v1/stoq/metrics/historical', timeRange);
  }

  /**
   * Run connection diagnostics
   */
  async runDiagnostics(connectionId: string): Promise<{
    connectionId: string;
    tests: Array<{
      name: string;
      status: 'pass' | 'fail' | 'warning';
      result: any;
      recommendations?: string[];
    }>;
    overall: 'healthy' | 'issues' | 'critical';
    executedAt: string;
  }> {
    return post(`/api/v1/stoq/diagnostics/${connectionId}`);
  }

  /**
   * Get STOQ system health
   */
  async getSystemHealth(): Promise<STOQSystemHealth> {
    return get<STOQSystemHealth>('/api/v1/stoq/system/health');
  }

  /**
   * Get performance benchmarks
   */
  async runBenchmark(test: {
    type: 'throughput' | 'latency' | 'stream_multiplexing' | 'connection_establishment';
    duration: number;
    targets?: string[];
    parameters?: Record<string, any>;
  }): Promise<{
    testId: string;
    type: string;
    status: 'running' | 'completed' | 'failed';
    results?: {
      throughput?: number;
      latency?: number;
      connectionTime?: number;
      streamCount?: number;
      efficiency?: number;
    };
    startTime: string;
    endTime?: string;
  }> {
    return post('/api/v1/stoq/benchmark', test);
  }

  /**
   * Get benchmark results
   */
  async getBenchmarkResult(testId: string): Promise<{
    testId: string;
    status: string;
    results: any;
    report: string;
    completedAt: string;
  }> {
    return get(`/api/v1/stoq/benchmark/${testId}`);
  }

  /**
   * Configure transport settings globally
   */
  async updateTransportSettings(settings: {
    maxConcurrentStreams?: number;
    initialMaxData?: number;
    initialMaxStreamData?: number;
    idleTimeout?: number;
    keepAlive?: boolean;
    congestionControl?: 'bbr' | 'cubic' | 'reno';
  }): Promise<{ applied: boolean; errors?: string[] }> {
    return put('/api/v1/stoq/config/transport', settings);
  }

  /**
   * Get current transport configuration
   */
  async getTransportSettings(): Promise<{
    current: Record<string, any>;
    defaults: Record<string, any>;
    optimized: Record<string, any>;
    recommendations: Array<{
      setting: string;
      reason: string;
      impact: string;
    }>;
  }> {
    return get('/api/v1/stoq/config/transport');
  }
}

// Singleton instance
export const stoqAPI = new STOQAPI();
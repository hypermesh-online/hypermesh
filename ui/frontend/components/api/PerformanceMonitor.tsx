// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Performance Monitor - Real-time STOQ performance monitoring component
 * 
 * Displays live performance metrics from STOQ transport layer:
 * - Real-time throughput monitoring (targeting 40 Gbps)
 * - Latency and network quality metrics
 * - Connection health and optimization suggestions
 * - Performance trends and bottleneck analysis
 */

import React, { useState } from 'react';
import { usePerformanceMetrics, useQUICConnections, useNetworkQuality } from '../../lib/api';
import { Badge } from '../ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';

export function PerformanceMonitor() {
  const [selectedConnection, setSelectedConnection] = useState<string | undefined>();
  
  const { connections, activeConnections, isLoading: connectionsLoading } = useQUICConnections();
  const { 
    metrics, 
    latestMetrics, 
    throughputAchievement, 
    performanceGrade, 
    bottlenecks, 
    trends,
    isLoading: metricsLoading 
  } = usePerformanceMetrics(selectedConnection, undefined, true);
  
  const { data: networkQuality, isLoading: qualityLoading } = useNetworkQuality(selectedConnection);

  if (connectionsLoading || metricsLoading) {
    return (
      <div className="bg-gray-900 border border-gray-700 rounded-lg p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-gray-700 rounded w-48"></div>
          <div className="grid grid-cols-3 gap-4">
            <div className="h-20 bg-gray-700 rounded"></div>
            <div className="h-20 bg-gray-700 rounded"></div>
            <div className="h-20 bg-gray-700 rounded"></div>
          </div>
          <div className="h-32 bg-gray-700 rounded"></div>
        </div>
      </div>
    );
  }

  const formatThroughput = (mbps: number) => {
    if (mbps >= 1000) {
      return `${(mbps / 1000).toFixed(2)} Gbps`;
    }
    return `${mbps.toFixed(1)} Mbps`;
  };

  const getThroughputColor = (achievement: number) => {
    if (achievement >= 90) return 'text-green-400';
    if (achievement >= 75) return 'text-blue-400';
    if (achievement >= 50) return 'text-yellow-400';
    if (achievement >= 25) return 'text-orange-400';
    return 'text-red-400';
  };

  const getGradeColor = (grade: string) => {
    if (grade.startsWith('A')) return 'text-green-400';
    if (grade === 'B') return 'text-blue-400';
    if (grade === 'C') return 'text-yellow-400';
    if (grade === 'D') return 'text-orange-400';
    return 'text-red-400';
  };

  return (
    <div className="bg-gray-900 border border-gray-700 rounded-lg p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-white text-lg font-medium">Performance Monitor</h3>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-xs">
            {activeConnections.length} Active Connections
          </Badge>
          {latestMetrics && (
            <Badge 
              variant="outline" 
              className={`text-xs ${getGradeColor(performanceGrade)}`}
            >
              Grade: {performanceGrade}
            </Badge>
          )}
        </div>
      </div>

      <Tabs defaultValue="overview" className="w-full">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="connections">Connections</TabsTrigger>
          <TabsTrigger value="quality">Quality</TabsTrigger>
          <TabsTrigger value="bottlenecks">Analysis</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          {/* Key Metrics */}
          <div className="grid grid-cols-3 gap-4">
            <div className="bg-gray-800 rounded-lg p-4">
              <div className="text-gray-400 text-sm mb-1">Throughput</div>
              <div className={`text-2xl font-mono font-bold ${getThroughputColor(throughputAchievement)}`}>
                {latestMetrics ? formatThroughput(latestMetrics.throughput.download) : 'N/A'}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                Target: 40 Gbps ({throughputAchievement.toFixed(1)}%)
              </div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <div className="text-gray-400 text-sm mb-1">Latency</div>
              <div className="text-2xl font-mono font-bold text-blue-400">
                {latestMetrics ? `${latestMetrics.latency.rtt.toFixed(1)} ms` : 'N/A'}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                Jitter: {latestMetrics ? `${latestMetrics.latency.jitter.toFixed(1)} ms` : 'N/A'}
              </div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <div className="text-gray-400 text-sm mb-1">Efficiency</div>
              <div className="text-2xl font-mono font-bold text-purple-400">
                {latestMetrics ? `${latestMetrics.throughput.efficiency.toFixed(1)}%` : 'N/A'}
              </div>
              <div className="text-xs text-gray-500 mt-1">
                Loss: {latestMetrics ? `${latestMetrics.latency.packetLoss.toFixed(2)}%` : 'N/A'}
              </div>
            </div>
          </div>

          {/* Performance Trends */}
          {trends && (
            <div className="bg-gray-800 rounded-lg p-4">
              <h4 className="text-white font-medium mb-3">Performance Trends</h4>
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div>
                  <div className="text-gray-400">Throughput Trend</div>
                  <div className={`font-mono ${trends.throughput >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {trends.throughput >= 0 ? '+' : ''}{trends.throughput.toFixed(1)}%
                  </div>
                </div>
                <div>
                  <div className="text-gray-400">Latency Trend</div>
                  <div className={`font-mono ${trends.latency <= 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {trends.latency >= 0 ? '+' : ''}{trends.latency.toFixed(1)}%
                  </div>
                </div>
                <div>
                  <div className="text-gray-400">Stability</div>
                  <div className={`font-mono ${Math.abs(trends.packetLoss) < 10 ? 'text-green-400' : 'text-yellow-400'}`}>
                    {trends.packetLoss >= 0 ? '+' : ''}{trends.packetLoss.toFixed(1)}%
                  </div>
                </div>
              </div>
            </div>
          )}
        </TabsContent>

        <TabsContent value="connections" className="space-y-4">
          <div className="grid gap-3">
            {connections.length === 0 ? (
              <div className="text-center py-8 text-gray-400">
                No QUIC connections available
              </div>
            ) : (
              connections.map(connection => (
                <div 
                  key={connection.id}
                  className={`bg-gray-800 rounded-lg p-4 cursor-pointer transition-colors ${
                    selectedConnection === connection.id ? 'ring-2 ring-blue-500' : 'hover:bg-gray-750'
                  }`}
                  onClick={() => setSelectedConnection(connection.id)}
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="font-mono text-sm text-white">{connection.id.slice(0, 8)}...</div>
                    <Badge 
                      variant={connection.status === 'connected' ? 'default' : 'destructive'}
                      className="text-xs"
                    >
                      {connection.status}
                    </Badge>
                  </div>
                  <div className="text-xs text-gray-400 space-y-1">
                    <div>Remote: {connection.remoteAddress}</div>
                    <div>Protocol: {connection.protocol} v{connection.version}</div>
                    <div>Streams: {connection.streams.active}/{connection.streams.total}</div>
                    <div>Last Activity: {new Date(connection.lastActivity).toLocaleTimeString()}</div>
                  </div>
                </div>
              ))
            )}
          </div>
        </TabsContent>

        <TabsContent value="quality" className="space-y-4">
          {networkQuality ? (
            <div className="space-y-4">
              <div className="bg-gray-800 rounded-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="text-white font-medium">Network Quality</h4>
                  <Badge 
                    variant={networkQuality.overall === 'excellent' ? 'default' : 'destructive'}
                    className="capitalize"
                  >
                    {networkQuality.overall}
                  </Badge>
                </div>
                <div className="text-2xl font-bold text-blue-400 mb-2">
                  {networkQuality.score}/100
                </div>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <div className="text-gray-400">Bandwidth</div>
                    <div className="text-white">{networkQuality.factors.bandwidth}/100</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Latency</div>
                    <div className="text-white">{networkQuality.factors.latency}/100</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Stability</div>
                    <div className="text-white">{networkQuality.factors.stability}/100</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Error Rate</div>
                    <div className="text-white">{networkQuality.factors.errorRate}/100</div>
                  </div>
                </div>
              </div>

              {networkQuality.recommendations.length > 0 && (
                <div className="bg-gray-800 rounded-lg p-4">
                  <h4 className="text-white font-medium mb-3">Recommendations</h4>
                  <ul className="space-y-2 text-sm">
                    {networkQuality.recommendations.map((rec, index) => (
                      <li key={index} className="text-gray-300 flex items-start gap-2">
                        <span className="text-blue-400 mt-0.5">•</span>
                        {rec}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          ) : qualityLoading ? (
            <div className="bg-gray-800 rounded-lg p-4">
              <div className="animate-pulse space-y-3">
                <div className="h-4 bg-gray-700 rounded w-32"></div>
                <div className="h-8 bg-gray-700 rounded w-16"></div>
                <div className="space-y-2">
                  <div className="h-3 bg-gray-700 rounded w-full"></div>
                  <div className="h-3 bg-gray-700 rounded w-3/4"></div>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-center py-8 text-gray-400">
              Select a connection to view quality metrics
            </div>
          )}
        </TabsContent>

        <TabsContent value="bottlenecks" className="space-y-4">
          {bottlenecks.length > 0 ? (
            <div className="bg-gray-800 rounded-lg p-4">
              <h4 className="text-white font-medium mb-3">Performance Bottlenecks</h4>
              <ul className="space-y-3">
                {bottlenecks.map((bottleneck, index) => (
                  <li key={index} className="flex items-start gap-3 text-sm">
                    <span className="text-red-400 mt-0.5">⚠</span>
                    <span className="text-gray-300">{bottleneck}</span>
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <div className="bg-gray-800 rounded-lg p-4 text-center py-8">
              <div className="text-green-400 text-2xl mb-2">✓</div>
              <div className="text-white font-medium">No Performance Issues</div>
              <div className="text-gray-400 text-sm">System is operating optimally</div>
            </div>
          )}

          {latestMetrics && (
            <div className="bg-gray-800 rounded-lg p-4">
              <h4 className="text-white font-medium mb-3">Detailed Metrics</h4>
              <div className="grid grid-cols-2 gap-4 text-xs">
                <div>
                  <div className="text-gray-400 mb-2">Congestion Control</div>
                  <div className="space-y-1">
                    <div>Window Size: {latestMetrics.congestion.windowSize.toLocaleString()}</div>
                    <div>In Flight: {latestMetrics.congestion.inFlight}</div>
                    <div>Retransmissions: {latestMetrics.congestion.retransmissions}</div>
                    <div>Congestion Events: {latestMetrics.congestion.congestionEvents}</div>
                  </div>
                </div>
                <div>
                  <div className="text-gray-400 mb-2">Stream Utilization</div>
                  <div className="space-y-1">
                    <div>Active Streams: {latestMetrics.streams.activeStreams}</div>
                    <div>Max Streams: {latestMetrics.streams.maxStreams}</div>
                    <div>Creation Rate: {latestMetrics.streams.streamCreationRate.toFixed(1)}/s</div>
                    <div>Completion Rate: {latestMetrics.streams.streamCompletionRate.toFixed(1)}/s</div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </TabsContent>
      </Tabs>

      {/* Live Indicator */}
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>
          Last updated: {latestMetrics ? new Date(latestMetrics.timestamp).toLocaleTimeString() : 'Never'}
        </span>
        <div className="flex items-center gap-1">
          <div className="w-1 h-1 bg-green-400 rounded-full animate-pulse" />
          <span>Live Updates</span>
        </div>
      </div>
    </div>
  );
}
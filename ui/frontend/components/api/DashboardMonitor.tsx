// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Dashboard Monitor - Comprehensive real-time monitoring for Web3 ecosystem
 * 
 * Integrates all monitoring components in a unified dashboard:
 * - System status from all services (TrustChain, HyperMesh, STOQ)
 * - Real-time performance metrics with 40 Gbps target tracking
 * - Asset allocation and resource utilization
 * - Network health and Byzantine detection alerts
 */

import React, { useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { SystemStatusWidget } from './SystemStatusWidget';
import { PerformanceMonitor } from './PerformanceMonitor';
import {
  useStoqSystemStatus as useSystemStatus,
  useStoqAssets as useAssets,
  useStoqAllocations as useAllocations,
  useStoqByzantineDetections as useByzantineDetections,
  useStoqQUICConnections as useQUICConnections,
  useStoqPerformanceMetrics as usePerformanceMetrics,
  getSystemHealthSummary
} from '../../lib/api';
import {
  Activity,
  Network,
  Shield,
  AlertTriangle,
  Zap,
  TrendingUp
} from 'lucide-react';

export function DashboardMonitor() {
  const [activeTab, setActiveTab] = useState('overview');
  
  // Real-time data hooks
  const { systemStatus, isHealthy, hasWarnings, isCritical } = useSystemStatus(true);
  const { assets, availableAssets, allocatedAssets } = useAssets();
  const { allocations, activeAllocations } = useAllocations();
  const { detections, criticalDetections, unresolved } = useByzantineDetections();
  const { connections, activeConnections } = useQUICConnections();
  const { latestMetrics, throughputAchievement, performanceGrade, bottlenecks } = usePerformanceMetrics(
    undefined, undefined, true
  );

  const healthSummary = getSystemHealthSummary(systemStatus);

  // Calculate key metrics
  const keyMetrics = React.useMemo(() => {
    return {
      systemHealth: healthSummary.score,
      throughputTarget: latestMetrics ? (latestMetrics.throughput.download / 40000) * 100 : 0,
      resourceUtilization: activeAllocations ? (activeAllocations.length / Math.max(assets?.length || 1, 1)) * 100 : 0,
      networkConnections: activeConnections?.length || 0,
      securityAlerts: criticalDetections?.length || 0,
      assetHealth: availableAssets ? (availableAssets.length / Math.max(assets?.length || 1, 1)) * 100 : 0
    };
  }, [systemStatus, latestMetrics, activeAllocations, assets, activeConnections, criticalDetections, availableAssets]);

  const getHealthColor = (score: number) => {
    if (score >= 90) return 'text-green-400';
    if (score >= 75) return 'text-blue-400';
    if (score >= 50) return 'text-yellow-400';
    if (score >= 25) return 'text-orange-400';
    return 'text-red-400';
  };

  const getHealthBadge = (isHealthy: boolean, hasWarnings: boolean, isCritical: boolean) => {
    if (isCritical) return <Badge variant="destructive" className="text-xs">Critical</Badge>;
    if (hasWarnings) return <Badge variant="secondary" className="text-xs bg-yellow-500/20 text-yellow-400">Warning</Badge>;
    if (isHealthy) return <Badge variant="default" className="text-xs bg-green-500/20 text-green-400">Healthy</Badge>;
    return <Badge variant="outline" className="text-xs">Unknown</Badge>;
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white flex items-center gap-2">
            <Activity className="h-6 w-6 text-cyan-400" />
            System Monitor
          </h2>
          <p className="text-gray-400">Real-time monitoring and performance analytics</p>
        </div>
        <div className="flex items-center gap-2">
          {getHealthBadge(isHealthy, hasWarnings, isCritical)}
          <Badge variant="outline" className="text-xs text-green-400 border-green-400">
            Live Updates
          </Badge>
        </div>
      </div>

      {/* Key Metrics Overview */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">System Health</p>
                <p className={`text-lg font-bold ${getHealthColor(keyMetrics.systemHealth)}`}>
                  {keyMetrics.systemHealth}%
                </p>
              </div>
              <Activity className="h-4 w-4 text-cyan-400" />
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">Throughput</p>
                <p className={`text-lg font-bold ${getHealthColor(keyMetrics.throughputTarget)}`}>
                  {keyMetrics.throughputTarget.toFixed(1)}%
                </p>
              </div>
              <TrendingUp className="h-4 w-4 text-green-400" />
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">Resources</p>
                <p className={`text-lg font-bold ${getHealthColor(keyMetrics.resourceUtilization)}`}>
                  {keyMetrics.resourceUtilization.toFixed(0)}%
                </p>
              </div>
              <Zap className="h-4 w-4 text-purple-400" />
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">Connections</p>
                <p className="text-lg font-bold text-blue-400">
                  {keyMetrics.networkConnections}
                </p>
              </div>
              <Network className="h-4 w-4 text-blue-400" />
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">Security</p>
                <p className={`text-lg font-bold ${keyMetrics.securityAlerts > 0 ? 'text-red-400' : 'text-green-400'}`}>
                  {keyMetrics.securityAlerts}
                </p>
              </div>
              <Shield className="h-4 w-4 text-green-400" />
            </div>
          </CardContent>
        </Card>

        <Card className="bg-gray-900 border-gray-700">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs text-gray-400">Asset Health</p>
                <p className={`text-lg font-bold ${getHealthColor(keyMetrics.assetHealth)}`}>
                  {keyMetrics.assetHealth.toFixed(0)}%
                </p>
              </div>
              <Activity className="h-4 w-4 text-cyan-400" />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Detailed Monitoring Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="performance">Performance</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="resources">Resources</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-6">
          <div className="grid gap-6 lg:grid-cols-2">
            <SystemStatusWidget />
            
            <Card className="bg-gray-900 border-gray-700">
              <CardHeader>
                <CardTitle className="text-white flex items-center gap-2">
                  <TrendingUp className="h-5 w-5 text-green-400" />
                  Performance Summary
                </CardTitle>
                <CardDescription>Current system performance overview</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Performance Grade</span>
                    <Badge variant="outline" className={getHealthColor(throughputAchievement)}>
                      {performanceGrade}
                    </Badge>
                  </div>
                  
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Throughput Achievement</span>
                    <span className={`text-sm font-mono ${getHealthColor(throughputAchievement)}`}>
                      {throughputAchievement.toFixed(1)}% of 40 Gbps
                    </span>
                  </div>
                  
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Active Bottlenecks</span>
                    <span className={`text-sm ${bottlenecks.length > 0 ? 'text-red-400' : 'text-green-400'}`}>
                      {bottlenecks.length} detected
                    </span>
                  </div>
                  
                  {latestMetrics && (
                    <div className="pt-4 border-t border-gray-700 space-y-2">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-400">Latency</span>
                        <span className="text-white font-mono">{latestMetrics.latency.rtt.toFixed(1)} ms</span>
                      </div>
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-400">Packet Loss</span>
                        <span className="text-white font-mono">{latestMetrics.latency.packetLoss.toFixed(2)}%</span>
                      </div>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="performance" className="space-y-6">
          <PerformanceMonitor />
        </TabsContent>

        <TabsContent value="security" className="space-y-6">
          <div className="grid gap-6">
            <Card className="bg-gray-900 border-gray-700">
              <CardHeader>
                <CardTitle className="text-white flex items-center gap-2">
                  <Shield className="h-5 w-5 text-green-400" />
                  Byzantine Detection
                  {criticalDetections && criticalDetections.length > 0 && (
                    <Badge variant="destructive" className="text-xs">
                      {criticalDetections.length} Critical
                    </Badge>
                  )}
                </CardTitle>
                <CardDescription>
                  Real-time Byzantine fault detection and network security monitoring
                </CardDescription>
              </CardHeader>
              <CardContent>
                {detections && detections.length > 0 ? (
                  <div className="space-y-3">
                    {detections.slice(0, 5).map((detection, index) => (
                      <div key={index} className="flex items-center justify-between p-3 bg-gray-800 rounded-lg">
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="text-sm font-medium text-white">
                              {detection.behaviour.replace('_', ' ')}
                            </span>
                            <Badge variant={detection.severity === 'critical' ? 'destructive' : 'secondary'} className="text-xs">
                              {detection.severity}
                            </Badge>
                          </div>
                          <p className="text-xs text-gray-400">
                            Node: {detection.nodeId.slice(0, 12)}... | {new Date(detection.detectedAt).toLocaleTimeString()}
                          </p>
                        </div>
                        <Badge variant="outline" className={
                          detection.status === 'resolved' ? 'text-green-400' :
                          detection.status === 'investigating' ? 'text-yellow-400' :
                          'text-red-400'
                        }>
                          {detection.status}
                        </Badge>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8">
                    <Shield className="h-12 w-12 text-green-400 mx-auto mb-2" />
                    <div className="text-white font-medium">No Security Threats</div>
                    <div className="text-gray-400 text-sm">Network is secure and operational</div>
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="resources" className="space-y-6">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card className="bg-gray-900 border-gray-700">
              <CardHeader>
                <CardTitle className="text-white">Asset Overview</CardTitle>
                <CardDescription>Current asset allocation and utilization</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Total Assets</span>
                    <span className="text-white font-mono">{assets?.length || 0}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Available</span>
                    <span className="text-green-400 font-mono">{availableAssets?.length || 0}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Allocated</span>
                    <span className="text-blue-400 font-mono">{allocatedAssets?.length || 0}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Active Allocations</span>
                    <span className="text-purple-400 font-mono">{activeAllocations?.length || 0}</span>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="bg-gray-900 border-gray-700">
              <CardHeader>
                <CardTitle className="text-white">Network Status</CardTitle>
                <CardDescription>QUIC connections and transport performance</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Total Connections</span>
                    <span className="text-white font-mono">{connections?.length || 0}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-400">Active Connections</span>
                    <span className="text-green-400 font-mono">{activeConnections?.length || 0}</span>
                  </div>
                  {latestMetrics && (
                    <>
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-gray-400">Current Throughput</span>
                        <span className="text-blue-400 font-mono">
                          {(latestMetrics.throughput.download / 1000).toFixed(2)} Gbps
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-gray-400">Target Progress</span>
                        <span className={`font-mono ${getHealthColor(throughputAchievement)}`}>
                          {throughputAchievement.toFixed(1)}%
                        </span>
                      </div>
                    </>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
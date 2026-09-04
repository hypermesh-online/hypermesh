// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Shield, Network, HardDrive, Coins, Activity, RefreshCw } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  DetailedMetricsTabs,
  getStatusIcon,
  getStatusColor,
  getTrendIcon,
  formatUptime,
  getUptimePercentage,
  defaultMetrics,
  defaultSystemStatuses,
  metricTrends
} from './ecosystem-metrics-dashboard';
import type { EcosystemMetricsDashboardProps } from './ecosystem-metrics-dashboard';

// Re-export types for consumers
export type { EcosystemMetrics, SystemStatus, MetricTrend } from './ecosystem-metrics-dashboard';

export function EcosystemMetricsDashboard({
  metrics = defaultMetrics,
  systemStatuses = defaultSystemStatuses,
  onRefresh,
  autoRefresh = true,
  refreshInterval = 30000,
  loading = false,
  className
}: EcosystemMetricsDashboardProps) {
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [activeTab, setActiveTab] = useState('overview');

  const handleRefresh = useCallback(() => {
    setLastRefresh(new Date());
    onRefresh?.();
  }, [onRefresh]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(() => { handleRefresh(); }, refreshInterval);
    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, handleRefresh]);

  const overallHealthScore = () => {
    const onlineCount = systemStatuses.filter(s => s.status === 'online').length;
    return (onlineCount / systemStatuses.length) * 100;
  };

  return (
    <div className={cn("space-y-6", className)}>
      {/* Dashboard Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">HyperMesh Ecosystem Dashboard</h1>
          <p className="text-gray-400 mt-2">
            Quantum-secure, user-sovereign internet infrastructure
          </p>
        </div>
        <div className="flex items-center space-x-4">
          <div className="text-sm text-gray-400">
            Last updated: {lastRefresh.toLocaleTimeString()}
          </div>
          <Button
            variant="outline" size="sm" onClick={handleRefresh} disabled={loading}
            className="border-green-500/30 text-green-400 hover:bg-green-500/20"
          >
            <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Key Metrics Grid */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Assets</CardTitle>
            <HardDrive className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-400">{metrics.totalAssets.toLocaleString()}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.totalAssets.trend)}
              <span className={cn(
                metricTrends.totalAssets.trend === 'up' ? 'text-green-400' :
                metricTrends.totalAssets.trend === 'down' ? 'text-red-400' : 'text-gray-400'
              )}>
                {metricTrends.totalAssets.change > 0 ? '+' : ''}{metricTrends.totalAssets.change}%
              </span>
              <span>{metricTrends.totalAssets.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active Certificates</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{metrics.activeCertificates.toLocaleString()}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.activeCertificates.trend)}
              <span className="text-green-400">+{metricTrends.activeCertificates.change}%</span>
              <span>{metricTrends.activeCertificates.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Network Throughput</CardTitle>
            <Network className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-yellow-400">{metrics.networkThroughput.toFixed(2)} Gbps</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.networkThroughput.trend)}
              <span className="text-red-400">{metricTrends.networkThroughput.change}%</span>
              <span>{metricTrends.networkThroughput.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Economic Rewards</CardTitle>
            <Coins className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{metrics.economicRewards.toFixed(2)}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.economicRewards.trend)}
              <span className="text-green-400">+{metricTrends.economicRewards.change}%</span>
              <span>{metricTrends.economicRewards.period}</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Health Overview */}
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-white flex items-center space-x-2">
                <Activity className="h-5 w-5 text-green-400" />
                <span>System Health Overview</span>
              </CardTitle>
              <CardDescription className="text-gray-400">
                Real-time status of all HyperMesh ecosystem components
              </CardDescription>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-green-400">{overallHealthScore().toFixed(1)}%</div>
              <div className="text-sm text-gray-400">Overall Health</div>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {systemStatuses.map((system) => (
              <div
                key={system.name}
                className={cn(
                  "border rounded-lg p-4 transition-all duration-300",
                  system.status === 'online' ? 'bg-green-500/5 border-green-500/30' :
                  system.status === 'warning' ? 'bg-yellow-500/5 border-yellow-500/30' :
                  system.status === 'offline' ? 'bg-red-500/5 border-red-500/30' :
                  'bg-blue-500/5 border-blue-500/30'
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <h4 className="font-medium text-white">{system.name}</h4>
                  <div className="flex items-center space-x-2">
                    {getStatusIcon(system.status)}
                    <Badge className={getStatusColor(system.status)}>
                      {system.status.toUpperCase()}
                    </Badge>
                  </div>
                </div>
                {system.description && (
                  <p className="text-xs text-gray-400 mb-3">{system.description}</p>
                )}
                <div className="space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Uptime:</span>
                    <span className="text-white">{formatUptime(system.uptime)}</span>
                  </div>
                  <Progress value={getUptimePercentage(system.uptime)} className="h-1" />
                  <div className="flex justify-between">
                    <span className="text-gray-400">Last Check:</span>
                    <span className="text-white">{new Date(system.lastChecked).toLocaleTimeString()}</span>
                  </div>
                </div>
                <div className="mt-3 pt-3 border-t border-current/20">
                  <div className="grid grid-cols-1 gap-1 text-xs">
                    {Object.entries(system.metrics).slice(0, 2).map(([key, value]) => (
                      <div key={key} className="flex justify-between">
                        <span className="text-gray-400">{key}:</span>
                        <span className="text-white font-mono">{value}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Detailed Metrics Tabs */}
      <DetailedMetricsTabs metrics={metrics} activeTab={activeTab} onTabChange={setActiveTab} />
    </div>
  );
}

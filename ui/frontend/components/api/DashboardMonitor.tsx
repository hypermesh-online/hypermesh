// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Dashboard Monitor - Unified system health overview
 *
 * Uses useNodeStatus, useNetworkPeers, useStoqStats from useBlockMatrix hooks.
 */

import React, { useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { SystemStatusWidget } from './SystemStatusWidget';
import { PerformanceMonitor } from './PerformanceMonitor';
import {
  useNodeStatus,
  useNetworkPeers,
  useStoqStats,
  useStoqPerformance,
  useAssetList,
  useBlockchainHeight,
} from '@/lib/hooks/useBlockMatrix';
import {
  Activity,
  Network,
  Shield,
  Zap,
  TrendingUp,
} from 'lucide-react';

export function DashboardMonitor() {
  const [activeTab, setActiveTab] = useState('overview');

  const nodeStatus = useNodeStatus();
  const peers = useNetworkPeers();
  const stoqStats = useStoqStats();
  const stoqPerf = useStoqPerformance();
  const assets = useAssetList();
  const chainHeight = useBlockchainHeight();

  const isOnline = !!nodeStatus.data && !nodeStatus.error;
  const peerCount = nodeStatus.data?.peers ?? peers.data?.length ?? 0;

  if (nodeStatus.isLoading && stoqStats.isLoading) {
    return <ModuleLoading />;
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white flex items-center gap-2">
            <Activity className="h-6 w-6 text-cyan-400" />
            System Monitor
          </h2>
          <p className="text-gray-400">Real-time monitoring and performance analytics</p>
        </div>
        <div className="flex items-center gap-2">
          <Badge
            variant={isOnline ? 'default' : 'destructive'}
            className={isOnline
              ? 'bg-green-500/20 text-green-400 text-xs'
              : 'text-xs'}
          >
            {isOnline ? 'Online' : 'Offline'}
          </Badge>
          <Badge variant="outline" className="text-xs text-green-400 border-green-400">
            Live Updates
          </Badge>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <CompactMetric
          label="Node Status"
          value={isOnline ? 'Online' : 'Offline'}
          icon={Activity}
          color={isOnline ? 'text-green-400' : 'text-red-400'}
          loading={nodeStatus.isLoading}
        />
        <CompactMetric
          label="Chain Height"
          value={String(chainHeight.data?.height ?? nodeStatus.data?.chain_height ?? 0)}
          icon={TrendingUp}
          color="text-cyan-400"
          loading={chainHeight.isLoading}
        />
        <CompactMetric
          label="Peers"
          value={String(peerCount)}
          icon={Network}
          color="text-blue-400"
          loading={peers.isLoading}
        />
        <CompactMetric
          label="Connections"
          value={String(stoqStats.data?.connections ?? stoqStats.data?.connections_active ?? 0)}
          icon={Zap}
          color="text-purple-400"
          loading={stoqStats.isLoading}
        />
        <CompactMetric
          label="Throughput"
          value={stoqPerf.data
            ? `${(throughputMbps(stoqPerf.data) / 1000).toFixed(1)} Gbps`
            : 'N/A'}
          icon={TrendingUp}
          color="text-green-400"
          loading={stoqPerf.isLoading}
        />
        <CompactMetric
          label="Assets"
          value={String(assets.data?.length ?? 0)}
          icon={Shield}
          color="text-cyan-400"
          loading={assets.isLoading}
        />
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="performance">Performance</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-6">
          <div className="grid gap-6 lg:grid-cols-2">
            <SystemStatusWidget />

            <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
              <CardHeader>
                <CardTitle className="text-white flex items-center gap-2">
                  <TrendingUp className="h-5 w-5 text-green-400" />
                  Performance Summary
                </CardTitle>
                <CardDescription className="text-gray-400">
                  Current system performance overview
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <SummaryRow
                    label="Throughput"
                    value={stoqPerf.data
                      ? `${(throughputMbps(stoqPerf.data) / 1000).toFixed(2)} Gbps`
                      : '--'}
                  />
                  <SummaryRow
                    label="Latency"
                    value={stoqPerf.data ? `${latencyMs(stoqPerf.data).toFixed(1)} ms` : '--'}
                  />
                  <SummaryRow
                    label="Packet Loss"
                    value={stoqPerf.data ? `${packetLossPct(stoqPerf.data).toFixed(2)}%` : '--'}
                  />
                  <SummaryRow
                    label="Bytes Transferred"
                    value={stoqStats.data
                      ? formatBytes((stoqStats.data.bytes_sent ?? 0) + (stoqStats.data.bytes_received ?? 0))
                      : '--'}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="performance" className="space-y-6">
          <PerformanceMonitor />
        </TabsContent>

        <TabsContent value="network" className="space-y-6">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
              <CardHeader>
                <CardTitle className="text-white">Asset Overview</CardTitle>
                <CardDescription className="text-gray-400">
                  Blockchain-registered assets
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <SummaryRow label="Total Assets" value={String(assets.data?.length ?? 0)} />
                  <SummaryRow
                    label="Chain Height"
                    value={String(chainHeight.data?.height ?? nodeStatus.data?.chain_height ?? 0)}
                  />
                  <SummaryRow label="Connected Peers" value={String(peerCount)} />
                </div>
              </CardContent>
            </Card>

            <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
              <CardHeader>
                <CardTitle className="text-white">Network Stats</CardTitle>
                <CardDescription className="text-gray-400">
                  STOQ transport statistics
                </CardDescription>
              </CardHeader>
              <CardContent>
                {stoqStats.isLoading ? (
                  <Skeleton className="h-20 w-full" />
                ) : stoqStats.data ? (
                  <div className="space-y-3">
                    <SummaryRow
                      label="Active Connections"
                      value={String(stoqStats.data.connections ?? stoqStats.data.connections_active ?? 0)}
                    />
                    <SummaryRow
                      label="Total Sent"
                      value={formatBytes(stoqStats.data.bytes_sent ?? 0)}
                    />
                    <SummaryRow
                      label="Total Received"
                      value={formatBytes(stoqStats.data.bytes_received ?? 0)}
                    />
                    <SummaryRow
                      label="Packets"
                      value={`${(stoqStats.data.packets_sent ?? 0).toLocaleString()} / ${(stoqStats.data.packets_received ?? 0).toLocaleString()}`}
                    />
                  </div>
                ) : (
                  <p className="text-gray-500 text-center py-4">No stats available</p>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function CompactMetric({
  label,
  value,
  icon: Icon,
  color,
  loading,
}: {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  loading: boolean;
}) {
  return (
    <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
      <CardContent className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs text-gray-400">{label}</p>
            {loading ? (
              <Skeleton className="h-6 w-16 mt-1" />
            ) : (
              <p className={`text-lg font-bold ${color}`}>{value}</p>
            )}
          </div>
          <Icon className={`h-4 w-4 ${color}`} />
        </div>
      </CardContent>
    </Card>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-gray-400">{label}</span>
      <span className="text-sm font-mono text-white">{value}</span>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

// --- PerformanceMetrics field normalizers ---
// The daemon `stoq.performance` handler uses `avg_latency_ms`,
// `throughput_bps`, and `packet_loss_rate`. Legacy `*_ms / *_mbps / *_pct`
// names are accepted as a fallback.

type PerfShape = {
  throughput_mbps?: number;
  latency_ms?: number;
  packet_loss_pct?: number;
  avg_latency_ms?: number;
  throughput_bps?: number;
  packet_loss_rate?: number;
};

function throughputMbps(perf: PerfShape): number {
  if (typeof perf.throughput_mbps === 'number') return perf.throughput_mbps;
  if (typeof perf.throughput_bps === 'number') return perf.throughput_bps / 1_000_000;
  return 0;
}

function latencyMs(perf: PerfShape): number {
  if (typeof perf.latency_ms === 'number') return perf.latency_ms;
  if (typeof perf.avg_latency_ms === 'number') return perf.avg_latency_ms;
  return 0;
}

function packetLossPct(perf: PerfShape): number {
  if (typeof perf.packet_loss_pct === 'number') return perf.packet_loss_pct;
  if (typeof perf.packet_loss_rate === 'number') return perf.packet_loss_rate * 100;
  return 0;
}

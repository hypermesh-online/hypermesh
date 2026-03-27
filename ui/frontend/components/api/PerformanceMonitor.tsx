// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Performance Monitor - STOQ transport performance metrics
 *
 * Uses useStoqPerformance, useStoqConnections from useBlockMatrix hooks.
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import { useStoqPerformance, useStoqConnections, useStoqStats } from '@/lib/hooks/useBlockMatrix';
import { Activity, Zap, AlertTriangle, Network } from 'lucide-react';
import { cn } from '@/lib/utils';

const colors = moduleColors.stoq;

export function PerformanceMonitor() {
  const performance = useStoqPerformance();
  const connections = useStoqConnections();
  const stats = useStoqStats();

  if (performance.isLoading && connections.isLoading) {
    return <ModuleLoading />;
  }

  if (performance.error && connections.error && stats.error) {
    return (
      <div className="p-6">
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">STOQ service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach STOQ transport backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const perf = performance.data;
  const throughputPct = perf?.throughput_mbps
    ? Math.min(100, (perf.throughput_mbps / 40000) * 100)
    : 0;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-white text-lg font-medium flex items-center gap-2">
          <Activity className="h-5 w-5 text-purple-400" />
          Performance Monitor
        </h3>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-xs text-purple-400 border-purple-500/30">
            {connections.data?.total ?? stats.data?.connections_active ?? 0} Connections
          </Badge>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <MetricCard
          title="Throughput"
          value={perf ? formatThroughput(perf.throughput_mbps) : 'N/A'}
          subtitle={`Target: 40 Gbps (${throughputPct.toFixed(1)}%)`}
          loading={performance.isLoading}
          color={getThroughputColor(throughputPct)}
        />
        <MetricCard
          title="Latency"
          value={perf ? `${perf.latency_ms.toFixed(1)} ms` : 'N/A'}
          subtitle={`Jitter: ${perf?.jitter_ms?.toFixed(1) ?? '0'} ms`}
          loading={performance.isLoading}
          color="text-blue-400"
        />
        <MetricCard
          title="Packet Loss"
          value={perf ? `${perf.packet_loss_pct.toFixed(2)}%` : 'N/A'}
          subtitle={perf?.packet_loss_pct != null && perf.packet_loss_pct < 0.1
            ? 'Minimal loss'
            : 'Monitor closely'}
          loading={performance.isLoading}
          color={perf?.packet_loss_pct != null && perf.packet_loss_pct < 0.5
            ? 'text-green-400'
            : 'text-yellow-400'}
        />
      </div>

      {/* Connection Details */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Network className="h-5 w-5 text-purple-400" />
            Active Connections
          </CardTitle>
        </CardHeader>
        <CardContent>
          {connections.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full" />
              ))}
            </div>
          ) : connections.data?.connections && connections.data.connections.length > 0 ? (
            <div className="space-y-2 max-h-60 overflow-y-auto">
              {connections.data.connections.map((conn) => (
                <div
                  key={conn.id}
                  className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                >
                  <div>
                    <p className="text-sm text-white font-mono">{conn.id.slice(0, 12)}...</p>
                    <p className="text-xs text-gray-400">{conn.remote_addr}</p>
                  </div>
                  <div className="text-right">
                    <Badge className={cn(
                      "text-xs",
                      conn.state === 'connected'
                        ? 'bg-green-500/20 text-green-400'
                        : 'bg-gray-500/20 text-gray-400',
                    )}>
                      {conn.state}
                    </Badge>
                    <p className="text-xs text-gray-400 mt-1">
                      {formatBytes(conn.bytes_sent)} / {formatBytes(conn.bytes_received)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-400">No active connections</div>
          )}
        </CardContent>
      </Card>

      {/* Transport Stats */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Zap className="h-5 w-5 text-purple-400" />
            Transport Stats
          </CardTitle>
        </CardHeader>
        <CardContent>
          {stats.isLoading ? (
            <Skeleton className="h-20 w-full" />
          ) : stats.error ? (
            <p className="text-gray-500 text-center py-4">Stats unavailable</p>
          ) : stats.data ? (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <StatBlock label="Bytes Sent" value={formatBytes(stats.data.bytes_sent)} />
              <StatBlock label="Bytes Received" value={formatBytes(stats.data.bytes_received)} />
              <StatBlock label="Packets Sent" value={stats.data.packets_sent.toLocaleString()} />
              <StatBlock label="Packets Received" value={stats.data.packets_received.toLocaleString()} />
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Live Indicator */}
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>Polling every 5s</span>
        <div className="flex items-center gap-1">
          <div className="w-1 h-1 bg-green-400 rounded-full animate-pulse" />
          <span>Live Updates</span>
        </div>
      </div>
    </div>
  );
}

function MetricCard({
  title,
  value,
  subtitle,
  loading,
  color,
}: {
  title: string;
  value: string;
  subtitle: string;
  loading: boolean;
  color: string;
}) {
  return (
    <div className="bg-black/40 border border-purple-500/30 rounded-lg p-4 backdrop-blur-lg">
      <div className="text-gray-400 text-sm mb-1">{title}</div>
      {loading ? (
        <Skeleton className="h-8 w-24" />
      ) : (
        <>
          <div className={`text-2xl font-mono font-bold ${color}`}>{value}</div>
          <div className="text-xs text-gray-500 mt-1">{subtitle}</div>
        </>
      )}
    </div>
  );
}

function StatBlock({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-3 rounded-lg bg-black/20 border border-gray-800">
      <div className="text-xs text-gray-400">{label}</div>
      <div className="text-sm font-mono text-white">{value}</div>
    </div>
  );
}

function formatThroughput(mbps: number): string {
  if (mbps >= 1000) return `${(mbps / 1000).toFixed(2)} Gbps`;
  return `${mbps.toFixed(1)} Mbps`;
}

function getThroughputColor(pct: number): string {
  if (pct >= 90) return 'text-green-400';
  if (pct >= 75) return 'text-blue-400';
  if (pct >= 50) return 'text-yellow-400';
  return 'text-orange-400';
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

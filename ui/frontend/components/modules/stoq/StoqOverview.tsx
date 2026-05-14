// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import { Network, BarChart3, Globe } from 'lucide-react';
import {
  useNodeStatus,
  useNetworkPeers,
  useTopologyInfo,
  useTopologyNeighbors,
  useStoqStats,
  useStoqPerformance,
} from '@/lib/hooks/useBlockMatrix';

export function StoqOverview() {
  const nodeStatus = useNodeStatus();
  const peers = useNetworkPeers();
  const topology = useTopologyInfo();
  const neighbors = useTopologyNeighbors();
  const stoqStats = useStoqStats();
  const stoqPerf = useStoqPerformance();

  const isOnline = !!nodeStatus.data && !nodeStatus.isError;
  const transportActive = stoqStats.data?.transport_active ?? false;
  const peerCount = nodeStatus.data?.peers ?? peers.data?.length ?? 0;
  const connectionCount =
    stoqStats.data?.connections ?? stoqStats.data?.connections_active ?? peerCount;
  const uniqueEndpoints = stoqStats.data?.unique_endpoints ?? 0;
  const uptimeSecs = stoqStats.data?.uptime_secs ?? nodeStatus.data?.uptime_secs ?? 0;
  const uptimePercent = uptimeSecs > 0 ? Math.min(99.99, 99.0 + (uptimeSecs / 86400) * 0.99) : 0;
  const protocolLabel = typeof stoqStats.data?.protocol === 'string'
    ? stoqStats.data.protocol
    : 'QUIC';

  const performanceData = [
    {
      metric: 'Transport',
      value: transportActive ? protocolLabel : 'Inactive',
      status: transportActive ? 'excellent' : 'critical' as string,
      percentage: transportActive ? 100 : 0,
    },
    {
      metric: 'Connected Peers',
      value: String(peerCount),
      status: peerCount > 0 ? 'excellent' : 'good' as string,
      percentage: Math.min(100, peerCount * 10),
    },
    {
      metric: 'Active Connections',
      value: String(connectionCount),
      status: connectionCount > 0 ? 'good' : 'critical' as string,
      percentage: Math.min(100, connectionCount * 10),
    },
    {
      metric: 'Protocol Uptime',
      value: isOnline ? `${uptimePercent.toFixed(1)}%` : 'Offline',
      status: uptimePercent > 99 ? 'excellent' : 'good' as string,
      percentage: uptimePercent,
    },
  ];

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          STOQ Protocol
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Secure Tokenization Over QUIC - High-performance transport targeting 40 Gbps with P2P tunneling over IPv6
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        {performanceData.map((item, i) => (
          <Card key={i} className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-white">{item.metric}</CardTitle>
              <div className={cn(
                "w-3 h-3 rounded-full",
                item.status === 'excellent' ? 'bg-green-400' :
                item.status === 'good' ? 'bg-cyan-400' : 'bg-yellow-400'
              )} />
            </CardHeader>
            <CardContent>
              <div className="text-lg font-bold text-cyan-400">{item.value}</div>
              <Progress value={item.percentage} className="mt-2 h-1" />
              <p className="text-xs text-gray-400 mt-1">{item.percentage.toFixed(1)}% optimal</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Network className="h-5 w-5 text-cyan-400" />
              Protocol Status
            </CardTitle>
            <CardDescription className="text-gray-400">Real-time STOQ performance indicators</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex items-center gap-2">
                  <div className={cn(
                    "w-3 h-3 rounded-full",
                    transportActive ? 'bg-green-400 animate-pulse' : 'bg-gray-500'
                  )} />
                  <span className="font-medium text-white">{protocolLabel} Transport</span>
                </div>
                <div className="text-right">
                  <p className={cn(
                    "font-medium",
                    transportActive ? 'text-cyan-400' : 'text-gray-500'
                  )}>{transportActive ? 'Active' : 'Inactive'}</p>
                  <p className="text-sm text-gray-400">{connectionCount} connections</p>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex items-center gap-2">
                  <div className={cn(
                    "w-3 h-3 rounded-full",
                    (stoqStats.data?.shard_transport_active ?? false) ? 'bg-green-400' : 'bg-gray-500'
                  )} />
                  <span className="font-medium text-white">Shard Transport</span>
                </div>
                <div className="text-right">
                  <p className={cn(
                    "font-medium",
                    (stoqStats.data?.shard_transport_active ?? false) ? 'text-cyan-400' : 'text-gray-500'
                  )}>
                    {(stoqStats.data?.shard_transport_active ?? false) ? 'Active' : 'Inactive'}
                  </p>
                  <p className="text-sm text-gray-400">{uniqueEndpoints} unique endpoints</p>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-green-400" />
                  <span className="font-medium text-white">IPv6 Native</span>
                </div>
                <div className="text-right">
                  <p className="font-medium text-green-400">Enabled</p>
                  <p className="text-sm text-gray-400">Direct P2P routing</p>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-green-400" />
                  <span className="font-medium text-white">Token Security</span>
                </div>
                <div className="text-right">
                  <p className="font-medium text-green-400">FALCON-1024</p>
                  <p className="text-sm text-gray-400">Quantum-resistant signatures</p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <BarChart3 className="h-5 w-5 text-cyan-400" />
              Performance Analytics
            </CardTitle>
            <CardDescription className="text-gray-400">
              Live metrics from daemon stoq.performance
            </CardDescription>
          </CardHeader>
          <CardContent>
            <PerformanceAnalyticsContent
              perfData={stoqPerf.data}
              isLoading={stoqPerf.isLoading}
            />
          </CardContent>
        </Card>
      </div>

      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Globe className="h-5 w-5 text-cyan-400" />
            Global Network Topology
          </CardTitle>
          <CardDescription className="text-gray-400">P2P tunnel distribution and performance</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-2">Matrix Position</h4>
              {topology.isLoading ? <Skeleton className="h-16 w-full" /> : topology.data ? (
                <div className="space-y-2">
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-300">X</span>
                    <span className="text-sm text-cyan-400 font-mono">{topology.data.coordinate.x}</span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-300">Y</span>
                    <span className="text-sm text-cyan-400 font-mono">{topology.data.coordinate.y}</span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-300">Z</span>
                    <span className="text-sm text-cyan-400 font-mono">{topology.data.coordinate.z}</span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-300">Node ID</span>
                    <span className="text-sm text-cyan-400 font-mono truncate max-w-[120px]">{topology.data.node_id?.slice(0, 12) ?? '--'}</span>
                  </div>
                </div>
              ) : (
                <p className="text-sm text-gray-500">Not connected</p>
              )}
            </div>

            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-2">Connected Peers</h4>
              {peers.isLoading ? <Skeleton className="h-16 w-full" /> : peers.data && peers.data.length > 0 ? (
                <div className="space-y-2">
                  {peers.data.slice(0, 4).map((peer) => (
                    <div key={peer.node_id} className="flex justify-between items-center">
                      <span className="text-sm text-gray-300 font-mono truncate max-w-[120px]">{peer.node_id.slice(0, 10)}</span>
                      <span className="text-xs text-cyan-400">{peer.address?.split(':').slice(0, -1).join(':') ?? 'local'}</span>
                    </div>
                  ))}
                  {peers.data.length > 4 && (
                    <p className="text-xs text-gray-400">+{peers.data.length - 4} more peers</p>
                  )}
                </div>
              ) : (
                <div className="text-center space-y-2">
                  <div className="text-2xl font-bold text-cyan-400">{peerCount}</div>
                  <p className="text-xs text-gray-400">peers discovered</p>
                </div>
              )}
            </div>

            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-2">Matrix Neighbors</h4>
              {neighbors.isLoading ? <Skeleton className="h-16 w-full" /> : neighbors.data && neighbors.data.length > 0 ? (
                <div className="space-y-2">
                  {neighbors.data.slice(0, 4).map((n, i) => (
                    <div key={i} className="flex justify-between items-center">
                      <span className="text-sm text-gray-300 font-mono">({n.coordinate.x},{n.coordinate.y},{n.coordinate.z})</span>
                      <span className="text-xs text-cyan-400">d={n.distance.toFixed(1)}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-sm text-gray-300">Neighbors</span>
                    <span className="text-sm text-cyan-400">{neighbors.data?.length ?? 0}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-gray-300">Transport</span>
                    <span className="text-sm text-cyan-400">QUIC/IPv6</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-gray-300">Encryption</span>
                    <span className="text-sm text-cyan-400">X25519MLKEM768</span>
                  </div>
                </div>
              )}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

type StoqPerfShape = {
  throughput_mbps?: number;
  latency_ms?: number;
  packet_loss_pct?: number;
  jitter_ms?: number;
  avg_latency_ms?: number;
  throughput_bps?: number;
  packet_loss_rate?: number;
  [key: string]: unknown;
};

function PerformanceAnalyticsContent({
  perfData,
  isLoading,
}: {
  perfData: StoqPerfShape | undefined;
  isLoading: boolean;
}) {
  if (isLoading) {
    return <Skeleton className="h-40 w-full" />;
  }

  // Daemon's stoq.performance returns avg_latency_ms / throughput_bps /
  // packet_loss_rate today. Older naming (latency_ms, throughput_mbps,
  // packet_loss_pct) is supported as a forward-compatibility fallback.
  const latency = perfData?.latency_ms ?? perfData?.avg_latency_ms ?? 0;
  const throughputBps = perfData?.throughput_bps;
  const throughputMbps =
    perfData?.throughput_mbps ??
    (typeof throughputBps === 'number' ? throughputBps / 1_000_000 : 0);
  const packetLossPct =
    perfData?.packet_loss_pct ??
    (typeof perfData?.packet_loss_rate === 'number'
      ? perfData.packet_loss_rate * 100
      : 0);
  const jitter = perfData?.jitter_ms ?? 0;
  const throughputPct = Math.min(100, (throughputMbps / 40_000) * 100);

  return (
    <div className="space-y-4">
      <MetricRow
        label="Throughput"
        value={throughputMbps > 0 ? formatThroughput(throughputMbps) : 'not reported by daemon'}
        percent={throughputPct}
        muted={throughputMbps === 0}
      />
      <MetricRow
        label="Average Latency"
        value={latency > 0 ? `${latency.toFixed(1)} ms` : 'not reported by daemon'}
        percent={Math.max(0, 100 - latency)}
        muted={latency === 0}
      />
      <MetricRow
        label="Packet Loss"
        value={packetLossPct > 0 ? `${packetLossPct.toFixed(3)}%` : 'not reported by daemon'}
        percent={Math.max(0, 100 - packetLossPct * 10)}
        muted={packetLossPct === 0}
      />
      <MetricRow
        label="Jitter"
        value={jitter > 0 ? `${jitter.toFixed(1)} ms` : 'not reported by daemon'}
        percent={Math.max(0, 100 - jitter * 2)}
        muted={jitter === 0}
      />
    </div>
  );
}

function MetricRow({
  label,
  value,
  percent,
  muted,
}: {
  label: string;
  value: string;
  percent: number;
  muted?: boolean;
}) {
  return (
    <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
      <div className="flex justify-between items-center mb-2">
        <span className="font-medium text-white">{label}</span>
        <span className={cn(muted ? 'text-gray-500 italic text-xs' : 'text-cyan-400')}>
          {value}
        </span>
      </div>
      {!muted ? <Progress value={percent} className="h-1" /> : null}
    </div>
  );
}

function formatThroughput(mbps: number): string {
  if (mbps >= 1000) return `${(mbps / 1000).toFixed(2)} Gbps`;
  return `${mbps.toFixed(1)} Mbps`;
}
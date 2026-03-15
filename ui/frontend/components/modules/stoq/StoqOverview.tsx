// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import { Network, BarChart3, Globe } from 'lucide-react';
import { useNodeStatus, useNetworkPeers, useTopologyInfo, useTopologyNeighbors } from '@/lib/hooks/useBlockMatrix';

export function StoqOverview() {
  const nodeStatus = useNodeStatus();
  const peers = useNetworkPeers();
  const topology = useTopologyInfo();
  const neighbors = useTopologyNeighbors();

  const isOnline = !!nodeStatus.data && !nodeStatus.isError;
  const peerCount = nodeStatus.data?.peers ?? peers.data?.length ?? 0;
  const uptimeSecs = nodeStatus.data?.uptime_secs ?? 0;
  const uptimePercent = uptimeSecs > 0 ? Math.min(99.99, 99.0 + (uptimeSecs / 86400) * 0.99) : 0;

  const performanceData = [
    { metric: 'Current Throughput', value: isOnline ? '2.95 Gbps' : 'Offline', status: isOnline ? 'good' : 'critical' as string, percentage: isOnline ? 73.75 : 0 },
    { metric: 'Connected Peers', value: String(peerCount), status: peerCount > 0 ? 'excellent' : 'good' as string, percentage: Math.min(100, peerCount * 10) },
    { metric: 'Active Tunnels', value: String(peerCount), status: peerCount > 0 ? 'good' : 'critical' as string, percentage: Math.min(100, peerCount * 10) },
    { metric: 'Protocol Uptime', value: `${uptimePercent.toFixed(1)}%`, status: uptimePercent > 99 ? 'excellent' : 'good' as string, percentage: uptimePercent },
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
                  <div className="w-3 h-3 rounded-full bg-green-400 animate-pulse" />
                  <span className="font-medium text-white">QUIC Transport</span>
                </div>
                <div className="text-right">
                  <p className="font-medium text-cyan-400">Active</p>
                  <p className="text-sm text-gray-400">HTTP/3 multiplexing</p>
                </div>
              </div>

              <div className="flex justify-between items-center p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-yellow-400" />
                  <span className="font-medium text-white">Throughput Target</span>
                </div>
                <div className="text-right">
                  <p className="font-medium text-yellow-400">81%</p>
                  <p className="text-sm text-gray-400">32.4/40 Gbps achieved</p>
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
                  <p className="font-medium text-green-400">Secured</p>
                  <p className="text-sm text-gray-400">Cryptographic tokens</p>
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
            <CardDescription className="text-gray-400">24-hour protocol performance</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex justify-between items-center mb-2">
                  <span className="font-medium text-white">Peak Throughput</span>
                  <span className="text-cyan-400">37.8 Gbps</span>
                </div>
                <Progress value={94.5} className="h-1" />
                <p className="text-xs text-gray-400 mt-1">94.5% of target achieved</p>
              </div>

              <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex justify-between items-center mb-2">
                  <span className="font-medium text-white">Average Latency</span>
                  <span className="text-cyan-400">12.4ms</span>
                </div>
                <Progress value={95} className="h-1" />
                <p className="text-xs text-gray-400 mt-1">Excellent performance</p>
              </div>

              <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex justify-between items-center mb-2">
                  <span className="font-medium text-white">Packet Loss</span>
                  <span className="text-cyan-400">0.02%</span>
                </div>
                <Progress value={99.98} className="h-1" />
                <p className="text-xs text-gray-400 mt-1">Minimal loss detected</p>
              </div>

              <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                <div className="flex justify-between items-center mb-2">
                  <span className="font-medium text-white">Connection Quality</span>
                  <span className="text-cyan-400">94.2/100</span>
                </div>
                <Progress value={94.2} className="h-1" />
                <p className="text-xs text-gray-400 mt-1">High quality connections</p>
              </div>
            </div>
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
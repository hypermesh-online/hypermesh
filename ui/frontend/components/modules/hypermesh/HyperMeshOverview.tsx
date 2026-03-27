// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useNodeStatus,
  useAssetList,
  useNetworkPeers,
  useBlockchainHeight,
} from '@/lib/hooks/useBlockMatrix';
import {
  Network,
  Activity,
  Database,
  Users,
  AlertTriangle,
  Box,
  Clock,
} from 'lucide-react';

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function HyperMeshOverview() {
  const { data: nodeStatus, isLoading: nodeLoading, error: nodeError } = useNodeStatus();
  const { data: heightData, isLoading: heightLoading } = useBlockchainHeight();
  const { data: peers, isLoading: peersLoading } = useNetworkPeers();
  const { data: assets, isLoading: assetsLoading } = useAssetList();

  const isLoading = nodeLoading && heightLoading && peersLoading && assetsLoading;

  if (isLoading) return <ModuleLoading />;

  if (nodeError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{nodeError.message}</p>
          <p className="text-sm text-gray-500 mt-1">
            Ensure the HyperMesh daemon is running and the gateway is reachable.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center py-4">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          HyperMesh Node Dashboard
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Real-time overview of your node, blockchain, peers, and assets.
        </p>
      </div>

      {/* Summary cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Chain Height</CardTitle>
            <Database className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {heightData?.height ?? nodeStatus?.chain_height ?? '--'}
            </div>
            <p className="text-xs text-gray-400">Blocks on local chain</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Connected Peers</CardTitle>
            <Users className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {peers?.length ?? nodeStatus?.peers ?? 0}
            </div>
            <p className="text-xs text-gray-400">Network nodes</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Registered Assets</CardTitle>
            <Box className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {assets?.length ?? 0}
            </div>
            <p className="text-xs text-gray-400">On-chain assets</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Uptime</CardTitle>
            <Clock className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {nodeStatus ? formatUptime(nodeStatus.uptime_secs) : '--'}
            </div>
            <p className="text-xs text-gray-400">Node runtime</p>
          </CardContent>
        </Card>
      </div>

      {/* Node identity card */}
      {nodeStatus && (
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Network className="h-5 w-5 text-cyan-400" />
              Node Identity
            </CardTitle>
            <CardDescription className="text-gray-400">
              Details about this HyperMesh node
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Node ID</span>
                  <span className="text-white font-mono text-sm">
                    {nodeStatus.node_id.length > 24
                      ? `${nodeStatus.node_id.slice(0, 24)}...`
                      : nodeStatus.node_id}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Matrix Coordinate</span>
                  <span className="text-cyan-400 font-mono">
                    ({nodeStatus.coordinate.x}, {nodeStatus.coordinate.y}, {nodeStatus.coordinate.z})
                  </span>
                </div>
              </div>
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Privacy Mode</span>
                  <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">
                    {nodeStatus.privacy_mode}
                  </Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Status</span>
                  <Badge className="bg-green-500/20 text-green-400 border-green-500/30">
                    Online
                  </Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Recent peers */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Activity className="h-5 w-5 text-cyan-400" />
            Connected Peers
          </CardTitle>
          <CardDescription className="text-gray-400">
            Nodes currently connected via STOQ
          </CardDescription>
        </CardHeader>
        <CardContent>
          {peersLoading ? (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="animate-pulse h-12 bg-gray-700 rounded-lg" />
              ))}
            </div>
          ) : peers && peers.length > 0 ? (
            <div className="space-y-2">
              {peers.map((peer) => (
                <div
                  key={peer.node_id}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 bg-green-400 rounded-full" />
                    <div>
                      <p className="text-sm font-mono text-white">{peer.node_id.slice(0, 16)}...</p>
                      <p className="text-xs text-gray-400">{peer.address}</p>
                    </div>
                  </div>
                  {peer.coordinate && (
                    <Badge variant="outline" className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30">
                      ({peer.coordinate.x},{peer.coordinate.y},{peer.coordinate.z})
                    </Badge>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Users className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No peers connected</p>
              <p className="text-xs text-gray-500 mt-1">
                Peers will appear when other nodes join the network
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

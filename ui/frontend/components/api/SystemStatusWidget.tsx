// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * System Status Widget - Compact node health indicator
 *
 * Uses useNodeStatus from useBlockMatrix hooks.
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { useNodeStatus, useNetworkPeers } from '@/lib/hooks/useBlockMatrix';
import { Activity, AlertTriangle } from 'lucide-react';

export function SystemStatusWidget() {
  const nodeStatus = useNodeStatus();
  const peers = useNetworkPeers();

  if (nodeStatus.isLoading) {
    return (
      <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
        <CardContent className="p-4">
          <div className="space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-6 w-24" />
            <Skeleton className="h-4 w-28" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (nodeStatus.error) {
    return (
      <Card className="bg-red-900/20 border-red-500/30">
        <CardContent className="p-4 flex items-center gap-3">
          <AlertTriangle className="h-5 w-5 text-red-400" />
          <div>
            <p className="text-red-400 font-medium text-sm">System Status Error</p>
            <p className="text-gray-500 text-xs">Failed to connect to node</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  const data = nodeStatus.data;
  const isOnline = !!data;
  const uptimeSecs = data?.uptime_secs ?? 0;
  const peerCount = data?.peers ?? peers.data?.length ?? 0;

  return (
    <Card className="bg-black/40 border-gray-700 backdrop-blur-lg">
      <CardContent className="p-4 space-y-4">
        {/* Overall Health */}
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-white font-medium text-sm">System Health</h3>
            <p className="text-sm text-green-400">
              {isOnline ? 'Online' : 'Offline'}
            </p>
          </div>
          <div className={`w-3 h-3 rounded-full ${isOnline ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
        </div>

        {/* Key Metrics */}
        <div className="grid grid-cols-2 gap-2 text-xs">
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Node ID</div>
            <div className="text-white font-mono truncate">
              {data?.node_id?.slice(0, 12) ?? '--'}
            </div>
          </div>
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Chain Height</div>
            <div className="text-white font-mono">{data?.chain_height ?? 0}</div>
          </div>
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Peers</div>
            <div className="text-white font-mono">{peerCount}</div>
          </div>
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Uptime</div>
            <div className="text-white font-mono">{formatUptime(uptimeSecs)}</div>
          </div>
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Privacy Mode</div>
            <div className="text-white">{data?.privacy_mode ?? '--'}</div>
          </div>
          <div className="bg-black/20 rounded px-3 py-2">
            <div className="text-gray-400">Position</div>
            <div className="text-white font-mono">
              {data?.coordinate
                ? `(${data.coordinate.x},${data.coordinate.y},${data.coordinate.z})`
                : '--'}
            </div>
          </div>
        </div>

        {/* Live Indicator */}
        <div className="flex items-center justify-between text-xs text-gray-500">
          <span>Polling every 10s</span>
          <div className="flex items-center gap-1">
            <div className="w-1 h-1 bg-green-400 rounded-full animate-pulse" />
            <span>Live</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  return `${Math.floor(secs / 86400)}d ${Math.floor((secs % 86400) / 3600)}h`;
}

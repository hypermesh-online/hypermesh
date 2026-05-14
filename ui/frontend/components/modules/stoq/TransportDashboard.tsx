// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import { useStoqStats, useStoqConnections } from '@/lib/hooks/useBlockMatrix';
import { Network, Zap, ArrowUpRight, ArrowDownLeft, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

const colors = moduleColors.stoq;

export function TransportDashboard() {
  const stats = useStoqStats();
  const connections = useStoqConnections();

  if (stats.isLoading && connections.isLoading) {
    return <ModuleLoading />;
  }

  if (stats.error && connections.error) {
    return (
      <div className="p-6">
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">STOQ transport offline</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent`}>
        Transport Dashboard
      </h2>

      {/* Transport Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard
          title="Active Connections"
          value={String(stats.data?.connections ?? stats.data?.connections_active ?? 0)}
          icon={Network}
          loading={stats.isLoading}
        />
        <StatCard
          title="Bytes Sent"
          value={formatBytes(stats.data?.bytes_sent ?? 0)}
          icon={ArrowUpRight}
          loading={stats.isLoading}
        />
        <StatCard
          title="Bytes Received"
          value={formatBytes(stats.data?.bytes_received ?? 0)}
          icon={ArrowDownLeft}
          loading={stats.isLoading}
        />
        <StatCard
          title="Packets Total"
          value={String(
            (stats.data?.packets_sent ?? 0) + (stats.data?.packets_received ?? 0)
          )}
          icon={Zap}
          loading={stats.isLoading}
        />
      </div>

      {/* Connection List */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Network className="h-5 w-5 text-purple-400" />
            Active Connections
          </CardTitle>
          <CardDescription className="text-gray-400">
            STOQ QUIC connections with per-connection byte counters
          </CardDescription>
        </CardHeader>
        <CardContent>
          {connections.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full" />
              ))}
            </div>
          ) : connections.data?.connections && connections.data.connections.length > 0 ? (
            <div className="space-y-2 max-h-96 overflow-y-auto">
              {connections.data.connections.map((conn) => (
                <div
                  key={conn.node_id}
                  className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                >
                  <div>
                    <p className="text-sm font-mono text-white">
                      {conn.node_id.length > 12 ? `${conn.node_id.slice(0, 12)}...` : conn.node_id}
                    </p>
                    <p className="text-xs text-gray-400">{conn.address}</p>
                    {conn.coordinate ? (
                      <p className="text-xs text-gray-500 font-mono">
                        matrix ({conn.coordinate.x}, {conn.coordinate.y}, {conn.coordinate.z})
                      </p>
                    ) : null}
                  </div>
                  <div className="flex items-center gap-3">
                    <Badge className={cn(
                      "text-xs",
                      'bg-cyan-500/20 text-cyan-400',
                    )}>
                      {conn.protocol ?? 'QUIC'}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-400">
              {connections.data?.note ?? 'No active connections'}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function StatCard({
  title,
  value,
  icon: Icon,
  loading,
}: {
  title: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
  loading: boolean;
}) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{title}</CardTitle>
        <Icon className="h-4 w-4 text-purple-400" />
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className="h-8 w-24" />
        ) : (
          <div className="text-2xl font-bold text-purple-400">{value}</div>
        )}
      </CardContent>
    </Card>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useEngaugeCapacity, useEngaugeTraffic } from '@/lib/hooks/useBlockMatrix';
import { Activity, Network, Cpu, HardDrive, BarChart3, ShieldOff, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

export default function EngaugeAnalytics() {
  const capacity = useEngaugeCapacity();
  const traffic = useEngaugeTraffic();

  if (capacity.isLoading && traffic.isLoading) {
    return <ModuleLoading />;
  }

  if (capacity.error && traffic.error) {
    return (
      <div className="p-6 space-y-6">
        <h2 className="text-2xl font-bold text-white">Traffic Analytics</h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the analytics backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className="text-2xl font-bold text-white">Traffic Analytics</h2>

      {/* Capacity Breakdown */}
      <div className="grid gap-4 md:grid-cols-4">
        {[
          { label: 'CPU Usage', value: capacity.data?.cpu_usage, icon: Cpu },
          { label: 'Memory', value: capacity.data?.memory_usage, icon: Activity },
          { label: 'Storage', value: capacity.data?.storage_usage, icon: HardDrive },
          { label: 'Network', value: capacity.data?.network_usage, icon: Network },
        ].map(({ label, value, icon: Icon }) => (
          <Card key={label} className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-white">{label}</CardTitle>
              <Icon className="h-4 w-4 text-orange-400" />
            </CardHeader>
            <CardContent>
              {capacity.isLoading ? (
                <Skeleton className="h-8 w-24" />
              ) : capacity.error ? (
                <span className="text-sm text-gray-500">--</span>
              ) : (
                <>
                  <div className="text-2xl font-bold text-orange-400">
                    {value != null ? `${(value * 100).toFixed(1)}%` : '--'}
                  </div>
                  {value != null && (
                    <Progress value={value * 100} className="mt-2 h-1" />
                  )}
                </>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Traffic Details */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <BarChart3 className="h-5 w-5 text-orange-400" />
            Traffic Breakdown
          </CardTitle>
        </CardHeader>
        <CardContent>
          {traffic.isLoading ? (
            <Skeleton className="h-32 w-full" />
          ) : traffic.error ? (
            <p className="text-gray-500 text-center py-8">Traffic analysis unavailable</p>
          ) : traffic.data ? (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-3">
                <div className="p-4 rounded-lg bg-green-500/10 border border-green-500/30">
                  <div className="text-2xl font-bold text-green-400">
                    {formatBytes(traffic.data.bytes_in)}
                  </div>
                  <div className="text-sm text-gray-400">Bytes In</div>
                  <div className="text-xs text-gray-500 mt-1">
                    {traffic.data.packets_in.toLocaleString()} packets
                  </div>
                </div>
                <div className="p-4 rounded-lg bg-blue-500/10 border border-blue-500/30">
                  <div className="text-2xl font-bold text-blue-400">
                    {formatBytes(traffic.data.bytes_out)}
                  </div>
                  <div className="text-sm text-gray-400">Bytes Out</div>
                  <div className="text-xs text-gray-500 mt-1">
                    {traffic.data.packets_out.toLocaleString()} packets
                  </div>
                </div>
                <div className="p-4 rounded-lg bg-orange-500/10 border border-orange-500/30">
                  <div className="text-2xl font-bold text-orange-400">
                    {traffic.data.active_flows}
                  </div>
                  <div className="text-sm text-gray-400">Active Flows</div>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-gray-500 text-center py-8">No traffic data</p>
          )}
        </CardContent>
      </Card>

      {/* Privacy Mode Sharing Matrix */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <ShieldOff className="h-5 w-5 text-orange-400" />
            Privacy Mode Sharing Matrix
          </CardTitle>
          <CardDescription className="text-gray-400">
            What metrics are shared per privacy mode
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-800">
                  <th className="text-left py-2 text-gray-400">Frame Type</th>
                  <th className="text-center py-2 text-gray-400">Anonymous</th>
                  <th className="text-center py-2 text-gray-400">Private</th>
                  <th className="text-center py-2 text-gray-400">Public</th>
                </tr>
              </thead>
              <tbody>
                {[
                  { type: 'Capacity', anon: false, priv: true, pub: true },
                  { type: 'Congestion', anon: false, priv: true, pub: true },
                  { type: 'Routing', anon: false, priv: false, pub: true },
                  { type: 'Economic', anon: false, priv: false, pub: true },
                ].map((row) => (
                  <tr key={row.type} className="border-b border-gray-800/50">
                    <td className="py-2 text-white">{row.type}</td>
                    <td className="py-2 text-center">
                      <YesNoBadge yes={row.anon} />
                    </td>
                    <td className="py-2 text-center">
                      <YesNoBadge yes={row.priv} />
                    </td>
                    <td className="py-2 text-center">
                      <YesNoBadge yes={row.pub} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      {/* Total Capacity */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Total Capacity</CardTitle>
        </CardHeader>
        <CardContent>
          {capacity.isLoading ? (
            <Skeleton className="h-12 w-full" />
          ) : (
            <div className="flex items-center justify-between p-4 rounded-lg bg-black/20 border border-gray-800">
              <span className="text-gray-400">Total Capacity Score</span>
              <span className="text-2xl font-bold text-orange-400">
                {capacity.data?.total_capacity != null
                  ? `${(capacity.data.total_capacity * 100).toFixed(1)}%`
                  : '--'}
              </span>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function YesNoBadge({ yes }: { yes: boolean }) {
  return yes ? (
    <Badge className="bg-green-500/20 text-green-400">Yes</Badge>
  ) : (
    <Badge className="bg-gray-600/20 text-gray-500">No</Badge>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

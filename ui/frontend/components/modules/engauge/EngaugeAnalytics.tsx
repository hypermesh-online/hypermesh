// @ts-nocheck — Phase 8 will rewrite with useBlockMatrix hooks
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { Eye, ShieldOff, BarChart3, Activity, HardDrive, Cpu, Network, AlertTriangle, TrendingUp, TrendingDown, Minus } from 'lucide-react';
import { useTrafficAnalysis, useMetricsStream, useCapacityMetrics, useTrendingMetrics } from '@/lib/api';
import type { MetricsFrameType } from '@/lib/api';
import { cn } from '@/lib/utils';

export default function EngaugeAnalytics() {
  const traffic = useTrafficAnalysis();
  const metrics = useMetricsStream();
  const capacity = useCapacityMetrics();
  const trending = useTrendingMetrics();

  const allErrored = traffic.error && capacity.error && trending.error && metrics.error;

  if (allErrored) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Traffic Analytics</h2>
        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the analytics backend. Check that engauge is running.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Traffic Analytics</h2>

      {/* Capacity Metrics */}
      <div className="grid gap-4 md:grid-cols-4">
        {[
          { label: 'Bytes Served', value: capacity.data?.bytes_served, format: (v: number) => `${(v / (1024*1024*1024)).toFixed(2)} GB`, icon: Network },
          { label: 'Compute', value: capacity.data?.compute_delivered, format: (v: number) => `${v.toFixed(1)} CPU-s`, icon: Cpu },
          { label: 'Storage', value: capacity.data?.storage_committed, format: (v: number) => `${(v / (1024*1024*1024)).toFixed(2)} GB`, icon: HardDrive },
          { label: 'Utilization', value: capacity.data?.utilization_percent, format: (v: number) => `${v.toFixed(1)}%`, icon: Activity },
        ].map(({ label, value, format, icon: Icon }) => (
          <Card key={label} className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-white">{label}</CardTitle>
              <Icon className="h-4 w-4 text-orange-400" />
            </CardHeader>
            <CardContent>
              {capacity.isLoading ? <Skeleton className="h-8 w-24" /> : capacity.error ? (
                <span className="text-sm text-gray-500">--</span>
              ) : (
                <div className="text-2xl font-bold text-orange-400">
                  {value != null ? format(value) : '--'}
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Organic vs Speculative */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Eye className="h-5 w-5 text-orange-400" />
            Organic vs Speculative Traffic
          </CardTitle>
        </CardHeader>
        <CardContent>
          {traffic.isLoading ? <Skeleton className="h-32 w-full" /> : traffic.error ? (
            <p className="text-gray-500 text-center py-8">Traffic analysis unavailable</p>
          ) : traffic.data ? (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-3">
                <div className="p-4 rounded-lg bg-green-500/10 border border-green-500/30">
                  <div className="text-2xl font-bold text-green-400">{traffic.data.organic_count}</div>
                  <div className="text-sm text-gray-400">Organic Requests</div>
                </div>
                <div className="p-4 rounded-lg bg-yellow-500/10 border border-yellow-500/30">
                  <div className="text-2xl font-bold text-yellow-400">{traffic.data.speculative_count}</div>
                  <div className="text-sm text-gray-400">Speculative Requests</div>
                </div>
                <div className="p-4 rounded-lg bg-blue-500/10 border border-blue-500/30">
                  <div className="text-2xl font-bold text-blue-400">{(traffic.data.confidence * 100).toFixed(1)}%</div>
                  <div className="text-sm text-gray-400">Detection Confidence</div>
                </div>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-gray-400">Organic Rate</span>
                  <span className="text-green-400">{(traffic.data.organic_rate * 100).toFixed(1)}%</span>
                </div>
                <Progress value={traffic.data.organic_rate * 100} className="h-2" />
              </div>
            </div>
          ) : <p className="text-gray-500 text-center py-8">No traffic data</p>}
        </CardContent>
      </Card>

      {/* Privacy Mode Matrix */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <ShieldOff className="h-5 w-5 text-orange-400" />
            Privacy Mode Sharing Matrix
          </CardTitle>
          <CardDescription className="text-gray-400">What metrics are shared per privacy mode</CardDescription>
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
                ].map(row => (
                  <tr key={row.type} className="border-b border-gray-800/50">
                    <td className="py-2 text-white">{row.type}</td>
                    <td className="py-2 text-center">{row.anon ? <Badge className="bg-green-500/20 text-green-400">Yes</Badge> : <Badge className="bg-gray-600/20 text-gray-500">No</Badge>}</td>
                    <td className="py-2 text-center">{row.priv ? <Badge className="bg-green-500/20 text-green-400">Yes</Badge> : <Badge className="bg-gray-600/20 text-gray-500">No</Badge>}</td>
                    <td className="py-2 text-center">{row.pub ? <Badge className="bg-green-500/20 text-green-400">Yes</Badge> : <Badge className="bg-gray-600/20 text-gray-500">No</Badge>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      {/* Trending Metrics */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <TrendingUp className="h-5 w-5 text-orange-400" />
            Trending Metrics
          </CardTitle>
        </CardHeader>
        <CardContent>
          {trending.isLoading ? <Skeleton className="h-32 w-full" /> : trending.error ? (
            <p className="text-gray-500 text-center py-4">Trending data unavailable</p>
          ) : (
            <div className="grid gap-3 md:grid-cols-2">
              {trending.data?.map((m) => (
                <div key={m.metric_name} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                  <div>
                    <div className="text-sm text-white">{m.metric_name}</div>
                    <div className="text-xs text-gray-400">{m.current_value.toFixed(2)}</div>
                  </div>
                  <div className="flex items-center gap-1">
                    {m.trend_direction === 'up' ? <TrendingUp className="h-4 w-4 text-green-400" /> :
                     m.trend_direction === 'down' ? <TrendingDown className="h-4 w-4 text-red-400" /> :
                     <Minus className="h-4 w-4 text-gray-400" />}
                    <span className={cn("text-sm font-bold",
                      m.trend_direction === 'up' ? 'text-green-400' :
                      m.trend_direction === 'down' ? 'text-red-400' : 'text-gray-400'
                    )}>
                      {m.change_percent > 0 ? '+' : ''}{m.change_percent.toFixed(1)}%
                    </span>
                  </div>
                </div>
              )) || <p className="text-gray-500 text-center py-4">No trending data</p>}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Metrics Stream */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <BarChart3 className="h-5 w-5 text-orange-400" />
            Live Metrics Stream
          </CardTitle>
        </CardHeader>
        <CardContent>
          {metrics.isLoading ? <Skeleton className="h-32 w-full" /> : metrics.error ? (
            <p className="text-gray-500 text-center py-4">Metrics stream unavailable</p>
          ) : (
            <div className="space-y-2">
              {metrics.data?.slice(0, 10).map((frame, i) => (
                <div key={i} className="flex items-center justify-between p-2 rounded bg-black/20 border border-gray-800">
                  <Badge className="bg-orange-500/20 text-orange-400">{frame.frame_type}</Badge>
                  <span className="text-xs text-gray-400">{new Date(frame.timestamp).toLocaleTimeString()}</span>
                  {frame.privacy_filtered && <Badge className="bg-purple-500/20 text-purple-400 text-xs">Filtered</Badge>}
                </div>
              )) || <p className="text-gray-500 text-center py-4">No stream data</p>}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

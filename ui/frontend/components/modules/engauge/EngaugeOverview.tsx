// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { Activity, HardDrive, Cpu, Network, TrendingUp, TrendingDown, Minus, Gauge, AlertTriangle } from 'lucide-react';
import { useEngaugeOverview } from '@/lib/api';
import { cn } from '@/lib/utils';

export default function EngaugeOverview() {
  const { capacity, traffic, trending, throttle, pools, isLoading, error } = useEngaugeOverview();

  if (error && !isLoading && !capacity.data && !traffic.data && !trending.data && !throttle.data && !pools.data) {
    return (
      <div className="space-y-6">
        <div className="text-center py-4">
          <h2 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-red-600 bg-clip-text text-transparent">
            Capacity & Analytics Overview
          </h2>
        </div>
        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the engauge backend. Check that the service is running.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center py-4">
        <h2 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-red-600 bg-clip-text text-transparent">
          Capacity & Analytics Overview
        </h2>
      </div>

      {/* Capacity Cards */}
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
              {isLoading ? <Skeleton className="h-8 w-24" /> : (
                <div className="text-2xl font-bold text-orange-400">
                  {value != null ? format(value) : '--'}
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Throttle Gauge */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Gauge className="h-5 w-5 text-orange-400" />
            Governor Throttle
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? <Skeleton className="h-8 w-full" /> : (
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Signal</span>
                <span className={cn(
                  "font-bold",
                  (throttle.data?.governor_signal || 0) > 0.8 ? 'text-red-400' :
                  (throttle.data?.governor_signal || 0) > 0.5 ? 'text-yellow-400' : 'text-green-400'
                )}>
                  {((throttle.data?.governor_signal || 0) * 100).toFixed(1)}%
                </span>
              </div>
              <Progress value={(throttle.data?.governor_signal || 0) * 100} className="h-3" />
              {throttle.data?.is_throttled && (
                <Badge className="bg-red-500/20 text-red-400">Throttled: {throttle.data.reason}</Badge>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Trending Metrics */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Trending Metrics</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? <Skeleton className="h-32 w-full" /> : (
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
    </div>
  );
}

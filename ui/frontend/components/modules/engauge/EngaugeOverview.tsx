// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import { useEngaugeCapacity, useEngaugeTraffic, useEngaugeThrottle } from '@/lib/hooks/useBlockMatrix';
import { Activity, Network, Cpu, HardDrive, Gauge, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

const colors = moduleColors.engauge;

export default function EngaugeOverview() {
  const capacity = useEngaugeCapacity();
  const traffic = useEngaugeTraffic();
  const throttle = useEngaugeThrottle();

  if (capacity.isLoading && traffic.isLoading) {
    return <ModuleLoading />;
  }

  if (capacity.error && traffic.error && throttle.error) {
    return (
      <div className="p-6 space-y-6">
        <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent text-center`}>
          Capacity & Analytics Overview
        </h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the engauge backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const signalValue = capacity.data?.network_usage ?? 0;
  const signalPct = Math.min(signalValue * 100, 100);

  return (
    <div className="p-6 space-y-6">
      <div className="text-center py-4">
        <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent`}>
          Capacity & Analytics Overview
        </h2>
      </div>

      {/* Capacity Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <CapacityCard
          label="CPU Usage"
          value={capacity.data?.cpu_usage}
          format={(v) => `${(v * 100).toFixed(1)}%`}
          icon={Cpu}
          loading={capacity.isLoading}
        />
        <CapacityCard
          label="Memory"
          value={capacity.data?.memory_usage}
          format={(v) => `${(v * 100).toFixed(1)}%`}
          icon={Activity}
          loading={capacity.isLoading}
        />
        <CapacityCard
          label="Storage"
          value={capacity.data?.storage_usage}
          format={(v) => `${(v * 100).toFixed(1)}%`}
          icon={HardDrive}
          loading={capacity.isLoading}
        />
        <CapacityCard
          label="Network"
          value={capacity.data?.network_usage}
          format={(v) => `${(v * 100).toFixed(1)}%`}
          icon={Network}
          loading={capacity.isLoading}
        />
      </div>

      {/* Traffic Summary */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Network className="h-5 w-5 text-orange-400" />
            Traffic Summary
          </CardTitle>
        </CardHeader>
        <CardContent>
          {traffic.isLoading ? (
            <Skeleton className="h-24 w-full" />
          ) : traffic.error ? (
            <p className="text-gray-500 text-center py-4">Traffic data unavailable</p>
          ) : (
            <div className="grid gap-4 md:grid-cols-3">
              <MetricBlock
                label="Bytes In"
                value={formatBytes(traffic.data?.bytes_in ?? 0)}
                color="text-green-400"
              />
              <MetricBlock
                label="Bytes Out"
                value={formatBytes(traffic.data?.bytes_out ?? 0)}
                color="text-blue-400"
              />
              <MetricBlock
                label="Active Flows"
                value={String(traffic.data?.active_flows ?? 0)}
                color="text-orange-400"
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Throttle Gauge */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Gauge className="h-5 w-5 text-orange-400" />
            Governor Throttle
          </CardTitle>
        </CardHeader>
        <CardContent>
          {throttle.isLoading ? (
            <Skeleton className="h-8 w-full" />
          ) : throttle.error ? (
            <p className="text-gray-500 text-center py-4">Throttle data unavailable</p>
          ) : (
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Current Rate</span>
                <span className={cn(
                  "font-bold",
                  throttle.data?.is_throttled ? 'text-red-400' : 'text-green-400',
                )}>
                  {throttle.data?.current_rate ?? 0} / {throttle.data?.max_rate ?? 0}
                </span>
              </div>
              <Progress
                value={throttle.data?.max_rate
                  ? ((throttle.data?.current_rate ?? 0) / throttle.data.max_rate) * 100
                  : 0}
                className="h-3"
              />
              {throttle.data?.is_throttled ? (
                <Badge className="bg-red-500/20 text-red-400">
                  Throttled{throttle.data?.reason ? `: ${throttle.data.reason}` : ''}
                </Badge>
              ) : (
                <Badge className="bg-green-500/20 text-green-400">Normal operation</Badge>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function CapacityCard({
  label,
  value,
  format,
  icon: Icon,
  loading,
}: {
  label: string;
  value: number | undefined;
  format: (v: number) => string;
  icon: React.ComponentType<{ className?: string }>;
  loading: boolean;
}) {
  return (
    <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{label}</CardTitle>
        <Icon className="h-4 w-4 text-orange-400" />
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className="h-8 w-24" />
        ) : (
          <div className="text-2xl font-bold text-orange-400">
            {value != null ? format(value) : '--'}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function MetricBlock({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
      <div className={`text-2xl font-bold ${color}`}>{value}</div>
      <div className="text-sm text-gray-400">{label}</div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

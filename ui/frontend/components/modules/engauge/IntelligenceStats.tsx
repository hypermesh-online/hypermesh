// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useEngaugeCapacity, useEngaugeTraffic, useEngaugeThrottle } from '@/lib/hooks/useBlockMatrix';
import { Brain, Zap, Activity, AlertTriangle } from 'lucide-react';

export default function IntelligenceStats() {
  const capacity = useEngaugeCapacity();
  const traffic = useEngaugeTraffic();
  const throttle = useEngaugeThrottle();

  if (capacity.isLoading && traffic.isLoading) {
    return <ModuleLoading />;
  }

  if (capacity.error && traffic.error && throttle.error) {
    return (
      <div className="p-6 space-y-6">
        <h2 className="text-2xl font-bold text-white">Intelligence Stats</h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className="text-2xl font-bold text-white">Intelligence Stats</h2>

      {/* Intelligence Loop Status */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Brain className="h-5 w-5 text-orange-400" />
            Intelligence Loop (H1-H7)
          </CardTitle>
          <CardDescription className="text-gray-400">
            MetricsReporter, EngaugeBridge, PropagationWeight, ReplicationTrigger
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 md:grid-cols-2">
            <StatusRow
              label="Engauge Feed"
              status={!capacity.error}
              detail="10s periodic capacity ingestion"
            />
            <StatusRow
              label="Throttle Controller"
              status={!throttle.error}
              detail={throttle.data?.is_throttled ? 'Active - throttled' : 'Normal'}
            />
            <StatusRow
              label="Traffic Analysis"
              status={!traffic.error}
              detail={`${traffic.data?.active_flows ?? 0} active flows`}
            />
            <StatusRow
              label="Capacity Monitor"
              status={!capacity.error}
              detail={capacity.data
                ? `${((capacity.data.total_capacity ?? 0) * 100).toFixed(0)}% total`
                : 'Waiting'}
            />
          </div>
        </CardContent>
      </Card>

      {/* Resource Utilization Summary */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Activity className="h-5 w-5 text-orange-400" />
            Resource Utilization
          </CardTitle>
        </CardHeader>
        <CardContent>
          {capacity.isLoading ? (
            <Skeleton className="h-32 w-full" />
          ) : capacity.data ? (
            <div className="grid gap-3 md:grid-cols-4">
              <UtilCard label="CPU" value={capacity.data.cpu_usage} />
              <UtilCard label="Memory" value={capacity.data.memory_usage} />
              <UtilCard label="Storage" value={capacity.data.storage_usage} />
              <UtilCard label="Network" value={capacity.data.network_usage} />
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No capacity data</p>
          )}
        </CardContent>
      </Card>

      {/* Network Flow Summary */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Zap className="h-5 w-5 text-orange-400" />
            Network Flow Summary
          </CardTitle>
        </CardHeader>
        <CardContent>
          {traffic.isLoading ? (
            <Skeleton className="h-16 w-full" />
          ) : traffic.data ? (
            <div className="grid gap-3 md:grid-cols-3">
              <div className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center">
                <div className="text-xl font-bold text-green-400">
                  {traffic.data.packets_in.toLocaleString()}
                </div>
                <div className="text-xs text-gray-400">Packets In</div>
              </div>
              <div className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center">
                <div className="text-xl font-bold text-blue-400">
                  {traffic.data.packets_out.toLocaleString()}
                </div>
                <div className="text-xs text-gray-400">Packets Out</div>
              </div>
              <div className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center">
                <div className="text-xl font-bold text-orange-400">
                  {traffic.data.active_flows}
                </div>
                <div className="text-xs text-gray-400">Active Flows</div>
              </div>
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No traffic data</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function StatusRow({
  label,
  status,
  detail,
}: {
  label: string;
  status: boolean;
  detail: string;
}) {
  return (
    <div className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${status ? 'bg-green-400' : 'bg-red-400'}`} />
        <span className="text-sm text-white">{label}</span>
      </div>
      <span className="text-xs text-gray-400">{detail}</span>
    </div>
  );
}

function UtilCard({ label, value }: { label: string; value: number }) {
  const pct = (value * 100).toFixed(1);
  const color = value > 0.8 ? 'text-red-400' : value > 0.5 ? 'text-yellow-400' : 'text-green-400';
  return (
    <div className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center">
      <div className={`text-xl font-bold ${color}`}>{pct}%</div>
      <div className="text-xs text-gray-400">{label}</div>
    </div>
  );
}

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import { useStoqPerformance } from '@/lib/hooks/useBlockMatrix';
import { Gauge, Zap, Timer, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

const colors = moduleColors.stoq;

export function PerformanceView() {
  const performance = useStoqPerformance();

  if (performance.isLoading) {
    return <ModuleLoading />;
  }

  if (performance.error) {
    return (
      <div className="p-6">
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Performance data unavailable</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Daemon `stoq.performance` returns `avg_latency_ms`, `throughput_bps`,
  // `packet_loss_rate` today. The legacy `*_ms / *_mbps / *_pct` fields are
  // retained as a fallback so consumers can continue to render the same
  // dashboard if the daemon contract is extended.
  const perf = performance.data;
  const throughputBps = perf?.throughput_bps;
  const throughputMbps =
    perf?.throughput_mbps ??
    (typeof throughputBps === 'number' ? throughputBps / 1_000_000 : 0);
  const throughputPct = Math.min(100, (throughputMbps / 40000) * 100);
  const latency = perf?.latency_ms ?? perf?.avg_latency_ms ?? 0;
  const packetLoss =
    perf?.packet_loss_pct ??
    (typeof perf?.packet_loss_rate === 'number' ? perf.packet_loss_rate * 100 : 0);
  const jitter = perf?.jitter_ms ?? 0;

  return (
    <div className="p-6 space-y-6">
      <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent`}>
        Performance View
      </h2>

      {/* Primary Metrics */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Throughput</CardTitle>
            <Zap className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold font-mono ${getThroughputColor(throughputPct)}`}>
              {formatThroughput(throughputMbps)}
            </div>
            <Progress value={throughputPct} className="mt-2 h-1" />
            <p className="text-xs text-gray-400 mt-1">
              {throughputPct.toFixed(1)}% of 40 Gbps target
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Latency</CardTitle>
            <Timer className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className={cn(
              "text-2xl font-bold font-mono",
              latency < 10 ? 'text-green-400' :
              latency < 50 ? 'text-blue-400' :
              latency < 100 ? 'text-yellow-400' : 'text-red-400',
            )}>
              {latency.toFixed(1)} ms
            </div>
            <p className="text-xs text-gray-400 mt-1">Jitter: {jitter.toFixed(1)} ms</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Packet Loss</CardTitle>
            <Gauge className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className={cn(
              "text-2xl font-bold font-mono",
              packetLoss < 0.1 ? 'text-green-400' :
              packetLoss < 1 ? 'text-yellow-400' : 'text-red-400',
            )}>
              {packetLoss.toFixed(3)}%
            </div>
            <p className="text-xs text-gray-400 mt-1">
              {packetLoss < 0.1 ? 'Excellent' : packetLoss < 1 ? 'Acceptable' : 'High loss'}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Performance Grade */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Gauge className="h-5 w-5 text-purple-400" />
            Performance Grade
          </CardTitle>
          <CardDescription className="text-gray-400">
            Overall transport quality assessment
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-4">
            <GradeItem
              label="Throughput"
              value={throughputPct}
              grade={getGrade(throughputPct)}
            />
            <GradeItem
              label="Latency"
              value={Math.max(0, 100 - latency)}
              grade={getGrade(Math.max(0, 100 - latency))}
            />
            <GradeItem
              label="Stability"
              value={Math.max(0, 100 - packetLoss * 100)}
              grade={getGrade(Math.max(0, 100 - packetLoss * 100))}
            />
            <GradeItem
              label="Jitter"
              value={Math.max(0, 100 - jitter * 2)}
              grade={getGrade(Math.max(0, 100 - jitter * 2))}
            />
          </div>
        </CardContent>
      </Card>

      {/* Live Indicator */}
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>Polling every 5s</span>
        <div className="flex items-center gap-1">
          <div className="w-1 h-1 bg-green-400 rounded-full animate-pulse" />
          <span>Live Updates</span>
        </div>
      </div>
    </div>
  );
}

function GradeItem({
  label,
  value,
  grade,
}: {
  label: string;
  value: number;
  grade: string;
}) {
  return (
    <div className="p-4 rounded-lg bg-black/20 border border-gray-800 text-center">
      <div className="text-sm text-gray-400 mb-2">{label}</div>
      <Badge className={cn(
        "text-lg px-3 py-1",
        grade === 'A' ? 'bg-green-500/20 text-green-400' :
        grade === 'B' ? 'bg-blue-500/20 text-blue-400' :
        grade === 'C' ? 'bg-yellow-500/20 text-yellow-400' :
        'bg-red-500/20 text-red-400',
      )}>
        {grade}
      </Badge>
      <Progress value={value} className="mt-2 h-1" />
    </div>
  );
}

function getGrade(score: number): string {
  if (score >= 90) return 'A';
  if (score >= 75) return 'B';
  if (score >= 50) return 'C';
  return 'D';
}

function getThroughputColor(pct: number): string {
  if (pct >= 90) return 'text-green-400';
  if (pct >= 75) return 'text-blue-400';
  if (pct >= 50) return 'text-yellow-400';
  return 'text-orange-400';
}

function formatThroughput(mbps: number): string {
  if (mbps >= 1000) return `${(mbps / 1000).toFixed(2)} Gbps`;
  return `${mbps.toFixed(1)} Mbps`;
}

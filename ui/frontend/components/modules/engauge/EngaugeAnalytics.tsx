// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { Eye, ShieldOff, BarChart3 } from 'lucide-react';
import { useTrafficAnalysis, useMetricsStream } from '@/lib/api';
import type { MetricsFrameType } from '@/lib/api';
import { cn } from '@/lib/utils';

export default function EngaugeAnalytics() {
  const traffic = useTrafficAnalysis();
  const metrics = useMetricsStream();

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Traffic Analytics</h2>

      {/* Organic vs Speculative */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Eye className="h-5 w-5 text-orange-400" />
            Organic vs Speculative Traffic
          </CardTitle>
        </CardHeader>
        <CardContent>
          {traffic.isLoading ? <Skeleton className="h-32 w-full" /> : traffic.data ? (
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

      {/* Metrics Stream */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <BarChart3 className="h-5 w-5 text-orange-400" />
            Live Metrics Stream
          </CardTitle>
        </CardHeader>
        <CardContent>
          {metrics.isLoading ? <Skeleton className="h-32 w-full" /> : (
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

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useNGaugeRouting, useNGaugeThrottle } from '@/lib/hooks/useBlockMatrix';
import { Route, Gauge, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

export default function NGaugeRouting() {
  const advisory = useNGaugeRouting();
  const throttle = useNGaugeThrottle();

  if (advisory.isLoading && throttle.isLoading) {
    return <ModuleLoading />;
  }

  if (advisory.error && throttle.error) {
    return (
      <div className="p-6 space-y-6">
        <h2 className="text-2xl font-bold text-white">Routing Intelligence</h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">NGauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the routing intelligence backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className="text-2xl font-bold text-white">Routing Intelligence</h2>

      {/* Routing Advisory */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Route className="h-5 w-5 text-orange-400" />
            Routing Advisory
          </CardTitle>
          <CardDescription className="text-gray-400">
            Tensor-weighted routing with congestion forecasting
          </CardDescription>
        </CardHeader>
        <CardContent>
          {advisory.isLoading ? (
            <Skeleton className="h-48 w-full" />
          ) : advisory.error ? (
            <p className="text-gray-500 text-center py-8">Routing advisory unavailable</p>
          ) : advisory.data ? (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Congestion Level</div>
                  <div className="flex items-center gap-2">
                    <Progress
                      value={advisory.data.congestion_level * 100}
                      className="h-2 flex-1"
                    />
                    <span className={cn(
                      "text-sm font-bold",
                      advisory.data.congestion_level > 0.7 ? 'text-red-400' :
                      advisory.data.congestion_level > 0.4 ? 'text-yellow-400' :
                      'text-green-400',
                    )}>
                      {(advisory.data.congestion_level * 100).toFixed(1)}%
                    </span>
                  </div>
                </div>
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Recommended Paths</div>
                  <div className="text-2xl font-bold text-orange-400">
                    {advisory.data.recommended_paths?.length ?? 0}
                  </div>
                </div>
              </div>

              {/* Recommended Paths */}
              {advisory.data.recommended_paths && advisory.data.recommended_paths.length > 0 && (
                <div className="space-y-2">
                  <h4 className="text-sm text-gray-400 font-medium">Path Recommendations</h4>
                  {advisory.data.recommended_paths.slice(0, 5).map((path, i) => (
                    <div
                      key={i}
                      className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                    >
                      <span className="text-sm text-white font-mono truncate max-w-[200px]">
                        {path.destination}
                      </span>
                      <Badge className="bg-orange-500/20 text-orange-400 border-orange-500/30">
                        metric: {path.metric.toFixed(2)}
                      </Badge>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-8">No routing advisory data</p>
          )}
        </CardContent>
      </Card>

      {/* Governor Throttle */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Gauge className="h-5 w-5 text-orange-400" />
            Governor Throttle Status
          </CardTitle>
          <CardDescription className="text-gray-400">
            PID controller signal and throttle state
          </CardDescription>
        </CardHeader>
        <CardContent>
          {throttle.isLoading ? (
            <Skeleton className="h-16 w-full" />
          ) : throttle.error ? (
            <p className="text-gray-500 text-center py-4">Throttle data unavailable</p>
          ) : throttle.data ? (
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Current Rate</span>
                <span className={cn(
                  "font-bold",
                  throttle.data.is_throttled ? 'text-red-400' : 'text-green-400',
                )}>
                  {throttle.data.current_rate} / {throttle.data.max_rate}
                </span>
              </div>
              <Progress
                value={throttle.data.max_rate
                  ? (throttle.data.current_rate / throttle.data.max_rate) * 100
                  : 0}
                className="h-3"
              />
              {throttle.data.is_throttled ? (
                <Badge className="bg-red-500/20 text-red-400">
                  Throttled{throttle.data.reason ? `: ${throttle.data.reason}` : ''}
                </Badge>
              ) : (
                <Badge className="bg-green-500/20 text-green-400">Normal operation</Badge>
              )}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No throttle data</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

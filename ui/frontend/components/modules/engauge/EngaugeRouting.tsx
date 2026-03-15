// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { Route, Signal, Gauge, AlertTriangle } from 'lucide-react';
import { useRoutingAdvisory, useThrottleStatus } from '@/lib/api';
import { cn } from '@/lib/utils';

export default function EngaugeRouting() {
  const advisory = useRoutingAdvisory();
  const throttle = useThrottleStatus();

  const allErrored = advisory.error && throttle.error;

  if (allErrored) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Routing Intelligence</h2>
        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the routing intelligence backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Routing Intelligence</h2>

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
          {advisory.isLoading ? <Skeleton className="h-48 w-full" /> : advisory.error ? (
            <p className="text-gray-500 text-center py-8">Routing advisory unavailable</p>
          ) : advisory.data ? (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Tensor Weight Modifier</div>
                  <div className="text-2xl font-bold text-orange-400">{advisory.data.tensor_weight_modifier.toFixed(4)}</div>
                </div>
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Path Policy</div>
                  <div className="text-xl font-bold text-white">{advisory.data.path_policy}</div>
                </div>
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Congestion Forecast</div>
                  <div className="flex items-center gap-2">
                    <Progress value={advisory.data.congestion_forecast * 100} className="h-2 flex-1" />
                    <span className={cn("text-sm font-bold",
                      advisory.data.congestion_forecast > 0.7 ? 'text-red-400' :
                      advisory.data.congestion_forecast > 0.4 ? 'text-yellow-400' : 'text-green-400'
                    )}>
                      {(advisory.data.congestion_forecast * 100).toFixed(1)}%
                    </span>
                  </div>
                </div>
                <div className="p-4 rounded-lg bg-black/20 border border-gray-800">
                  <div className="text-sm text-gray-400 mb-1">Alternate Paths</div>
                  <div className="text-2xl font-bold text-orange-400">{advisory.data.alternate_paths}</div>
                </div>
              </div>
              <div className="p-3 rounded-lg bg-orange-500/10 border border-orange-500/30">
                <div className="text-sm text-gray-400">Recommended Tier</div>
                <Badge className="mt-1">{advisory.data.recommended_tier}</Badge>
              </div>
            </div>
          ) : <p className="text-gray-500 text-center py-8">No routing advisory data</p>}
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
          {throttle.isLoading ? <Skeleton className="h-16 w-full" /> : throttle.error ? (
            <p className="text-gray-500 text-center py-4">Throttle data unavailable</p>
          ) : throttle.data ? (
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Governor Signal</span>
                <span className={cn(
                  "font-bold",
                  throttle.data.governor_signal > 0.8 ? 'text-red-400' :
                  throttle.data.governor_signal > 0.5 ? 'text-yellow-400' : 'text-green-400'
                )}>
                  {(throttle.data.governor_signal * 100).toFixed(1)}%
                </span>
              </div>
              <Progress value={throttle.data.governor_signal * 100} className="h-3" />
              {throttle.data.is_throttled ? (
                <Badge className="bg-red-500/20 text-red-400">Throttled: {throttle.data.reason}</Badge>
              ) : (
                <Badge className="bg-green-500/20 text-green-400">Normal operation</Badge>
              )}
            </div>
          ) : <p className="text-gray-500 text-center py-4">No throttle data</p>}
        </CardContent>
      </Card>
    </div>
  );
}

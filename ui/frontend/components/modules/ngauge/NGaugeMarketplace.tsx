// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useNGaugeCapacity } from '@/lib/hooks/useBlockMatrix';
import { Store, Info, AlertTriangle } from 'lucide-react';

export default function NGaugeMarketplace() {
  const capacity = useNGaugeCapacity();

  if (capacity.isLoading) {
    return <ModuleLoading />;
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className="text-2xl font-bold text-white">Resource Marketplace</h2>

      {/* Alpha Notice */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Info className="h-5 w-5 text-orange-400" />
            Alpha Status
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="p-4 rounded-lg bg-orange-500/10 border border-orange-500/20">
            <p className="text-gray-300 text-sm">
              The Resource Marketplace is in alpha. Lease contracts, pricing engine, and
              resource pools are being built. Current capacity metrics from ngauge are shown below.
            </p>
            <Badge className="mt-3 bg-orange-500/20 text-orange-400 border-orange-500/30">
              Alpha Preview
            </Badge>
          </div>
        </CardContent>
      </Card>

      {/* Current Capacity Stats */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Store className="h-5 w-5 text-orange-400" />
            Available Resources
          </CardTitle>
          <CardDescription className="text-gray-400">
            Real-time resource availability from ngauge capacity metrics
          </CardDescription>
        </CardHeader>
        <CardContent>
          {capacity.error ? (
            <div className="flex items-center gap-3 py-4">
              <AlertTriangle className="h-5 w-5 text-red-400" />
              <p className="text-gray-500">Capacity data unavailable</p>
            </div>
          ) : (
            <div className="grid gap-3 md:grid-cols-2">
              <ResourceRow
                label="CPU"
                value={capacity.data?.cpu_usage}
                format={(v) => `${((1 - v) * 100).toFixed(1)}% available`}
              />
              <ResourceRow
                label="Memory"
                value={capacity.data?.memory_usage}
                format={(v) => `${((1 - v) * 100).toFixed(1)}% available`}
              />
              <ResourceRow
                label="Storage"
                value={capacity.data?.storage_usage}
                format={(v) => `${((1 - v) * 100).toFixed(1)}% available`}
              />
              <ResourceRow
                label="Network"
                value={capacity.data?.network_usage}
                format={(v) => `${((1 - v) * 100).toFixed(1)}% available`}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Tier Pricing (Static) */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Tier Pricing</CardTitle>
          <CardDescription className="text-gray-400">
            Market tier multipliers for resource pricing
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 md:grid-cols-4">
            {[
              { tier: 'L0 (Hot)', multiplier: '1.0x' },
              { tier: 'L1 (Warm)', multiplier: '0.8x' },
              { tier: 'L2 (Cool)', multiplier: '0.5x' },
              { tier: 'L3 (Cold)', multiplier: '0.2x' },
            ].map((p) => (
              <div
                key={p.tier}
                className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center"
              >
                <Badge className="mb-2">{p.tier}</Badge>
                <div className="text-xl font-bold text-orange-400">{p.multiplier}</div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function ResourceRow({
  label,
  value,
  format,
}: {
  label: string;
  value: number | undefined;
  format: (v: number) => string;
}) {
  return (
    <div className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
      <span className="text-sm text-white font-medium">{label}</span>
      <span className="text-sm text-orange-400">
        {value != null ? format(value) : '--'}
      </span>
    </div>
  );
}

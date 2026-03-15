// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Store, FileText, DollarSign, AlertTriangle } from 'lucide-react';
import { useResourcePools, useLeases, usePricingInfo, useCreateLease } from '@/lib/api';
import { cn } from '@/lib/utils';

const leaseStateColors: Record<string, string> = {
  Proposed: 'text-blue-400 bg-blue-500/20',
  Active: 'text-green-400 bg-green-500/20',
  Completed: 'text-gray-400 bg-gray-500/20',
  Cancelled: 'text-red-400 bg-red-500/20',
};

export default function EngaugeMarketplace() {
  const pools = useResourcePools();
  const leases = useLeases();
  const pricing = usePricingInfo();
  const createLease = useCreateLease();

  const allErrored = pools.error && leases.error && pricing.error;

  if (allErrored) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Resource Marketplace</h2>
        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Engauge service offline</p>
            <p className="text-gray-500 text-sm mt-1">Unable to reach the marketplace backend.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Resource Marketplace</h2>

      {/* Pricing Engine */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <DollarSign className="h-5 w-5 text-orange-400" />
            Tier Pricing
          </CardTitle>
          <CardDescription className="text-gray-400">Market tier multipliers for resource pricing</CardDescription>
        </CardHeader>
        <CardContent>
          {pricing.isLoading ? <Skeleton className="h-24 w-full" /> : pricing.error ? (
            <p className="text-gray-500 text-center py-4">Pricing data unavailable</p>
          ) : (
            <div className="grid gap-3 md:grid-cols-4">
              {pricing.data?.map(p => (
                <div key={p.tier} className="p-3 rounded-lg bg-black/20 border border-gray-800 text-center">
                  <Badge className="mb-2">{p.tier}</Badge>
                  <div className="text-xl font-bold text-orange-400">{p.multiplier}x</div>
                  <div className="text-xs text-gray-400">{p.effective_price.toFixed(4)} GG/unit</div>
                </div>
              )) || <p className="text-gray-500">No pricing data</p>}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Resource Pools */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Store className="h-5 w-5 text-orange-400" />
            Resource Pools
          </CardTitle>
        </CardHeader>
        <CardContent>
          {pools.isLoading ? <Skeleton className="h-40 w-full" /> : pools.error ? (
            <p className="text-gray-500 text-center py-4">Resource pool data unavailable</p>
          ) : (
            <div className="space-y-2">
              {pools.data?.map(pool => (
                <div key={pool.pool_id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                  <div>
                    <div className="text-sm text-white font-medium">{pool.resource_type}</div>
                    <div className="text-xs text-gray-400">
                      {pool.available_units}/{pool.total_units} available - Sovereign: {pool.sovereign_allocation_pct}%
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-orange-400">{pool.price_per_unit.toFixed(4)} GG/unit</span>
                    <Button
                      size="sm"
                      className="bg-orange-500/20 text-orange-400 hover:bg-orange-500/30"
                      onClick={() => createLease.mutate({ pool_id: pool.pool_id, units: 1, tier: pool.tier })}
                      disabled={createLease.isPending}
                    >
                      Lease
                    </Button>
                  </div>
                </div>
              )) || <p className="text-gray-500 text-center py-4">No pools available</p>}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Active Leases */}
      <Card className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <FileText className="h-5 w-5 text-orange-400" />
            Lease Contracts
          </CardTitle>
        </CardHeader>
        <CardContent>
          {leases.isLoading ? <Skeleton className="h-32 w-full" /> : leases.error ? (
            <p className="text-gray-500 text-center py-4">Lease data unavailable</p>
          ) : (
            <div className="space-y-2">
              {leases.data?.map(lease => (
                <div key={lease.lease_id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                  <div className="flex items-center gap-3">
                    <Badge className={leaseStateColors[lease.state] || ''}>{lease.state}</Badge>
                    <div>
                      <div className="text-sm text-white font-mono">{lease.lease_id.slice(0, 12)}...</div>
                      <div className="text-xs text-gray-400">{lease.units} units - {lease.cost_gg.toFixed(4)} GG</div>
                    </div>
                  </div>
                  <div className="text-xs text-gray-400">
                    {new Date(lease.expires_at).toLocaleDateString()}
                  </div>
                </div>
              )) || <p className="text-gray-500 text-center py-4">No leases</p>}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

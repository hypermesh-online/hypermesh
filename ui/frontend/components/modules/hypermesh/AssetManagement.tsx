// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useAssetList } from '@/lib/hooks/useBlockMatrix';
import {
  Box,
  AlertTriangle,
  Database,
  Hash,
} from 'lucide-react';

export function AssetManagement() {
  const { data: assets, isLoading, error } = useAssetList();

  if (isLoading) return <ModuleLoading />;

  if (error) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{error.message}</p>
        </CardContent>
      </Card>
    );
  }

  // Group by category for summary
  const categoryMap = React.useMemo(() => {
    const map = new Map<string, number>();
    if (assets) {
      for (const a of assets) {
        map.set(a.category, (map.get(a.category) ?? 0) + 1);
      }
    }
    return map;
  }, [assets]);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Asset Management</h2>

      {/* Summary */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Assets</CardTitle>
            <Database className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{assets?.length ?? 0}</div>
            <p className="text-xs text-gray-400">Registered on-chain</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Categories</CardTitle>
            <Box className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{categoryMap.size}</div>
            <p className="text-xs text-gray-400">Asset types</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Categories</CardTitle>
            <Hash className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-1">
              {Array.from(categoryMap.entries()).map(([cat, count]) => (
                <Badge key={cat} variant="outline" className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30">
                  {cat}: {count}
                </Badge>
              ))}
              {categoryMap.size === 0 && (
                <span className="text-xs text-gray-500">None</span>
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Asset list */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Database className="h-5 w-5 text-cyan-400" />
            Asset Registry
          </CardTitle>
          <CardDescription className="text-gray-400">
            All blockchain-registered assets with content hashes
          </CardDescription>
        </CardHeader>
        <CardContent>
          {assets && assets.length > 0 ? (
            <div className="space-y-2 max-h-[500px] overflow-y-auto">
              {assets.map((asset) => (
                <div
                  key={asset.id}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <Box className="h-4 w-4 text-cyan-400 shrink-0" />
                    <div className="min-w-0">
                      <p className="text-sm text-white font-mono truncate">{asset.id}</p>
                      <p className="text-xs text-gray-400">
                        Block #{asset.block_index} | Hash:{' '}
                        <span className="font-mono">{asset.content_hash.slice(0, 16)}...</span>
                      </p>
                    </div>
                  </div>
                  <Badge
                    variant="outline"
                    className="text-xs bg-purple-500/20 text-purple-400 border-purple-500/30 shrink-0 ml-2"
                  >
                    {asset.category}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8">
              <Database className="h-12 w-12 text-gray-600 mx-auto mb-3" />
              <h3 className="text-lg font-medium text-white mb-2">No Assets</h3>
              <p className="text-gray-400">
                Assets are registered on the blockchain when files are stored or DNS names are
                created. Use the CLI to store content:{' '}
                <code className="text-cyan-400 bg-black/40 px-1 rounded">
                  hypermesh store &lt;file&gt;
                </code>
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

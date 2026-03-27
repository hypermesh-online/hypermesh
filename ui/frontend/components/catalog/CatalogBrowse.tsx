// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Package, Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { useAssetList } from '@/lib/hooks/useBlockMatrix';
import { ModuleLoading } from '@/components/ui/ModuleLoading';

export function CatalogBrowse() {
  const { data: assets, isLoading, error } = useAssetList();
  const [filter, setFilter] = React.useState('');

  if (isLoading) return <ModuleLoading />;

  const filteredAssets = (assets || []).filter((a: { content_hash?: string; category?: string }) =>
    !filter || (a.content_hash || '').includes(filter) || (a.category || '').toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Search className="h-4 w-4 text-gray-400" />
        <Input
          placeholder="Search assets by hash or category..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="max-w-sm"
        />
      </div>

      {error && (
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="p-4 text-red-400 text-sm">{error.message}</CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {filteredAssets.map((asset: { content_hash?: string; category?: string; block_index?: number }, i: number) => (
          <Card key={asset.content_hash || i} className="border-pink-500/20 bg-pink-500/5">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <Package className="h-4 w-4 text-pink-400" />
                <span className="truncate font-mono text-xs">{(asset.content_hash || '').slice(0, 16)}...</span>
              </CardTitle>
            </CardHeader>
            <CardContent>
              <Badge variant="outline" className="text-xs">{asset.category || 'unknown'}</Badge>
              {asset.block_index !== undefined && (
                <p className="text-xs text-gray-500 mt-2">Block #{asset.block_index}</p>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {filteredAssets.length === 0 && !isLoading && (
        <p className="text-center text-gray-500 py-8">No assets found</p>
      )}
    </div>
  );
}

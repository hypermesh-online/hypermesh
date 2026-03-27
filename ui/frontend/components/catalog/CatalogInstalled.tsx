// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle, Package } from 'lucide-react';
import { useAssetList } from '@/lib/hooks/useBlockMatrix';
import { ModuleLoading } from '@/components/ui/ModuleLoading';

export function CatalogInstalled() {
  const { data: assets, isLoading } = useAssetList();

  if (isLoading) return <ModuleLoading />;

  const assetList = assets || [];

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-pink-400">Installed Assets</h3>
        <Badge variant="outline">{assetList.length} registered</Badge>
      </div>

      {assetList.length === 0 ? (
        <Card className="border-gray-700">
          <CardContent className="p-8 text-center text-gray-500">
            <Package className="h-8 w-8 mx-auto mb-2 opacity-50" />
            <p>No assets registered on this node</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-2">
          {assetList.map((asset: { content_hash?: string; category?: string; block_index?: number }, i: number) => (
            <Card key={asset.content_hash || i} className="border-gray-700/50">
              <CardContent className="p-3 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <CheckCircle className="h-4 w-4 text-green-400" />
                  <span className="font-mono text-xs text-gray-300">{(asset.content_hash || '').slice(0, 24)}...</span>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">{asset.category || 'system'}</Badge>
                  {asset.block_index !== undefined && (
                    <span className="text-xs text-gray-500">#{asset.block_index}</span>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

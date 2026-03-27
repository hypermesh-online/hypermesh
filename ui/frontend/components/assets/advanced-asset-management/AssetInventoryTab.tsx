// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Database, Box } from 'lucide-react';
import type { AssetRecord } from '@/lib/blockmatrix-api';

interface AssetInventoryTabProps {
  assets: AssetRecord[];
  isLoading: boolean;
}

export function AssetInventoryTab({ assets, isLoading }: AssetInventoryTabProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Database className="h-5 w-5 text-purple-400" />
          Asset Inventory
        </CardTitle>
        <CardDescription className="text-gray-400">
          Blockchain-registered assets with content hashes and categories
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="animate-pulse h-16 bg-gray-700 rounded-lg" />
            ))}
          </div>
        ) : assets.length > 0 ? (
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {assets.map((asset) => (
              <div
                key={asset.id}
                className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg border border-purple-500/20"
              >
                <div className="flex items-center gap-3 flex-1 min-w-0">
                  <Box className="h-4 w-4 text-purple-400 shrink-0" />
                  <div className="min-w-0">
                    <p className="text-sm text-white font-mono truncate">{asset.id}</p>
                    <p className="text-xs text-gray-400">
                      Block #{asset.block_index} | Hash: {asset.content_hash.slice(0, 16)}...
                    </p>
                  </div>
                </div>
                <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400 border-purple-500/30 shrink-0 ml-2">
                  {asset.category}
                </Badge>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Database className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Assets Registered</h3>
            <p className="text-gray-400">
              Assets appear here after they are registered on the blockchain.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

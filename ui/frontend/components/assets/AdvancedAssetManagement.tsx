// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Advanced Asset Management Dashboard
 *
 * Shows blockchain-registered assets from the real IPC API.
 * Uses useBlockMatrix hooks exclusively -- no legacy API imports.
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { useAssetList, useNodeStatus } from '@/lib/hooks/useBlockMatrix';
import { Database, AlertTriangle, Globe, Box } from 'lucide-react';
import {
  AssetInventoryTab,
  ProxyAddressingTab,
  AnalyticsTab,
} from './advanced-asset-management';

export function AdvancedAssetManagement() {
  const { data: assets, isLoading: assetsLoading, error: assetsError } = useAssetList();
  const { data: nodeStatus } = useNodeStatus();

  if (assetsLoading) return <ModuleLoading />;

  if (assetsError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{assetsError.message}</p>
        </CardContent>
      </Card>
    );
  }

  const assetCount = assets?.length ?? 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center py-4">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-purple-400 to-pink-600 bg-clip-text text-transparent mb-2">
          Advanced Asset Management
        </h1>
        <p className="text-gray-400 max-w-3xl mx-auto">
          Blockchain-registered asset management with NAT-like proxy addressing
          and real-time analytics.
        </p>
      </div>

      {/* Overview cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Assets</CardTitle>
            <Database className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{assetCount}</div>
            <p className="text-xs text-gray-400">Registered on-chain</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Node Status</CardTitle>
            <Box className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">
              {nodeStatus ? 'Online' : 'Offline'}
            </div>
            <p className="text-xs text-gray-400">
              {nodeStatus ? `${nodeStatus.peers} peers` : 'Not connected'}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Chain Height</CardTitle>
            <Globe className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">
              {nodeStatus?.chain_height ?? '--'}
            </div>
            <p className="text-xs text-gray-400">Local blockchain</p>
          </CardContent>
        </Card>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="assets" className="space-y-6">
        <TabsList className="grid w-full grid-cols-3 bg-black/40">
          <TabsTrigger value="assets" className="data-[state=active]:bg-purple-500/20">
            Asset Inventory
          </TabsTrigger>
          <TabsTrigger value="proxies" className="data-[state=active]:bg-purple-500/20">
            Proxy Addressing
          </TabsTrigger>
          <TabsTrigger value="analytics" className="data-[state=active]:bg-purple-500/20">
            Analytics
          </TabsTrigger>
        </TabsList>

        <TabsContent value="assets" className="space-y-6">
          <AssetInventoryTab assets={assets ?? []} isLoading={assetsLoading} />
        </TabsContent>

        <TabsContent value="proxies" className="space-y-6">
          <ProxyAddressingTab />
        </TabsContent>

        <TabsContent value="analytics" className="space-y-6">
          <AnalyticsTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}

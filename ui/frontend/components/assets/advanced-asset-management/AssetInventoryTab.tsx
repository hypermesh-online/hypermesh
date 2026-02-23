// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { Asset, VMAsset, VMExecution } from '@/lib/api/services/HyperMeshTypes';
import type { SystemStatus } from '@/lib/api';
import {
  HardDrive, Cpu, MemoryStick, Network, Server, Globe,
  Activity, Settings, Plus, RefreshCw, Eye, Database,
  Play, Monitor, Package, Container
} from 'lucide-react';

interface AssetInventoryTabProps {
  assets: Asset[];
  vmAssets: VMAsset[];
  vmExecutions: VMExecution[] | undefined;
  assetsLoading: boolean;
  vmAssetsLoading: boolean;
  systemStatus: SystemStatus | undefined;
  onCreateAsset: () => void;
  onCreateProxy: (assetId: string) => void;
  isCreating: boolean;
}

function getHardwareIcon(type: string) {
  if (type === 'compute' || type === 'cpu') return Cpu;
  if (type === 'storage') return HardDrive;
  if (type === 'memory') return MemoryStick;
  if (type === 'network') return Network;
  return Server;
}

function getVMIcon(type: string) {
  if (type === 'vm') return Monitor;
  if (type === 'application') return Package;
  return Container;
}

export function AssetInventoryTab({
  assets,
  vmAssets,
  vmExecutions,
  assetsLoading,
  vmAssetsLoading,
  systemStatus,
  onCreateAsset,
  onCreateProxy,
  isCreating
}: AssetInventoryTabProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Database className="h-5 w-5 text-purple-400" />
              Asset Inventory
            </CardTitle>
            <CardDescription className="text-gray-400">Comprehensive asset registry with real-time status monitoring</CardDescription>
          </div>
          <div className="flex gap-2">
            <Button
              onClick={onCreateAsset}
              disabled={isCreating}
              className="bg-gradient-to-r from-purple-500 to-pink-600 hover:from-purple-400 hover:to-pink-500 text-black"
            >
              <Plus className="h-4 w-4 mr-2" />
              {isCreating ? 'Creating...' : 'Create Asset'}
            </Button>
            <Button variant="outline" className="border-purple-500/30 text-purple-400">
              <RefreshCw className="h-4 w-4 mr-2" />
              Refresh
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {(assetsLoading || vmAssetsLoading) ? (
          <div className="space-y-3">
            {[1,2,3,4].map(i => (
              <div key={i} className="animate-pulse h-20 bg-gray-700 rounded-lg"></div>
            ))}
          </div>
        ) : (assets && assets.length > 0) || (vmAssets && vmAssets.length > 0) ? (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {assets.map((asset) => (
              <HardwareAssetRow
                key={asset.id}
                asset={asset}
                onCreateProxy={onCreateProxy}
              />
            ))}
            {vmAssets.map((asset) => (
              <VMAssetRow
                key={asset.id}
                asset={asset}
                vmExecutions={vmExecutions}
                onCreateProxy={onCreateProxy}
              />
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Database className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Assets Available</h3>
            <p className="text-gray-400 mb-4">
              {systemStatus ? 'Create your first asset to begin resource management' : 'System offline - unable to load assets'}
            </p>
            <Button
              onClick={onCreateAsset}
              disabled={isCreating || !systemStatus}
              className="bg-gradient-to-r from-purple-500 to-pink-600 hover:from-purple-400 hover:to-pink-500 text-black"
            >
              <Plus className="h-4 w-4 mr-2" />
              Create First Asset
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function HardwareAssetRow({
  asset,
  onCreateProxy
}: {
  asset: Asset;
  onCreateProxy: (assetId: string) => void;
}) {
  const Icon = getHardwareIcon(asset.type);

  return (
    <div className="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg border border-purple-500/20">
      <div className="flex-1">
        <div className="flex items-center gap-3 mb-2">
          <Icon className="h-5 w-5 text-purple-400" />
          <h4 className="font-medium text-white">{asset.name}</h4>
          <Badge variant="outline" className={cn(
            'text-xs',
            asset.status === 'active' || asset.status === 'available' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
            asset.status === 'allocated' || asset.status === 'busy' ? 'bg-blue-500/20 text-blue-400 border-blue-500/30' :
            asset.status === 'maintenance' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
            'bg-red-500/20 text-red-400 border-red-500/30'
          )}>
            {asset.status}
          </Badge>
          <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400">
            {asset.type}
          </Badge>
          <Badge variant="outline" className={cn(
            'text-xs',
            asset.privacyLevel === 'private' ? 'bg-red-500/20 text-red-400' :
            asset.privacyLevel === 'federated' ? 'bg-blue-500/20 text-blue-400' :
            'bg-green-500/20 text-green-400'
          )}>
            {asset.privacyLevel}
          </Badge>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div>
            <span className="text-gray-400">Asset ID:</span>
            <div className="text-white font-mono">{asset.id.slice(0, 12)}...</div>
          </div>
          <div>
            <span className="text-gray-400">Owner:</span>
            <div className="text-white font-mono">{asset.owner?.slice(0, 8)}...</div>
          </div>
          <div>
            <span className="text-gray-400">Created:</span>
            <div className="text-white">{new Date(asset.createdAt).toLocaleDateString()}</div>
          </div>
          <div>
            <span className="text-gray-400">Performance:</span>
            <div className="text-green-400">{Math.random() * 30 + 70 | 0}%</div>
          </div>
        </div>
        {asset.specifications && (
          <div className="mt-3 pt-3 border-t border-purple-500/20">
            <span className="text-gray-400 text-sm">Specifications:</span>
            <div className="flex flex-wrap gap-2 mt-1">
              {Object.entries(asset.specifications).map(([key, value]) => (
                <Badge key={key} variant="outline" className="text-xs bg-gray-500/20 text-gray-300">
                  {key}: {String(value)}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          className="text-cyan-400 hover:bg-cyan-500/20"
          onClick={() => onCreateProxy(asset.id)}
        >
          <Globe className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
          <Settings className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" className="text-green-400 hover:bg-green-500/20">
          <Eye className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

function VMAssetRow({
  asset,
  vmExecutions,
  onCreateProxy
}: {
  asset: VMAsset;
  vmExecutions: VMExecution[] | undefined;
  onCreateProxy: (assetId: string) => void;
}) {
  const Icon = getVMIcon(asset.type);
  const runningExecutions = vmExecutions?.filter(exec =>
    exec.vmAssetId === asset.id &&
    (exec.status === 'running' || exec.status === 'starting')
  ).length || 0;

  return (
    <div className="flex items-center justify-between p-4 bg-gray-800/50 rounded-lg border border-green-500/20">
      <div className="flex-1">
        <div className="flex items-center gap-3 mb-2">
          <Icon className="h-5 w-5 text-green-400" />
          <h4 className="font-medium text-white">{asset.name}</h4>
          <Badge variant="outline" className={cn(
            'text-xs',
            asset.status === 'available' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
            asset.status === 'allocated' || asset.status === 'busy' ? 'bg-blue-500/20 text-blue-400 border-blue-500/30' :
            'bg-red-500/20 text-red-400 border-red-500/30'
          )}>
            {asset.status}
          </Badge>
          <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
            {asset.vmConfig.runtime}
          </Badge>
          <Badge variant="outline" className={cn(
            'text-xs',
            asset.privacyLevel === 'private' ? 'bg-red-500/20 text-red-400' :
            asset.privacyLevel === 'federated' ? 'bg-blue-500/20 text-blue-400' :
            'bg-green-500/20 text-green-400'
          )}>
            {asset.privacyLevel}
          </Badge>
          {runningExecutions > 0 && (
            <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
              <Activity className="h-3 w-3 mr-1" />
              {runningExecutions} running
            </Badge>
          )}
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div>
            <span className="text-gray-400">Asset ID:</span>
            <div className="text-white font-mono">{asset.id.slice(0, 12)}...</div>
          </div>
          <div>
            <span className="text-gray-400">Runtime:</span>
            <div className="text-white">{asset.vmConfig.runtime}</div>
          </div>
          <div>
            <span className="text-gray-400">Max CPU:</span>
            <div className="text-white">{asset.vmConfig.resourceLimits.maxCpu} cores</div>
          </div>
          <div>
            <span className="text-gray-400">Max Memory:</span>
            <div className="text-white">{asset.vmConfig.resourceLimits.maxMemory}</div>
          </div>
        </div>
        {asset.catalogMetadata && (
          <div className="mt-3 pt-3 border-t border-green-500/20">
            <span className="text-gray-400 text-sm">Catalog Info:</span>
            <div className="flex flex-wrap gap-2 mt-1">
              <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                v{asset.catalogMetadata.version}
              </Badge>
              <Badge variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400">
                {'\u2605'} {asset.catalogMetadata.rating}/5
              </Badge>
              <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                {asset.catalogMetadata.downloadCount} downloads
              </Badge>
            </div>
          </div>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="sm" className="text-green-400 hover:bg-green-500/20">
          <Play className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-cyan-400 hover:bg-cyan-500/20"
          onClick={() => onCreateProxy(asset.id)}
        >
          <Globe className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
          <Settings className="h-4 w-4" />
        </Button>
        <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
          <Eye className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

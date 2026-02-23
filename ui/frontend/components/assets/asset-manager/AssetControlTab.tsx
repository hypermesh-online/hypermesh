// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import type { Asset, VMAsset, VMExecution } from '@/lib/api/services/HyperMeshTypes';
import type { AssetControlMetrics } from './types';
import { getAssetIcon, getPrivacyIcon } from './utils';
import {
  Database,
  Plus,
  Settings,
  Play,
  Pause,
  Square,
  Eye,
  Edit,
  Upload,
  RefreshCw,
  Shield,
  Target,
  Activity
} from 'lucide-react';

interface AssetControlTabProps {
  assets: Asset[] | undefined;
  vmAssets: VMAsset[] | undefined;
  vmExecutions: VMExecution[] | undefined;
  assetsLoading: boolean;
  vmAssetsLoading: boolean;
  selectedAsset: string | null;
  setSelectedAsset: (id: string | null) => void;
  assetMetrics: AssetControlMetrics;
  systemStatus: any;
  onExecuteVM: (vmAssetId: string) => void;
}

export function AssetControlTab({
  assets,
  vmAssets,
  vmExecutions,
  assetsLoading,
  vmAssetsLoading,
  selectedAsset,
  setSelectedAsset,
  assetMetrics,
  systemStatus,
  onExecuteVM
}: AssetControlTabProps) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Target className="h-5 w-5 text-green-400" />
              Real-time Asset Control Panel
            </CardTitle>
            <CardDescription className="text-gray-400">
              Monitor and control all assets with real-time status and performance metrics
            </CardDescription>
          </div>
          <Button variant="outline" className="border-green-500/30 text-green-400">
            <RefreshCw className="h-4 w-4 mr-2" />
            Refresh
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {(assetsLoading || vmAssetsLoading) ? (
          <div className="space-y-3">
            {[1,2,3,4].map(i => (
              <div key={i} className="animate-pulse h-24 bg-gray-700 rounded-lg"></div>
            ))}
          </div>
        ) : (assets && assets.length > 0) || (vmAssets && vmAssets.length > 0) ? (
          <div className="space-y-4">
            {/* Hardware Assets */}
            {assets?.map((asset) => (
              <HardwareAssetRow
                key={asset.id}
                asset={asset}
                isSelected={selectedAsset === asset.id}
                onSelect={() => setSelectedAsset(selectedAsset === asset.id ? null : asset.id)}
                assetMetrics={assetMetrics}
              />
            ))}

            {/* VM Assets */}
            {vmAssets?.map((asset) => (
              <VMAssetRow
                key={asset.id}
                asset={asset}
                isSelected={selectedAsset === asset.id}
                onSelect={() => setSelectedAsset(selectedAsset === asset.id ? null : asset.id)}
                vmExecutions={vmExecutions}
                onExecuteVM={onExecuteVM}
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
              disabled={!systemStatus}
              className="bg-gradient-to-r from-green-500 to-blue-600 hover:from-green-400 hover:to-blue-500 text-black"
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
  isSelected,
  onSelect,
  assetMetrics
}: {
  asset: Asset;
  isSelected: boolean;
  onSelect: () => void;
  assetMetrics: AssetControlMetrics;
}) {
  const AssetIcon = getAssetIcon(asset.type);
  const PrivacyIcon = getPrivacyIcon(asset.privacyLevel || 'federated');

  return (
    <div
      className={cn(
        'p-4 rounded-lg border transition-all cursor-pointer',
        isSelected ? 'bg-green-500/10 border-green-500/40 ring-2 ring-green-500/30' :
        'bg-gray-800/50 border-gray-600/30 hover:border-green-500/30'
      )}
      onClick={onSelect}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <AssetIcon className="h-6 w-6 text-green-400" />
          <div>
            <h4 className="text-white font-medium">{asset.name}</h4>
            <p className="text-sm text-gray-400">Asset ID: {asset.id.slice(0, 12)}...</p>
          </div>
          <div className="flex gap-2">
            <Badge variant="outline" className={cn(
              'text-xs',
              asset.status === 'active' || asset.status === 'available' ? 'bg-green-500/20 text-green-400' :
              asset.status === 'allocated' ? 'bg-blue-500/20 text-blue-400' :
              'bg-red-500/20 text-red-400'
            )}>
              {asset.status}
            </Badge>
            <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400">
              {asset.type}
            </Badge>
            <Badge variant="outline" className={cn(
              'text-xs flex items-center gap-1',
              asset.privacyLevel === 'private' ? 'bg-red-500/20 text-red-400' :
              asset.privacyLevel === 'federated' ? 'bg-blue-500/20 text-blue-400' :
              'bg-green-500/20 text-green-400'
            )}>
              <PrivacyIcon className="h-3 w-3" />
              {asset.privacyLevel}
            </Badge>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" className="text-green-400 hover:bg-green-500/20">
            <Play className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
            <Settings className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
            <Eye className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {isSelected && (
        <div className="pt-3 border-t border-green-500/20 space-y-4">
          <div className="grid gap-4 md:grid-cols-4">
            {[
              { label: 'CPU Usage', value: assetMetrics.cpuUsage },
              { label: 'Memory', value: assetMetrics.memoryUsage },
              { label: 'Storage', value: assetMetrics.storageUsage },
              { label: 'Network', value: assetMetrics.networkUsage }
            ].map((metric) => (
              <div key={metric.label} className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-gray-400 text-sm">{metric.label}</span>
                  <span className="text-white font-mono text-sm">{metric.value.toFixed(1)}%</span>
                </div>
                <Progress value={metric.value} className="h-1" />
              </div>
            ))}
          </div>

          <div className="flex items-center justify-between pt-2">
            <div className="flex gap-2">
              <Button variant="outline" size="sm" className="border-green-500/30 text-green-400">
                <Edit className="h-4 w-4 mr-1" />Edit
              </Button>
              <Button variant="outline" size="sm" className="border-blue-500/30 text-blue-400">
                <Upload className="h-4 w-4 mr-1" />Share
              </Button>
              <Button variant="outline" size="sm" className="border-purple-500/30 text-purple-400">
                <Shield className="h-4 w-4 mr-1" />Secure
              </Button>
            </div>
            <div className="text-sm text-gray-400">
              Performance Score: <span className="text-green-400 font-medium">{assetMetrics.performanceScore.toFixed(0)}%</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function VMAssetRow({
  asset,
  isSelected,
  onSelect,
  vmExecutions,
  onExecuteVM
}: {
  asset: VMAsset;
  isSelected: boolean;
  onSelect: () => void;
  vmExecutions: VMExecution[] | undefined;
  onExecuteVM: (vmAssetId: string) => void;
}) {
  const AssetIcon = getAssetIcon(asset.type);
  const PrivacyIcon = getPrivacyIcon(asset.privacyLevel || 'federated');
  const runningExecutions = vmExecutions?.filter(e =>
    e.vmAssetId === asset.id && e.status === 'running'
  ).length || 0;

  return (
    <div
      className={cn(
        'p-4 rounded-lg border transition-all cursor-pointer',
        isSelected ? 'bg-blue-500/10 border-blue-500/40 ring-2 ring-blue-500/30' :
        'bg-gray-800/50 border-gray-600/30 hover:border-blue-500/30'
      )}
      onClick={onSelect}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <AssetIcon className="h-6 w-6 text-blue-400" />
          <div>
            <h4 className="text-white font-medium">{asset.name}</h4>
            <p className="text-sm text-gray-400">VM Asset: {asset.id.slice(0, 12)}...</p>
          </div>
          <div className="flex gap-2">
            <Badge variant="outline" className={cn(
              'text-xs',
              asset.status === 'available' ? 'bg-green-500/20 text-green-400' :
              asset.status === 'allocated' ? 'bg-blue-500/20 text-blue-400' :
              'bg-red-500/20 text-red-400'
            )}>
              {asset.status}
            </Badge>
            <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
              {asset.vmConfig.runtime}
            </Badge>
            <Badge variant="outline" className={cn(
              'text-xs flex items-center gap-1',
              asset.privacyLevel === 'private' ? 'bg-red-500/20 text-red-400' :
              asset.privacyLevel === 'federated' ? 'bg-blue-500/20 text-blue-400' :
              'bg-green-500/20 text-green-400'
            )}>
              <PrivacyIcon className="h-3 w-3" />
              {asset.privacyLevel}
            </Badge>
            {runningExecutions > 0 && (
              <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                <Activity className="h-3 w-3 mr-1" />
                {runningExecutions} running
              </Badge>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={(e) => { e.stopPropagation(); onExecuteVM(asset.id); }}
            className="text-green-400 hover:bg-green-500/20"
          >
            <Play className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
            <Settings className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
            <Eye className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {isSelected && (
        <div className="pt-3 border-t border-blue-500/20 space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            <div>
              <span className="text-gray-400 text-sm">Runtime:</span>
              <div className="text-white font-medium">{asset.vmConfig.runtime}</div>
            </div>
            <div>
              <span className="text-gray-400 text-sm">Max CPU:</span>
              <div className="text-white font-medium">{asset.vmConfig.resourceLimits.maxCpu} cores</div>
            </div>
            <div>
              <span className="text-gray-400 text-sm">Max Memory:</span>
              <div className="text-white font-medium">{asset.vmConfig.resourceLimits.maxMemory}</div>
            </div>
          </div>

          <div className="flex items-center justify-between pt-2">
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => onExecuteVM(asset.id)}
                className="border-green-500/30 text-green-400"
              >
                <Play className="h-4 w-4 mr-1" />Execute
              </Button>
              <Button variant="outline" size="sm" className="border-yellow-500/30 text-yellow-400">
                <Pause className="h-4 w-4 mr-1" />Pause
              </Button>
              <Button variant="outline" size="sm" className="border-red-500/30 text-red-400">
                <Square className="h-4 w-4 mr-1" />Stop
              </Button>
            </div>
            <div className="text-sm text-gray-400">
              Active Executions: <span className="text-blue-400 font-medium">{runningExecutions}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

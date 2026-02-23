// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { MapPin, Plus, Zap, RefreshCw, Settings, Server, Network, Globe, Router } from 'lucide-react';
import type { ProxyMapping } from './types';

interface MappingsTabProps {
  proxyMappings: ProxyMapping[];
  testingProxy: string | null;
  systemStatus: unknown;
  onCreateMapping: () => void;
  onTestConnection: (mappingId: string) => void;
  isCreating: boolean;
}

export function MappingsTab({
  proxyMappings,
  testingProxy,
  systemStatus,
  onCreateMapping,
  onTestConnection,
  isCreating
}: MappingsTabProps) {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <MapPin className="h-5 w-5 text-cyan-400" />
              NAT-like Address Mappings
            </CardTitle>
            <CardDescription className="text-gray-400">
              Configure virtual addresses for your resources with NAT-like translation
            </CardDescription>
          </div>
          <div className="flex gap-2">
            <Button
              onClick={onCreateMapping}
              disabled={isCreating}
              className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
            >
              <Plus className="h-4 w-4 mr-2" />
              {isCreating ? 'Creating...' : 'Create Mapping'}
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {proxyMappings.length > 0 ? (
          <div className="space-y-4">
            {proxyMappings.map((mapping) => (
              <MappingRow
                key={mapping.id}
                mapping={mapping}
                testingProxy={testingProxy}
                onTestConnection={onTestConnection}
              />
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <MapPin className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Address Mappings</h3>
            <p className="text-gray-400 mb-4">
              Create your first NAT-like address mapping to enable remote resource access.
            </p>
            <Button
              onClick={onCreateMapping}
              disabled={isCreating || !systemStatus}
              className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
            >
              <Plus className="h-4 w-4 mr-2" />
              Create First Mapping
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function MappingRow({
  mapping,
  testingProxy,
  onTestConnection
}: {
  mapping: ProxyMapping;
  testingProxy: string | null;
  onTestConnection: (id: string) => void;
}) {
  return (
    <div className={cn(
      'p-4 rounded-lg border transition-all',
      mapping.status === 'active' ? 'bg-cyan-500/5 border-cyan-500/30' :
      mapping.status === 'connecting' ? 'bg-yellow-500/5 border-yellow-500/30' :
      mapping.status === 'error' ? 'bg-red-500/5 border-red-500/30' :
      'bg-gray-500/5 border-gray-600/30'
    )}>
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <Router className="h-5 w-5 text-cyan-400" />
          <div>
            <h4 className="text-white font-medium">Asset {mapping.assetId.slice(0, 8)}...</h4>
            <p className="text-sm text-gray-400">NAT-like proxy mapping</p>
          </div>
          <Badge variant="outline" className={cn(
            'text-xs',
            mapping.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
            mapping.status === 'connecting' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
            mapping.status === 'error' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
            'bg-gray-500/20 text-gray-400 border-gray-500/30'
          )}>
            {mapping.status}
          </Badge>
          <Badge variant="outline" className={cn(
            'text-xs',
            mapping.trustLevel === 'high' ? 'bg-green-500/20 text-green-400' :
            mapping.trustLevel === 'medium' ? 'bg-yellow-500/20 text-yellow-400' :
            'bg-red-500/20 text-red-400'
          )}>
            {mapping.trustLevel} trust
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onTestConnection(mapping.id)}
            disabled={testingProxy === mapping.id}
            className="text-cyan-400 hover:bg-cyan-500/20"
          >
            {testingProxy === mapping.id ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : (
              <Zap className="h-4 w-4" />
            )}
          </Button>
          <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
            <Settings className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <div className="grid gap-4 text-sm">
        <div className="grid md:grid-cols-3 gap-4">
          <div className="bg-gray-800/50 p-3 rounded border">
            <div className="flex items-center gap-2 mb-2">
              <Server className="h-4 w-4 text-gray-400" />
              <span className="text-gray-400 font-medium">Local Address</span>
            </div>
            <div className="text-white font-mono text-xs">{mapping.localAddress}</div>
          </div>
          <div className="bg-blue-800/50 p-3 rounded border border-blue-500/30">
            <div className="flex items-center gap-2 mb-2">
              <Network className="h-4 w-4 text-blue-400" />
              <span className="text-blue-400 font-medium">Proxy Address</span>
            </div>
            <div className="text-blue-400 font-mono text-xs">{mapping.proxyAddress}</div>
          </div>
          <div className="bg-cyan-800/50 p-3 rounded border border-cyan-500/30">
            <div className="flex items-center gap-2 mb-2">
              <Globe className="h-4 w-4 text-cyan-400" />
              <span className="text-cyan-400 font-medium">Virtual Address</span>
            </div>
            <div className="text-cyan-400 font-mono text-xs">{mapping.virtualAddress}</div>
          </div>
        </div>

        <div className="grid md:grid-cols-4 gap-4 pt-3 border-t border-gray-600/30">
          <div>
            <span className="text-gray-400">Bandwidth:</span>
            <div className="text-white font-medium">{mapping.bandwidth.toFixed(0)} Mbps</div>
          </div>
          <div>
            <span className="text-gray-400">Latency:</span>
            <div className="text-white font-medium">{mapping.latency.toFixed(1)} ms</div>
          </div>
          <div>
            <span className="text-gray-400">Connections:</span>
            <div className="text-white font-medium">{mapping.connectionCount}</div>
          </div>
          <div>
            <span className="text-gray-400">Last Seen:</span>
            <div className="text-white font-medium">
              {new Date(mapping.lastSeen).toLocaleTimeString()}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

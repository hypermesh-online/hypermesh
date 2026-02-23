// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Network, Globe, Shield } from 'lucide-react';
import type { ProxyNode } from './types';

interface NodesTabProps {
  availableProxyNodes: ProxyNode[];
  onValidateTrust: (nodeId: string) => void;
  isValidating: boolean;
}

export function NodesTab({ availableProxyNodes, onValidateTrust, isValidating }: NodesTabProps) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Network className="h-5 w-5 text-green-400" />
          Trust-based Proxy Selection
        </CardTitle>
        <CardDescription className="text-gray-400">
          Select trusted proxy nodes for NAT-like address translation and routing
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {availableProxyNodes.map((node) => (
            <ProxyNodeCard
              key={node.id}
              node={node}
              onValidateTrust={onValidateTrust}
              isValidating={isValidating}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function ProxyNodeCard({
  node,
  onValidateTrust,
  isValidating
}: {
  node: ProxyNode;
  onValidateTrust: (nodeId: string) => void;
  isValidating: boolean;
}) {
  return (
    <Card className="bg-gray-800/50 border-gray-600/30">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Globe className="h-4 w-4 text-green-400" />
            <span className="text-white font-medium">{node.location}</span>
          </div>
          <Badge variant="outline" className={cn(
            'text-xs',
            node.securityLevel === 'quantum' ? 'bg-purple-500/20 text-purple-400' :
            node.securityLevel === 'enhanced' ? 'bg-blue-500/20 text-blue-400' :
            node.securityLevel === 'standard' ? 'bg-green-500/20 text-green-400' :
            'bg-gray-500/20 text-gray-400'
          )}>
            {node.securityLevel}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="text-xs text-gray-400 font-mono">{node.address}</div>

        <div className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <span className="text-gray-400">Validation:</span>
            <Badge variant="outline" className={cn('text-xs',
              node.validationStatus === 'verified' ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
            )}>
              {node.validationStatus === 'verified' ? 'Verified' : 'Rejected'}
            </Badge>
          </div>
          <div>
            <span className="text-gray-400">Uptime:</span>
            <div className="text-green-400 font-medium">{node.uptime.toFixed(1)}%</div>
          </div>
          <div>
            <span className="text-gray-400">Bandwidth:</span>
            <div className="text-white font-medium">{node.bandwidth.toFixed(0)} Mbps</div>
          </div>
          <div>
            <span className="text-gray-400">Latency:</span>
            <div className="text-white font-medium">{node.latency.toFixed(1)} ms</div>
          </div>
        </div>

        <div className="space-y-2">
          <span className="text-gray-400 text-sm">Capabilities:</span>
          <div className="flex flex-wrap gap-1">
            {node.capabilities.slice(0, 3).map((capability) => (
              <Badge key={capability} variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                {capability}
              </Badge>
            ))}
            {node.capabilities.length > 3 && (
              <Badge variant="outline" className="text-xs bg-gray-500/20 text-gray-400">
                +{node.capabilities.length - 3}
              </Badge>
            )}
          </div>
        </div>

        <div className="flex gap-2 pt-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onValidateTrust(node.id)}
            disabled={isValidating}
            className="flex-1 text-xs border-green-500/30 text-green-400"
          >
            <Shield className="h-3 w-3 mr-1" />
            Validate
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex-1 text-xs border-cyan-500/30 text-cyan-400"
          >
            Select
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

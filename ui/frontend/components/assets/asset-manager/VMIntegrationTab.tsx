// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { CatalogApplication, VMExecution } from '@/lib/api/services/HyperMeshTypes';
import {
  Container,
  Package,
  Monitor,
  Pause,
  Square
} from 'lucide-react';

interface VMIntegrationTabProps {
  catalogApps: CatalogApplication[] | undefined;
  vmExecutions: VMExecution[] | undefined;
  onInstallApp: (appId: string) => void;
  isInstalling: boolean;
}

export function VMIntegrationTab({
  catalogApps,
  vmExecutions,
  onInstallApp,
  isInstalling
}: VMIntegrationTabProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Container className="h-5 w-5 text-purple-400" />
          VM Asset Integration
        </CardTitle>
        <CardDescription className="text-gray-400">
          Install Catalog applications and manage VM asset executions
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-6 lg:grid-cols-2">
          {/* Catalog Applications */}
          <div className="space-y-4">
            <h4 className="text-white font-medium flex items-center gap-2">
              <Package className="h-4 w-4 text-purple-400" />
              Available Catalog Applications
            </h4>
            <div className="space-y-3 max-h-96 overflow-y-auto">
              {catalogApps && catalogApps.length > 0 ? (
                catalogApps.slice(0, 10).map((app) => (
                  <div key={app.id} className="p-3 bg-gray-800/50 rounded-lg border border-gray-600/30">
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <Package className="h-4 w-4 text-purple-400" />
                        <span className="text-white font-medium text-sm">{app.name}</span>
                        <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400">
                          v{app.version}
                        </Badge>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => onInstallApp(app.id)}
                        disabled={isInstalling}
                        className="text-xs border-purple-500/30 text-purple-400"
                      >
                        {isInstalling ? 'Installing...' : 'Install'}
                      </Button>
                    </div>
                    <p className="text-xs text-gray-400 mb-2">{app.description}</p>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400">
                        \u2605 {app.rating}/5
                      </Badge>
                      <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                        {app.downloadCount} downloads
                      </Badge>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center py-6 text-gray-400">
                  <Package className="h-8 w-8 mx-auto mb-2 text-gray-600" />
                  <p>No catalog applications available</p>
                </div>
              )}
            </div>
          </div>

          {/* VM Executions */}
          <div className="space-y-4">
            <h4 className="text-white font-medium flex items-center gap-2">
              <Monitor className="h-4 w-4 text-blue-400" />
              Active VM Executions
            </h4>
            <div className="space-y-3 max-h-96 overflow-y-auto">
              {vmExecutions && vmExecutions.length > 0 ? (
                vmExecutions.filter(exec => exec.status === 'running' || exec.status === 'starting').map((execution) => (
                  <div key={execution.id} className="p-3 bg-blue-500/5 border border-blue-500/20 rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <Monitor className="h-4 w-4 text-blue-400" />
                        <span className="text-white font-medium text-sm">
                          Execution {execution.id.slice(0, 8)}...
                        </span>
                        <Badge variant="outline" className={cn(
                          'text-xs',
                          execution.status === 'running' ? 'bg-green-500/20 text-green-400' :
                          execution.status === 'starting' ? 'bg-yellow-500/20 text-yellow-400' :
                          'bg-red-500/20 text-red-400'
                        )}>
                          {execution.status}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="sm" className="text-yellow-400 hover:bg-yellow-500/20">
                          <Pause className="h-3 w-3" />
                        </Button>
                        <Button variant="ghost" size="sm" className="text-red-400 hover:bg-red-500/20">
                          <Square className="h-3 w-3" />
                        </Button>
                      </div>
                    </div>
                    <div className="text-xs text-gray-400">
                      VM Asset: {execution.vmAssetId.slice(0, 8)}... -
                      Started: {execution.startTime ? new Date(execution.startTime).toLocaleTimeString() : 'Pending'}
                    </div>
                    {execution.result && (
                      <div className="mt-2 p-2 bg-gray-700/50 rounded text-xs">
                        <span className="text-gray-400">Output:</span>
                        <div className="text-green-400 font-mono">{execution.result.output?.slice(0, 100)}...</div>
                      </div>
                    )}
                  </div>
                ))
              ) : (
                <div className="text-center py-6 text-gray-400">
                  <Monitor className="h-8 w-8 mx-auto mb-2 text-gray-600" />
                  <p>No VM executions running</p>
                </div>
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// @ts-nocheck — Phase 8 will rewrite with useBlockMatrix hooks
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  Package, 
  Play,
  Trash2,
  Settings,
  Activity,
  HardDrive,
  CheckCircle
} from 'lucide-react';
import { useCatalogApplications, useVMAssets, useVMExecutions } from '@/lib/api';

export function CatalogInstalled() {
  const { installedApps, isLoading } = useCatalogApplications();
  const { vmAssets } = useVMAssets();
  const { data: executions } = useVMExecutions();

  if (isLoading) {
    return (
      <div className="text-center py-12">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-400 mx-auto"></div>
        <p className="text-gray-400 mt-4">Loading installed applications...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-green-400 to-blue-600 bg-clip-text text-transparent mb-2">
          Installed Applications
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Manage your installed HyperMesh applications and VM assets.
        </p>
      </div>

      <Tabs defaultValue="applications" className="space-y-6">
        <TabsList className="bg-black/40 border border-purple-500/30">
          <TabsTrigger value="applications">Applications ({installedApps.length})</TabsTrigger>
          <TabsTrigger value="vm-assets">VM Assets ({vmAssets?.length || 0})</TabsTrigger>
          <TabsTrigger value="executions">Executions ({executions?.length || 0})</TabsTrigger>
        </TabsList>

        <TabsContent value="applications" className="space-y-4">
          {installedApps.length > 0 ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {installedApps.map((app) => (
                <Card key={app.id} className="bg-black/40 border-green-500/30 backdrop-blur-lg">
                  <CardHeader className="pb-2">
                    <CardTitle className="text-lg text-white flex items-center gap-2">
                      <Package className="h-4 w-4" />
                      {app.name}
                    </CardTitle>
                    <CardDescription className="text-gray-400 text-sm">
                      {app.description}
                    </CardDescription>
                  </CardHeader>
                  
                  <CardContent className="space-y-4">
                    <div className="flex items-center gap-2">
                      <CheckCircle className="h-4 w-4 text-green-400" />
                      <span className="text-sm text-green-400">Installed & Ready</span>
                    </div>
                    
                    {app.assetId && (
                      <div className="text-xs text-gray-400">
                        Asset ID: {app.assetId}
                      </div>
                    )}
                    
                    <div className="flex gap-2">
                      <Button size="sm" className="flex-1 bg-green-600 hover:bg-green-700">
                        <Play className="h-3 w-3 mr-1" />
                        Run
                      </Button>
                      <Button variant="outline" size="sm" className="border-purple-500/30">
                        <Settings className="h-3 w-3" />
                      </Button>
                      <Button variant="outline" size="sm" className="border-red-500/30 text-red-400">
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : (
            <div className="text-center py-12">
              <Package className="h-16 w-16 text-gray-600 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-gray-300 mb-2">No Installed Applications</h3>
              <p className="text-gray-400">Install applications from the catalog to see them here.</p>
            </div>
          )}
        </TabsContent>

        <TabsContent value="vm-assets" className="space-y-4">
          {vmAssets && vmAssets.length > 0 ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {vmAssets.map((asset) => (
                <Card key={asset.id} className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
                  <CardHeader className="pb-2">
                    <CardTitle className="text-lg text-white flex items-center gap-2">
                      <HardDrive className="h-4 w-4" />
                      {asset.name}
                    </CardTitle>
                    <CardDescription className="text-gray-400 text-sm">
                      VM Asset • {asset.status}
                    </CardDescription>
                  </CardHeader>
                  
                  <CardContent className="space-y-4">
                    <div className="space-y-2 text-xs">
                      <div className="flex justify-between">
                        <span className="text-gray-400">CPU</span>
                        <span className="text-blue-400">{asset.vmConfig?.resourceLimits?.maxCpu || 'N/A'}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Memory</span>
                        <span className="text-blue-400">{asset.vmConfig?.resourceLimits?.maxMemory || 'N/A'}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Privacy</span>
                        <span className="text-blue-400">{asset.privacyLevel || 'N/A'}</span>
                      </div>
                    </div>
                    
                    <div className="flex gap-2">
                      <Button size="sm" className="flex-1 bg-blue-600 hover:bg-blue-700">
                        <Play className="h-3 w-3 mr-1" />
                        Execute
                      </Button>
                      <Button variant="outline" size="sm" className="border-purple-500/30">
                        <Activity className="h-3 w-3" />
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : (
            <div className="text-center py-12">
              <HardDrive className="h-16 w-16 text-gray-600 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-gray-300 mb-2">No VM Assets</h3>
              <p className="text-gray-400">Install applications to create VM assets.</p>
            </div>
          )}
        </TabsContent>

        <TabsContent value="executions" className="space-y-4">
          {executions && executions.length > 0 ? (
            <div className="space-y-4">
              {executions.map((execution) => (
                <Card key={execution.id} className="bg-black/40 border-orange-500/30 backdrop-blur-lg">
                  <CardContent className="p-4">
                    <div className="flex justify-between items-center">
                      <div>
                        <h4 className="text-white font-medium">{execution.vmAssetId}</h4>
                        <p className="text-gray-400 text-sm">
                          Operation: {execution.request.operation} • Status: {execution.status}
                        </p>
                      </div>
                      <Badge className="bg-orange-500/20 text-orange-400 border-orange-500/30">
                        {execution.status}
                      </Badge>
                    </div>
                    
                    {execution.execution?.startTime && (
                      <div className="mt-2 text-xs text-gray-400">
                        Started: {new Date(execution.execution.startTime).toLocaleString()}
                      </div>
                    )}
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : (
            <div className="text-center py-12">
              <Activity className="h-16 w-16 text-gray-600 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-gray-300 mb-2">No Executions</h3>
              <p className="text-gray-400">Execute VM assets to see execution history here.</p>
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
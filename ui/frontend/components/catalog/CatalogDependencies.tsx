// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { 
  Package, 
  GitBranch,
  AlertTriangle,
  CheckCircle,
  RefreshCw,
  Download,
  Settings
} from 'lucide-react';

// Mock dependency data for demonstration
const mockDependencies = [
  {
    id: '1',
    name: 'HyperMesh-Core',
    version: '1.2.3',
    status: 'installed',
    description: 'Core HyperMesh runtime and asset management',
    dependents: ['my-app', 'data-processor'],
    size: '45MB'
  },
  {
    id: '2',
    name: 'Julia-ML',
    version: '2.1.0',
    status: 'update-available',
    description: 'Machine learning utilities for Julia',
    dependents: ['ml-trainer'],
    size: '128MB'
  },
  {
    id: '3',
    name: 'StateProof-Utils',
    version: '0.8.5',
    status: 'missing',
    description: 'State proof generation and validation',
    dependents: ['verified-app'],
    size: '12MB'
  }
];

export function CatalogDependencies() {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'installed': return <CheckCircle className="h-4 w-4 text-green-400" />;
      case 'update-available': return <RefreshCw className="h-4 w-4 text-yellow-400" />;
      case 'missing': return <AlertTriangle className="h-4 w-4 text-red-400" />;
      default: return <Package className="h-4 w-4 text-gray-400" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'installed': return 'bg-green-500/20 text-green-400 border-green-500/30';
      case 'update-available': return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      case 'missing': return 'bg-red-500/20 text-red-400 border-red-500/30';
      default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
    }
  };

  const getActionButton = (status: string, dep: any) => {
    switch (status) {
      case 'installed':
        return (
          <Button variant="outline" size="sm" className="border-purple-500/30">
            <Settings className="h-3 w-3 mr-1" />
            Manage
          </Button>
        );
      case 'update-available':
        return (
          <Button size="sm" className="bg-yellow-600 hover:bg-yellow-700">
            <RefreshCw className="h-3 w-3 mr-1" />
            Update
          </Button>
        );
      case 'missing':
        return (
          <Button size="sm" className="bg-red-600 hover:bg-red-700">
            <Download className="h-3 w-3 mr-1" />
            Install
          </Button>
        );
      default:
        return null;
    }
  };

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-orange-400 to-red-600 bg-clip-text text-transparent mb-2">
          Dependency Management
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Manage dependencies and resolve conflicts across your HyperMesh applications and VM assets.
        </p>
      </div>

      {/* Summary Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <CheckCircle className="h-8 w-8 text-green-400" />
              <div>
                <h3 className="text-xl font-bold text-white">
                  {mockDependencies.filter(d => d.status === 'installed').length}
                </h3>
                <p className="text-sm text-gray-400">Installed</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <RefreshCw className="h-8 w-8 text-yellow-400" />
              <div>
                <h3 className="text-xl font-bold text-white">
                  {mockDependencies.filter(d => d.status === 'update-available').length}
                </h3>
                <p className="text-sm text-gray-400">Updates Available</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardContent className="p-4">
            <div className="flex items-center gap-3">
              <AlertTriangle className="h-8 w-8 text-red-400" />
              <div>
                <h3 className="text-xl font-bold text-white">
                  {mockDependencies.filter(d => d.status === 'missing').length}
                </h3>
                <p className="text-sm text-gray-400">Missing</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Global Actions */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardContent className="p-4">
          <div className="flex gap-4 items-center">
            <div className="flex-1">
              <h3 className="text-white font-medium">Dependency Actions</h3>
              <p className="text-gray-400 text-sm">Manage all dependencies at once</p>
            </div>
            <div className="flex gap-2">
              <Button className="bg-green-600 hover:bg-green-700">
                <RefreshCw className="h-4 w-4 mr-2" />
                Update All
              </Button>
              <Button variant="outline" className="border-purple-500/30">
                <Download className="h-4 w-4 mr-2" />
                Install Missing
              </Button>
              <Button variant="outline" className="border-purple-500/30">
                <GitBranch className="h-4 w-4 mr-2" />
                Resolve Conflicts
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Dependencies List */}
      <div className="space-y-4">
        {mockDependencies.map((dep) => (
          <Card key={dep.id} className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardContent className="p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-2">
                    {getStatusIcon(dep.status)}
                    <Package className="h-6 w-6 text-purple-400" />
                  </div>
                  
                  <div className="flex-1">
                    <div className="flex items-center gap-3">
                      <h3 className="text-lg font-semibold text-white">{dep.name}</h3>
                      <Badge variant="secondary" className="text-xs">
                        v{dep.version}
                      </Badge>
                      <Badge className={`text-xs border ${getStatusColor(dep.status)}`}>
                        {dep.status.replace('-', ' ')}
                      </Badge>
                    </div>
                    
                    <p className="text-gray-400 text-sm mt-1">{dep.description}</p>
                    
                    <div className="flex items-center gap-4 mt-2 text-xs text-gray-400">
                      <span>Size: {dep.size}</span>
                      <span>Used by: {dep.dependents.join(', ')}</span>
                    </div>
                  </div>
                </div>
                
                <div className="flex items-center gap-2">
                  {getActionButton(dep.status, dep)}
                </div>
              </div>

              {/* Dependency Graph Visualization */}
              {dep.dependents.length > 0 && (
                <div className="mt-4 pt-4 border-t border-purple-500/20">
                  <div className="flex items-center gap-2 mb-2">
                    <GitBranch className="h-4 w-4 text-gray-400" />
                    <span className="text-sm text-gray-400">Dependency Graph</span>
                  </div>
                  <div className="flex gap-2 flex-wrap">
                    {dep.dependents.map((dependent) => (
                      <Badge 
                        key={dependent}
                        variant="outline" 
                        className="text-xs border-purple-500/30 text-purple-300"
                      >
                        {dependent}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {mockDependencies.length === 0 && (
        <div className="text-center py-12">
          <Package className="h-16 w-16 text-gray-600 mx-auto mb-4" />
          <h3 className="text-xl font-semibold text-gray-300 mb-2">No Dependencies</h3>
          <p className="text-gray-400">Install applications to see their dependencies here.</p>
        </div>
      )}
    </div>
  );
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  Plus,
  Upload,
  Settings,
  Package,
  Code,
  FileText
} from 'lucide-react';
import { useCreateVMAsset } from '@/lib/api/hooks/useAssets';
import { PrivacyLevel } from '@/lib/api/services/HyperMeshAPI';
import type { CatalogApplication } from '@/lib/api/services/HyperMeshAPI';

export function CatalogCreate() {
  const createVMAsset = useCreateVMAsset();
  const [formData, setFormData] = React.useState({
    name: '',
    description: '',
    sourceCode: '',
    privacyLevel: 'private' as PrivacyLevel,
    resourceLimits: {
      maxCpu: 1,
      maxMemory: '1GB',
      maxStorage: '1GB',
      maxExecutionTime: 300
    }
  });

  const handleCreateVMAsset = async () => {
    try {
      const catalogApp: CatalogApplication = {
        id: `create-${Date.now()}`,
        name: formData.name,
        version: '1.0.0',
        type: 'Application',
        adapter: 'Native',
        status: 'Available',
        description: formData.description || formData.name,
        requirements: {
          cpu: formData.resourceLimits.maxCpu,
          memory: parseInt(formData.resourceLimits.maxMemory) || 1,
          storage: parseInt(formData.resourceLimits.maxStorage) || 1,
        },
        dependencies: [],
        author: 'local',
        downloads: 0,
        rating: 0,
        size: '0',
        lastUpdated: new Date().toISOString(),
      };
      await createVMAsset.mutateAsync({
        catalogApp,
        config: {
          privacyLevel: formData.privacyLevel,
          resourceLimits: formData.resourceLimits,
        }
      });
      alert('VM Asset created successfully!');
      // Reset form
      setFormData({
        name: '',
        description: '',
        sourceCode: '',
        privacyLevel: 'private',
        resourceLimits: {
          maxCpu: 1,
          maxMemory: '1GB',
          maxStorage: '1GB',
          maxExecutionTime: 300
        }
      });
    } catch (error) {
      console.error('Failed to create VM asset:', error);
      alert('Failed to create VM asset. Check console for details.');
    }
  };

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-blue-400 to-purple-600 bg-clip-text text-transparent mb-2">
          Create HyperMesh Assets
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Create and deploy custom applications as HyperMesh VM assets with Proof of State verification.
        </p>
      </div>

      <Tabs defaultValue="vm-asset" className="space-y-6">
        <TabsList className="bg-black/40 border border-purple-500/30">
          <TabsTrigger value="vm-asset">VM Asset</TabsTrigger>
          <TabsTrigger value="upload">Upload Application</TabsTrigger>
          <TabsTrigger value="template">From Template</TabsTrigger>
        </TabsList>

        <TabsContent value="vm-asset" className="space-y-6">
          <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Package className="h-5 w-5" />
                Create VM Asset
              </CardTitle>
              <CardDescription>
                Create a new virtual machine asset with custom source code and resource allocation.
              </CardDescription>
            </CardHeader>
            
            <CardContent className="space-y-6">
              {/* Basic Information */}
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">
                    Asset Name
                  </label>
                  <input
                    type="text"
                    value={formData.name}
                    onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
                    placeholder="Enter asset name..."
                    className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white placeholder:text-gray-400"
                  />
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">
                    Description
                  </label>
                  <textarea
                    value={formData.description}
                    onChange={(e) => setFormData(prev => ({ ...prev, description: e.target.value }))}
                    placeholder="Describe your asset..."
                    rows={3}
                    className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white placeholder:text-gray-400"
                  />
                </div>
              </div>

              {/* Source Code */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Source Code (Julia)
                </label>
                <textarea
                  value={formData.sourceCode}
                  onChange={(e) => setFormData(prev => ({ ...prev, sourceCode: e.target.value }))}
                  placeholder={`# Enter Julia code here...
function main()
    println("Hello HyperMesh!")
end

main()`}
                  rows={12}
                  className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white placeholder:text-gray-400 font-mono text-sm"
                />
              </div>

              {/* Privacy Level */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Privacy Level
                </label>
                <select
                  value={formData.privacyLevel}
                  onChange={(e) => setFormData(prev => ({ ...prev, privacyLevel: e.target.value as PrivacyLevel }))}
                  className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white"
                >
                  <option value="private">Private (Internal only)</option>
                  <option value="federated">Federated (Trusted networks)</option>
                  <option value="public">Public (Cross-network)</option>
                  <option value="anonymous">Anonymous (Privacy-first)</option>
                  <option value="verified">Verified (Full state proof)</option>
                </select>
              </div>

              {/* Resource Limits */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-4">
                  Resource Limits
                </label>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Max CPU Cores</label>
                    <input
                      type="number"
                      min="1"
                      max="16"
                      value={formData.resourceLimits.maxCpu}
                      onChange={(e) => setFormData(prev => ({ 
                        ...prev, 
                        resourceLimits: { 
                          ...prev.resourceLimits, 
                          maxCpu: parseInt(e.target.value) 
                        }
                      }))}
                      className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Max Memory</label>
                    <select
                      value={formData.resourceLimits.maxMemory}
                      onChange={(e) => setFormData(prev => ({ 
                        ...prev, 
                        resourceLimits: { 
                          ...prev.resourceLimits, 
                          maxMemory: e.target.value 
                        }
                      }))}
                      className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white text-sm"
                    >
                      <option value="512MB">512MB</option>
                      <option value="1GB">1GB</option>
                      <option value="2GB">2GB</option>
                      <option value="4GB">4GB</option>
                      <option value="8GB">8GB</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Max Storage</label>
                    <select
                      value={formData.resourceLimits.maxStorage}
                      onChange={(e) => setFormData(prev => ({ 
                        ...prev, 
                        resourceLimits: { 
                          ...prev.resourceLimits, 
                          maxStorage: e.target.value 
                        }
                      }))}
                      className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white text-sm"
                    >
                      <option value="1GB">1GB</option>
                      <option value="5GB">5GB</option>
                      <option value="10GB">10GB</option>
                      <option value="25GB">25GB</option>
                      <option value="50GB">50GB</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Max Execution Time (sec)</label>
                    <input
                      type="number"
                      min="60"
                      max="3600"
                      value={formData.resourceLimits.maxExecutionTime}
                      onChange={(e) => setFormData(prev => ({ 
                        ...prev, 
                        resourceLimits: { 
                          ...prev.resourceLimits, 
                          maxExecutionTime: parseInt(e.target.value) 
                        }
                      }))}
                      className="w-full px-3 py-2 bg-black/40 border border-purple-500/30 rounded text-white text-sm"
                    />
                  </div>
                </div>
              </div>

              {/* Create Button */}
              <div className="pt-4">
                <Button 
                  onClick={handleCreateVMAsset}
                  disabled={createVMAsset.isPending || !formData.name || !formData.sourceCode}
                  className="w-full bg-purple-600 hover:bg-purple-700 text-white"
                >
                  <Plus className="h-4 w-4 mr-2" />
                  {createVMAsset.isPending ? 'Creating Asset...' : 'Create VM Asset'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="upload" className="space-y-6">
          <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Upload className="h-5 w-5" />
                Upload Application
              </CardTitle>
              <CardDescription>
                Upload an existing application or package to create a HyperMesh asset.
              </CardDescription>
            </CardHeader>
            
            <CardContent className="space-y-6">
              <div className="border-2 border-dashed border-purple-500/30 rounded-lg p-8 text-center">
                <Upload className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <h3 className="text-lg font-medium text-gray-300 mb-2">
                  Drag & Drop Files Here
                </h3>
                <p className="text-gray-400 mb-4">
                  Upload .jl files, packages, or archives
                </p>
                <Button variant="outline" className="border-purple-500/30">
                  Choose Files
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="template" className="space-y-6">
          <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <FileText className="h-5 w-5" />
                Asset Templates
              </CardTitle>
              <CardDescription>
                Start with pre-built templates for common use cases.
              </CardDescription>
            </CardHeader>
            
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2">
                {[
                  { name: 'Hello World', description: 'Simple starter template', icon: Code },
                  { name: 'Data Processing', description: 'Julia data analysis template', icon: Package },
                  { name: 'API Service', description: 'HTTP API service template', icon: Settings },
                  { name: 'Machine Learning', description: 'ML model training template', icon: Code }
                ].map((template) => (
                  <Card key={template.name} className="bg-black/20 border-purple-500/20 hover:border-purple-400/30 transition-all cursor-pointer">
                    <CardContent className="p-4">
                      <div className="flex items-center gap-3">
                        <template.icon className="h-8 w-8 text-purple-400" />
                        <div>
                          <h4 className="text-white font-medium">{template.name}</h4>
                          <p className="text-gray-400 text-sm">{template.description}</p>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
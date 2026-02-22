// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Asset Creation Wizard - Standalone Component
 * 
 * Streamlined asset creation workflow with step-by-step guidance.
 * Integrates with the main AssetManager but can be used independently.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { useCreateAsset, useCreateVMAsset } from '@/lib/api';
import type { CatalogApplication } from '@/lib/api/services/HyperMeshAPI';
import { 
  Plus,
  Lock,
  Users,
  Globe,
  Shield,
  Cpu,
  HardDrive,
  MemoryStick,
  Network,
  Monitor,
  CheckCircle,
  ArrowLeft,
  ArrowRight,
  Settings
} from 'lucide-react';

interface WizardStep {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<any>;
}

interface AssetConfiguration {
  name: string;
  type: 'cpu' | 'storage' | 'network' | 'vm';
  privacyLevel: 'private' | 'private_network' | 'p2p' | 'public_network' | 'full_public';
  resourceLimits: {
    cpu: number;
    memory: string;
    storage: string;
    network: string;
  };
  vmConfig?: {
    runtime: 'julia' | 'python' | 'javascript' | 'rust';
    environmentVariables: Record<string, string>;
  };
}

interface AssetCreationWizardProps {
  onComplete?: (assetId: string) => void;
  onCancel?: () => void;
  initialConfig?: Partial<AssetConfiguration>;
}

export function AssetCreationWizard({ 
  onComplete, 
  onCancel, 
  initialConfig = {} 
}: AssetCreationWizardProps) {
  const createAsset = useCreateAsset();
  const createVMAsset = useCreateVMAsset();
  
  const [currentStep, setCurrentStep] = React.useState(0);
  const [config, setConfig] = React.useState<AssetConfiguration>({
    name: '',
    type: 'cpu',
    privacyLevel: 'private_network',
    resourceLimits: {
      cpu: 2,
      memory: '4GB',
      storage: '50GB',
      network: '100Mbps'
    },
    ...initialConfig
  });

  const steps: WizardStep[] = [
    {
      id: 'type',
      title: 'Asset Type',
      description: 'Choose the type of asset to create',
      icon: Settings
    },
    {
      id: 'basic',
      title: 'Basic Info',
      description: 'Set name and configuration',
      icon: Plus
    },
    {
      id: 'privacy',
      title: 'Privacy Level',
      description: 'Configure sharing scope',
      icon: Shield
    },
    {
      id: 'resources',
      title: 'Resources',
      description: 'Set resource limits',
      icon: Cpu
    },
    {
      id: 'review',
      title: 'Review',
      description: 'Confirm and create',
      icon: CheckCircle
    }
  ];

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handlePrevious = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleCreate = async () => {
    try {
      let assetId: string;
      
      if (config.type === 'vm') {
        // Build a CatalogApplication stub for the VM creation
        const catalogApp = {
          id: `wizard-${Date.now()}`,
          name: config.name,
          version: '1.0.0',
          type: 'Application' as const,
          adapter: (config.vmConfig?.runtime === 'python' ? 'Python' :
                    config.vmConfig?.runtime === 'javascript' ? 'Node.js' :
                    config.vmConfig?.runtime === 'rust' ? 'Native' : 'Julia') as CatalogApplication['adapter'],
          status: 'Available' as const,
          description: config.name,
          requirements: {
            cpu: config.resourceLimits.cpu,
            memory: parseInt(config.resourceLimits.memory) || 4,
            storage: parseInt(config.resourceLimits.storage) || 50,
          },
          dependencies: [],
          author: 'local',
          downloads: 0,
          rating: 0,
          size: '0',
          lastUpdated: new Date().toISOString(),
        };
        const result = await createVMAsset.mutateAsync({
          catalogApp,
          config: {
            privacyLevel: config.privacyLevel,
            resourceLimits: {
              maxCpu: config.resourceLimits.cpu,
              maxMemory: config.resourceLimits.memory,
              maxStorage: config.resourceLimits.storage
            },
          }
        });
        assetId = result.id;
      } else {
        const result = await createAsset.mutateAsync({
          name: config.name,
          type: config.type,
          privacyLevel: config.privacyLevel,
          owner: 'local',
          status: 'available',
          location: { nodeId: 'local', address: 'localhost' },
          specifications: config.resourceLimits,
          allocation: { totalCapacity: 0, allocatedCapacity: 0, availableCapacity: 0, unit: 'units' },
        });
        assetId = result.id;
      }
      
      onComplete?.(assetId);
    } catch (error) {
      console.error('Asset creation failed:', error);
      alert('Asset creation failed. Check console for details.');
    }
  };

  const canProceed = () => {
    switch (currentStep) {
      case 0: return true; // Type selection
      case 1: return config.name.trim().length > 0;
      case 2: return true; // Privacy level
      case 3: return true; // Resources
      case 4: return true; // Review
      default: return false;
    }
  };

  const isCreating = createAsset.isPending || createVMAsset.isPending;

  return (
    <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg max-w-4xl mx-auto">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Plus className="h-5 w-5 text-blue-400" />
          Asset Creation Wizard
        </CardTitle>
        <CardDescription className="text-gray-400">
          Create a new asset with guided configuration and privacy controls
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Progress Indicator */}
        <div className="flex items-center justify-between">
          {steps.map((step, index) => {
            const Icon = step.icon;
            const isCompleted = index < currentStep;
            const isCurrent = index === currentStep;
            
            return (
              <div key={step.id} className="flex items-center">
                <div className={cn(
                  'flex items-center justify-center w-10 h-10 rounded-full border-2 transition-all',
                  isCompleted ? 'bg-green-500 border-green-500 text-white' :
                  isCurrent ? 'bg-blue-500 border-blue-500 text-white' :
                  'bg-gray-800 border-gray-600 text-gray-400'
                )}>
                  {isCompleted ? <CheckCircle className="h-5 w-5" /> : <Icon className="h-5 w-5" />}
                </div>
                <div className={cn(
                  'ml-3',
                  isCurrent ? 'text-white' : 'text-gray-400'
                )}>
                  <div className="font-medium text-sm">{step.title}</div>
                  <div className="text-xs hidden md:block">{step.description}</div>
                </div>
                {index < steps.length - 1 && (
                  <div className={cn(
                    'h-0.5 w-8 mx-4 transition-colors',
                    isCompleted ? 'bg-green-500' : 'bg-gray-600'
                  )} />
                )}
              </div>
            );
          })}
        </div>

        <div className="min-h-[400px]">
          {/* Step 0: Asset Type Selection */}
          {currentStep === 0 && (
            <div className="space-y-4">
              <h3 className="text-white font-medium text-lg">Choose Asset Type</h3>
              <div className="grid gap-4 md:grid-cols-2">
                {[
                  {
                    type: 'cpu' as const,
                    icon: Cpu,
                    title: 'Compute Resource',
                    desc: 'CPU and processing power for computational tasks',
                    color: 'blue'
                  },
                  { 
                    type: 'storage' as const, 
                    icon: HardDrive, 
                    title: 'Storage Resource', 
                    desc: 'Persistent storage for data and files',
                    color: 'green'
                  },
                  { 
                    type: 'network' as const, 
                    icon: Network, 
                    title: 'Network Resource', 
                    desc: 'Bandwidth and network connectivity',
                    color: 'purple'
                  },
                  { 
                    type: 'vm' as const, 
                    icon: Monitor, 
                    title: 'Virtual Machine', 
                    desc: 'Complete VM environment with runtime support',
                    color: 'cyan'
                  }
                ].map((option) => {
                  const Icon = option.icon;
                  const isSelected = config.type === option.type;
                  
                  return (
                    <div
                      key={option.type}
                      onClick={() => setConfig(prev => ({ ...prev, type: option.type }))}
                      className={cn(
                        'p-6 rounded-lg border cursor-pointer transition-all',
                        isSelected ? 
                          `bg-${option.color}-500/10 border-${option.color}-500/40 ring-2 ring-${option.color}-500/30` :
                          'bg-gray-800/50 border-gray-600/30 hover:border-gray-500/50'
                      )}
                    >
                      <div className="flex items-start gap-4">
                        <Icon className={cn(
                          'h-8 w-8 mt-1',
                          option.color === 'blue' ? 'text-blue-400' :
                          option.color === 'green' ? 'text-green-400' :
                          option.color === 'purple' ? 'text-purple-400' :
                          'text-cyan-400'
                        )} />
                        <div>
                          <h4 className="text-white font-medium text-lg">{option.title}</h4>
                          <p className="text-gray-400 text-sm mt-1">{option.desc}</p>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Step 1: Basic Information */}
          {currentStep === 1 && (
            <div className="space-y-6">
              <h3 className="text-white font-medium text-lg">Basic Information</h3>
              <div className="space-y-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-300">Asset Name</label>
                  <input
                    type="text"
                    value={config.name}
                    onChange={(e) => setConfig(prev => ({ ...prev, name: e.target.value }))}
                    placeholder="Enter a descriptive name for your asset..."
                    className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                  />
                  <p className="text-xs text-gray-400">
                    Choose a name that clearly identifies the purpose of this asset
                  </p>
                </div>

                {config.type === 'vm' && (
                  <div className="space-y-2">
                    <label className="text-sm font-medium text-gray-300">Runtime Environment</label>
                    <select
                      value={config.vmConfig?.runtime || 'julia'}
                      onChange={(e) => setConfig(prev => ({ 
                        ...prev, 
                        vmConfig: { 
                          ...prev.vmConfig, 
                          runtime: e.target.value as any,
                          environmentVariables: prev.vmConfig?.environmentVariables || {}
                        }
                      }))}
                      className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                    >
                      <option value="julia">Julia Runtime</option>
                      <option value="python">Python Runtime</option>
                      <option value="javascript">JavaScript Runtime</option>
                      <option value="rust">Rust Runtime</option>
                    </select>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Step 2: Privacy Level */}
          {currentStep === 2 && (
            <div className="space-y-6">
              <h3 className="text-white font-medium text-lg">Privacy & Sharing Configuration</h3>
              <div className="space-y-4">
                {[
                  {
                    level: 'private' as const,
                    icon: Lock,
                    title: 'Private',
                    desc: 'Resources available only to your local applications and trusted processes',
                    color: 'red',
                    features: ['Local access only', 'Maximum security', 'No external sharing']
                  },
                  {
                    level: 'private_network' as const,
                    icon: Users,
                    title: 'Private Network',
                    desc: 'Shared with trusted networks, verified peers, and federated partners',
                    color: 'blue',
                    features: ['Trusted network sharing', 'Verified peer access', 'Balanced security']
                  },
                  {
                    level: 'full_public' as const,
                    icon: Globe,
                    title: 'Public',
                    desc: 'Available to the global HyperMesh network with full consensus validation',
                    color: 'green',
                    features: ['Global network access', 'Maximum rewards', 'Full consensus required']
                  }
                ].map((option) => {
                  const Icon = option.icon;
                  const isSelected = config.privacyLevel === option.level;
                  
                  return (
                    <div
                      key={option.level}
                      onClick={() => setConfig(prev => ({ ...prev, privacyLevel: option.level }))}
                      className={cn(
                        'p-6 rounded-lg border cursor-pointer transition-all',
                        isSelected ? 
                          `bg-${option.color}-500/10 border-${option.color}-500/40 ring-2 ring-${option.color}-500/30` :
                          'bg-gray-800/50 border-gray-600/30 hover:border-gray-500/50'
                      )}
                    >
                      <div className="flex items-start gap-4">
                        <Icon className={cn(
                          'h-8 w-8 mt-1',
                          option.color === 'red' ? 'text-red-400' :
                          option.color === 'blue' ? 'text-blue-400' :
                          'text-green-400'
                        )} />
                        <div className="flex-1">
                          <h4 className="text-white font-medium text-lg">{option.title}</h4>
                          <p className="text-gray-400 text-sm mt-1 mb-3">{option.desc}</p>
                          <div className="flex flex-wrap gap-2">
                            {option.features.map((feature) => (
                              <Badge key={feature} variant="outline" className="text-xs bg-gray-500/20 text-gray-300">
                                {feature}
                              </Badge>
                            ))}
                          </div>
                        </div>
                        {isSelected && (
                          <CheckCircle className={cn(
                            'h-6 w-6',
                            option.color === 'red' ? 'text-red-400' :
                            option.color === 'blue' ? 'text-blue-400' :
                            'text-green-400'
                          )} />
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Step 3: Resource Limits */}
          {currentStep === 3 && (
            <div className="space-y-6">
              <h3 className="text-white font-medium text-lg">Resource Allocation Limits</h3>
              <div className="grid gap-6 md:grid-cols-2">
                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
                    <Cpu className="h-4 w-4 text-blue-400" />
                    CPU Cores
                  </label>
                  <input
                    type="number"
                    value={config.resourceLimits.cpu}
                    onChange={(e) => setConfig(prev => ({ 
                      ...prev, 
                      resourceLimits: { ...prev.resourceLimits, cpu: parseInt(e.target.value) || 1 }
                    }))}
                    min="1"
                    max="32"
                    className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                  />
                  <p className="text-xs text-gray-400">Number of CPU cores to allocate</p>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
                    <MemoryStick className="h-4 w-4 text-green-400" />
                    Memory
                  </label>
                  <select
                    value={config.resourceLimits.memory}
                    onChange={(e) => setConfig(prev => ({ 
                      ...prev, 
                      resourceLimits: { ...prev.resourceLimits, memory: e.target.value }
                    }))}
                    className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-green-500 focus:outline-none focus:ring-2 focus:ring-green-500/20"
                  >
                    <option value="1GB">1 GB</option>
                    <option value="2GB">2 GB</option>
                    <option value="4GB">4 GB</option>
                    <option value="8GB">8 GB</option>
                    <option value="16GB">16 GB</option>
                    <option value="32GB">32 GB</option>
                    <option value="64GB">64 GB</option>
                  </select>
                  <p className="text-xs text-gray-400">Maximum memory allocation</p>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
                    <HardDrive className="h-4 w-4 text-purple-400" />
                    Storage
                  </label>
                  <select
                    value={config.resourceLimits.storage}
                    onChange={(e) => setConfig(prev => ({ 
                      ...prev, 
                      resourceLimits: { ...prev.resourceLimits, storage: e.target.value }
                    }))}
                    className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-purple-500 focus:outline-none focus:ring-2 focus:ring-purple-500/20"
                  >
                    <option value="10GB">10 GB</option>
                    <option value="25GB">25 GB</option>
                    <option value="50GB">50 GB</option>
                    <option value="100GB">100 GB</option>
                    <option value="250GB">250 GB</option>
                    <option value="500GB">500 GB</option>
                    <option value="1TB">1 TB</option>
                  </select>
                  <p className="text-xs text-gray-400">Storage space allocation</p>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-gray-300 flex items-center gap-2">
                    <Network className="h-4 w-4 text-cyan-400" />
                    Network Bandwidth
                  </label>
                  <select
                    value={config.resourceLimits.network}
                    onChange={(e) => setConfig(prev => ({ 
                      ...prev, 
                      resourceLimits: { ...prev.resourceLimits, network: e.target.value }
                    }))}
                    className="w-full p-4 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-cyan-500 focus:outline-none focus:ring-2 focus:ring-cyan-500/20"
                  >
                    <option value="50Mbps">50 Mbps</option>
                    <option value="100Mbps">100 Mbps</option>
                    <option value="250Mbps">250 Mbps</option>
                    <option value="500Mbps">500 Mbps</option>
                    <option value="1Gbps">1 Gbps</option>
                    <option value="10Gbps">10 Gbps</option>
                  </select>
                  <p className="text-xs text-gray-400">Network bandwidth limit</p>
                </div>
              </div>
            </div>
          )}

          {/* Step 4: Review */}
          {currentStep === 4 && (
            <div className="space-y-6">
              <h3 className="text-white font-medium text-lg">Review Configuration</h3>
              <div className="bg-gray-800/50 rounded-lg p-6 space-y-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div>
                    <span className="text-gray-400 text-sm">Asset Name:</span>
                    <div className="text-white font-medium">{config.name || 'Unnamed Asset'}</div>
                  </div>
                  <div>
                    <span className="text-gray-400 text-sm">Asset Type:</span>
                    <div className="text-white font-medium capitalize">{config.type}</div>
                  </div>
                  <div>
                    <span className="text-gray-400 text-sm">Privacy Level:</span>
                    <div className="text-white font-medium capitalize">{config.privacyLevel}</div>
                  </div>
                  <div>
                    <span className="text-gray-400 text-sm">Resource Limits:</span>
                    <div className="text-white font-medium">
                      {config.resourceLimits.cpu} CPU, {config.resourceLimits.memory}, {config.resourceLimits.storage}, {config.resourceLimits.network}
                    </div>
                  </div>
                  {config.type === 'vm' && config.vmConfig && (
                    <div className="md:col-span-2">
                      <span className="text-gray-400 text-sm">VM Runtime:</span>
                      <div className="text-white font-medium capitalize">{config.vmConfig.runtime}</div>
                    </div>
                  )}
                </div>
              </div>
              
              <div className="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
                <h4 className="text-blue-400 font-medium text-sm mb-2">Ready to Create</h4>
                <p className="text-gray-300 text-sm">
                  Your asset will be created with the configuration above. Once created, it will be 
                  available for allocation and use according to the privacy level you selected.
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Navigation */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-600/30">
          <div className="flex gap-2">
            {currentStep > 0 && (
              <Button 
                variant="outline" 
                onClick={handlePrevious}
                className="border-gray-600 text-gray-400"
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Previous
              </Button>
            )}
            {onCancel && (
              <Button 
                variant="ghost" 
                onClick={onCancel}
                className="text-gray-400 hover:text-white"
              >
                Cancel
              </Button>
            )}
          </div>
          
          <div className="flex items-center gap-2">
            <div className="text-sm text-gray-400">
              Step {currentStep + 1} of {steps.length}
            </div>
            {currentStep < steps.length - 1 ? (
              <Button 
                onClick={handleNext}
                disabled={!canProceed()}
                className="bg-gradient-to-r from-blue-500 to-purple-600 hover:from-blue-400 hover:to-purple-500 text-black"
              >
                Next
                <ArrowRight className="h-4 w-4 ml-2" />
              </Button>
            ) : (
              <Button 
                onClick={handleCreate}
                disabled={!canProceed() || isCreating}
                className="bg-gradient-to-r from-green-500 to-blue-600 hover:from-green-400 hover:to-blue-500 text-black"
              >
                {isCreating ? 'Creating...' : 'Create Asset'}
              </Button>
            )}
          </div>
        </div>

        {/* Progress Bar */}
        <div className="space-y-2">
          <div className="flex justify-between text-xs text-gray-400">
            <span>Progress</span>
            <span>{Math.round(((currentStep + 1) / steps.length) * 100)}%</span>
          </div>
          <Progress value={((currentStep + 1) / steps.length) * 100} className="h-1" />
        </div>
      </CardContent>
    </Card>
  );
}
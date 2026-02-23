// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
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
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { useCreateAsset, useCreateVMAsset } from '@/lib/api';
import type { CatalogApplication } from '@/lib/api/services/HyperMeshAPI';
import { Plus, Shield, Cpu, CheckCircle, ArrowLeft, ArrowRight, Settings } from 'lucide-react';
import {
  TypeSelectionStep,
  BasicInfoStep,
  PrivacyStep,
  ResourcesStep,
  ReviewStep
} from './asset-creation-wizard';
import type { WizardStep, AssetConfiguration, AssetCreationWizardProps } from './asset-creation-wizard';

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
    { id: 'type', title: 'Asset Type', description: 'Choose the type of asset to create', icon: Settings },
    { id: 'basic', title: 'Basic Info', description: 'Set name and configuration', icon: Plus },
    { id: 'privacy', title: 'Privacy Level', description: 'Configure sharing scope', icon: Shield },
    { id: 'resources', title: 'Resources', description: 'Set resource limits', icon: Cpu },
    { id: 'review', title: 'Review', description: 'Confirm and create', icon: CheckCircle }
  ];

  const handleNext = () => {
    if (currentStep < steps.length - 1) setCurrentStep(currentStep + 1);
  };

  const handlePrevious = () => {
    if (currentStep > 0) setCurrentStep(currentStep - 1);
  };

  const handleCreate = async () => {
    try {
      let assetId: string;

      if (config.type === 'vm') {
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
      case 0: return true;
      case 1: return config.name.trim().length > 0;
      case 2: return true;
      case 3: return true;
      case 4: return true;
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
                <div className={cn('ml-3', isCurrent ? 'text-white' : 'text-gray-400')}>
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
          {currentStep === 0 && <TypeSelectionStep config={config} setConfig={setConfig} />}
          {currentStep === 1 && <BasicInfoStep config={config} setConfig={setConfig} />}
          {currentStep === 2 && <PrivacyStep config={config} setConfig={setConfig} />}
          {currentStep === 3 && <ResourcesStep config={config} setConfig={setConfig} />}
          {currentStep === 4 && <ReviewStep config={config} />}
        </div>

        {/* Navigation */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-600/30">
          <div className="flex gap-2">
            {currentStep > 0 && (
              <Button variant="outline" onClick={handlePrevious} className="border-gray-600 text-gray-400">
                <ArrowLeft className="h-4 w-4 mr-2" />
                Previous
              </Button>
            )}
            {onCancel && (
              <Button variant="ghost" onClick={onCancel} className="text-gray-400 hover:text-white">
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

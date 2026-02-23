// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import type { AssetCreationStep, NewAssetConfig } from './types';
import { Plus, Lock, Users, Globe } from 'lucide-react';

interface AssetCreationTabProps {
  creationStep: number;
  setCreationStep: (step: number) => void;
  creationSteps: AssetCreationStep[];
  newAssetConfig: NewAssetConfig;
  setNewAssetConfig: React.Dispatch<React.SetStateAction<NewAssetConfig>>;
  onCreateAsset: () => void;
  isCreating: boolean;
}

export function AssetCreationTab({
  creationStep,
  setCreationStep,
  creationSteps,
  newAssetConfig,
  setNewAssetConfig,
  onCreateAsset,
  isCreating
}: AssetCreationTabProps) {
  return (
    <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Plus className="h-5 w-5 text-blue-400" />
          Asset Creation Wizard
        </CardTitle>
        <CardDescription className="text-gray-400">
          Step-by-step guided asset creation with privacy controls and resource allocation
        </CardDescription>
      </CardHeader>
      <CardContent>
        {/* Creation Steps Progress */}
        <div className="flex items-center justify-between mb-8">
          {creationSteps.map((step, index) => (
            <div key={step.id} className="flex items-center">
              <div className={cn(
                'flex items-center justify-center w-10 h-10 rounded-full border-2 transition-colors',
                step.completed ? 'bg-green-500 border-green-500 text-white' :
                step.current ? 'bg-blue-500 border-blue-500 text-white' :
                'bg-gray-800 border-gray-600 text-gray-400'
              )}>
                {step.completed ? '\u2713' : index + 1}
              </div>
              <div className={cn(
                'ml-3 mr-8',
                step.current ? 'text-white' : 'text-gray-400'
              )}>
                <div className="font-medium text-sm">{step.title}</div>
                <div className="text-xs">{step.description}</div>
              </div>
              {index < creationSteps.length - 1 && (
                <div className={cn(
                  'h-0.5 w-12 transition-colors',
                  step.completed ? 'bg-green-500' : 'bg-gray-600'
                )} />
              )}
            </div>
          ))}
        </div>

        {/* Step Content */}
        <div className="space-y-6">
          {creationStep === 0 && (
            <BasicInfoStep
              config={newAssetConfig}
              setConfig={setNewAssetConfig}
            />
          )}

          {creationStep === 1 && (
            <PrivacyLevelStep
              config={newAssetConfig}
              setConfig={setNewAssetConfig}
            />
          )}

          {creationStep === 2 && (
            <ResourceLimitsStep
              config={newAssetConfig}
              setConfig={setNewAssetConfig}
            />
          )}

          {creationStep === 3 && (
            <ReviewStep config={newAssetConfig} />
          )}

          {/* Navigation Buttons */}
          <div className="flex items-center justify-between pt-6 border-t border-gray-600/30">
            <Button
              variant="outline"
              onClick={() => setCreationStep(Math.max(0, creationStep - 1))}
              disabled={creationStep === 0}
              className="border-gray-600 text-gray-400"
            >
              Previous
            </Button>

            {creationStep < 3 ? (
              <Button
                onClick={() => setCreationStep(creationStep + 1)}
                disabled={creationStep === 0 && !newAssetConfig.name}
                className="bg-gradient-to-r from-blue-500 to-purple-600 hover:from-blue-400 hover:to-purple-500 text-black"
              >
                Next
              </Button>
            ) : (
              <Button
                onClick={onCreateAsset}
                disabled={isCreating || !newAssetConfig.name}
                className="bg-gradient-to-r from-green-500 to-blue-600 hover:from-green-400 hover:to-blue-500 text-black"
              >
                {isCreating ? 'Creating...' : 'Create Asset'}
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function BasicInfoStep({
  config,
  setConfig
}: {
  config: NewAssetConfig;
  setConfig: React.Dispatch<React.SetStateAction<NewAssetConfig>>;
}) {
  return (
    <div className="space-y-4">
      <h3 className="text-white font-medium">Basic Asset Information</h3>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <label className="text-sm text-gray-400">Asset Name</label>
          <input
            type="text"
            value={config.name}
            onChange={(e) => setConfig(prev => ({ ...prev, name: e.target.value }))}
            placeholder="Enter asset name..."
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none"
          />
        </div>
        <div className="space-y-2">
          <label className="text-sm text-gray-400">Asset Type</label>
          <select
            value={config.type}
            onChange={(e) => setConfig(prev => ({ ...prev, type: e.target.value as any }))}
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
          >
            <option value="compute">Compute Resource</option>
            <option value="storage">Storage Resource</option>
            <option value="network">Network Resource</option>
            <option value="vm">Virtual Machine</option>
          </select>
        </div>
      </div>
    </div>
  );
}

function PrivacyLevelStep({
  config,
  setConfig
}: {
  config: NewAssetConfig;
  setConfig: React.Dispatch<React.SetStateAction<NewAssetConfig>>;
}) {
  const options = [
    { level: 'private' as const, icon: Lock, title: 'Private', desc: 'Resources available only to your local applications', color: 'red' },
    { level: 'federated' as const, icon: Users, title: 'Federated', desc: 'Shared with trusted networks and verified peers', color: 'blue' },
    { level: 'public' as const, icon: Globe, title: 'Public', desc: 'Available to the global HyperMesh network', color: 'green' }
  ];

  return (
    <div className="space-y-4">
      <h3 className="text-white font-medium">Privacy Level Configuration</h3>
      <div className="grid gap-4">
        {options.map((option) => {
          const Icon = option.icon;
          const isSelected = config.privacyLevel === option.level;
          return (
            <div
              key={option.level}
              onClick={() => setConfig(prev => ({ ...prev, privacyLevel: option.level }))}
              className={cn(
                'p-4 rounded-lg border cursor-pointer transition-all',
                isSelected
                  ? `bg-${option.color}-500/10 border-${option.color}-500/40 ring-2 ring-${option.color}-500/30`
                  : 'bg-gray-800/50 border-gray-600/30 hover:border-gray-500/50'
              )}
            >
              <div className="flex items-center gap-3">
                <Icon className={cn(
                  'h-6 w-6',
                  option.color === 'red' ? 'text-red-400' :
                  option.color === 'blue' ? 'text-blue-400' :
                  'text-green-400'
                )} />
                <div>
                  <h4 className="text-white font-medium">{option.title}</h4>
                  <p className="text-sm text-gray-400">{option.desc}</p>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ResourceLimitsStep({
  config,
  setConfig
}: {
  config: NewAssetConfig;
  setConfig: React.Dispatch<React.SetStateAction<NewAssetConfig>>;
}) {
  return (
    <div className="space-y-4">
      <h3 className="text-white font-medium">Resource Allocation Limits</h3>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <label className="text-sm text-gray-400">CPU Cores</label>
          <input
            type="number"
            value={config.resourceLimits.cpu}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, cpu: parseInt(e.target.value) }
            }))}
            min="1"
            max="16"
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
          />
        </div>
        <div className="space-y-2">
          <label className="text-sm text-gray-400">Memory</label>
          <select
            value={config.resourceLimits.memory}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, memory: e.target.value }
            }))}
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
          >
            <option value="2GB">2 GB</option>
            <option value="4GB">4 GB</option>
            <option value="8GB">8 GB</option>
            <option value="16GB">16 GB</option>
            <option value="32GB">32 GB</option>
          </select>
        </div>
        <div className="space-y-2">
          <label className="text-sm text-gray-400">Storage</label>
          <select
            value={config.resourceLimits.storage}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, storage: e.target.value }
            }))}
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
          >
            <option value="25GB">25 GB</option>
            <option value="50GB">50 GB</option>
            <option value="100GB">100 GB</option>
            <option value="250GB">250 GB</option>
            <option value="500GB">500 GB</option>
          </select>
        </div>
        <div className="space-y-2">
          <label className="text-sm text-gray-400">Network Bandwidth</label>
          <select
            value={config.resourceLimits.network}
            onChange={(e) => setConfig(prev => ({
              ...prev,
              resourceLimits: { ...prev.resourceLimits, network: e.target.value }
            }))}
            className="w-full p-3 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
          >
            <option value="100Mbps">100 Mbps</option>
            <option value="250Mbps">250 Mbps</option>
            <option value="500Mbps">500 Mbps</option>
            <option value="1Gbps">1 Gbps</option>
            <option value="10Gbps">10 Gbps</option>
          </select>
        </div>
      </div>
    </div>
  );
}

function ReviewStep({ config }: { config: NewAssetConfig }) {
  return (
    <div className="space-y-4">
      <h3 className="text-white font-medium">Review & Create Asset</h3>
      <div className="bg-gray-800/50 p-4 rounded-lg">
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <span className="text-gray-400">Name:</span>
            <div className="text-white font-medium">{config.name || 'Unnamed Asset'}</div>
          </div>
          <div>
            <span className="text-gray-400">Type:</span>
            <div className="text-white font-medium">{config.type}</div>
          </div>
          <div>
            <span className="text-gray-400">Privacy Level:</span>
            <div className="text-white font-medium">{config.privacyLevel}</div>
          </div>
          <div>
            <span className="text-gray-400">Resource Limits:</span>
            <div className="text-white font-medium">
              {config.resourceLimits.cpu} CPU, {config.resourceLimits.memory}, {config.resourceLimits.storage}, {config.resourceLimits.network}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

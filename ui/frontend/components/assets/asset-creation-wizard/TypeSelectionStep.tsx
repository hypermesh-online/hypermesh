// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';
import { Cpu, HardDrive, Network, Monitor } from 'lucide-react';
import type { AssetConfiguration } from './types';

interface TypeSelectionStepProps {
  config: AssetConfiguration;
  setConfig: React.Dispatch<React.SetStateAction<AssetConfiguration>>;
}

const assetTypeOptions = [
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
];

function getIconColor(color: string): string {
  switch (color) {
    case 'blue': return 'text-blue-400';
    case 'green': return 'text-green-400';
    case 'purple': return 'text-purple-400';
    default: return 'text-cyan-400';
  }
}

export function TypeSelectionStep({ config, setConfig }: TypeSelectionStepProps) {
  return (
    <div className="space-y-4">
      <h3 className="text-white font-medium text-lg">Choose Asset Type</h3>
      <div className="grid gap-4 md:grid-cols-2">
        {assetTypeOptions.map((option) => {
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
                <Icon className={cn('h-8 w-8 mt-1', getIconColor(option.color))} />
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
  );
}

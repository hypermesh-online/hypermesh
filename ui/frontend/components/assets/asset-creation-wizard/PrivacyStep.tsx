// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Lock, Users, Globe, CheckCircle } from 'lucide-react';
import type { AssetConfiguration } from './types';

interface PrivacyStepProps {
  config: AssetConfiguration;
  setConfig: React.Dispatch<React.SetStateAction<AssetConfiguration>>;
}

const privacyOptions = [
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
    desc: 'Available to the global HyperMesh network with full Proof of State verification',
    color: 'green',
    features: ['Global network access', 'Maximum rewards', 'Full state proof required']
  }
];

function getIconColor(color: string): string {
  switch (color) {
    case 'red': return 'text-red-400';
    case 'blue': return 'text-blue-400';
    default: return 'text-green-400';
  }
}

export function PrivacyStep({ config, setConfig }: PrivacyStepProps) {
  return (
    <div className="space-y-6">
      <h3 className="text-white font-medium text-lg">Privacy & Sharing Configuration</h3>
      <div className="space-y-4">
        {privacyOptions.map((option) => {
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
                <Icon className={cn('h-8 w-8 mt-1', getIconColor(option.color))} />
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
                  <CheckCircle className={cn('h-6 w-6', getIconColor(option.color))} />
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

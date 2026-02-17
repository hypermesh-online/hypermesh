// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface FeatureListProps {
  features: string[];
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  dotStyle?: 'circle' | 'square' | 'diamond';
  className?: string;
}

export function FeatureList({
  features,
  theme = 'cyan',
  dotStyle = 'circle',
  className
}: FeatureListProps) {
  const getThemeColor = () => {
    const colors = {
      cyan: 'bg-cyan-400',
      green: 'bg-green-400',
      purple: 'bg-purple-400',
      red: 'bg-red-400',
      yellow: 'bg-yellow-400'
    };
    return colors[theme];
  };

  const getDotStyle = () => {
    const styles = {
      circle: 'rounded-full',
      square: 'rounded-none',
      diamond: 'rotate-45'
    };
    return styles[dotStyle];
  };

  return (
    <div className={cn('space-y-2', className)}>
      {features.map((feature, index) => (
        <div key={index} className="flex items-center gap-2 text-sm text-gray-300">
          <div className={cn(
            'w-1.5 h-1.5',
            getThemeColor(),
            getDotStyle()
          )} />
          {feature}
        </div>
      ))}
    </div>
  );
}

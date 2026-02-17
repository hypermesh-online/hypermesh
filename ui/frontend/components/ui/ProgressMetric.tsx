// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';

interface ProgressMetricProps {
  label: string;
  value: number;
  maxValue?: number;
  unit?: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  showPercentage?: boolean;
  description?: string;
  status?: 'excellent' | 'good' | 'warning' | 'critical';
  className?: string;
}

export function ProgressMetric({
  label,
  value,
  maxValue = 100,
  unit,
  theme = 'cyan',
  showPercentage = true,
  description,
  status,
  className
}: ProgressMetricProps) {
  const percentage = (value / maxValue) * 100;

  const getStatusColor = () => {
    if (status) {
      const colors = {
        excellent: 'text-green-400',
        good: 'text-cyan-400',
        warning: 'text-yellow-400',
        critical: 'text-red-400'
      };
      return colors[status];
    }

    const themeColors = {
      cyan: 'text-cyan-400',
      green: 'text-green-400',
      purple: 'text-purple-400',
      red: 'text-red-400',
      yellow: 'text-yellow-400'
    };
    return themeColors[theme];
  };

  return (
    <div className={cn('space-y-2', className)}>
      <div className="flex justify-between items-center">
        <span className="text-sm font-medium text-white">{label}</span>
        <span className={cn('text-sm font-medium', getStatusColor())}>
          {value}{unit}
          {showPercentage && maxValue !== 100 && ` (${percentage.toFixed(1)}%)`}
        </span>
      </div>
      <Progress value={percentage} className="h-2" />
      {description && (
        <p className="text-xs text-gray-400">{description}</p>
      )}
    </div>
  );
}

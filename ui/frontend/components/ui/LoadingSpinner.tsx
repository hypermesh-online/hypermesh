// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function LoadingSpinner({
  size = 'md',
  theme = 'cyan',
  className
}: LoadingSpinnerProps) {
  const getSizeClasses = () => {
    const sizes = {
      sm: 'w-4 h-4',
      md: 'w-6 h-6',
      lg: 'w-8 h-8'
    };
    return sizes[size];
  };

  const getThemeColor = () => {
    const colors = {
      cyan: 'border-cyan-400',
      green: 'border-green-400',
      purple: 'border-purple-400',
      red: 'border-red-400',
      yellow: 'border-yellow-400'
    };
    return colors[theme];
  };

  return (
    <div className={cn(
      'animate-spin rounded-full border-2 border-gray-600 border-t-transparent',
      getSizeClasses(),
      getThemeColor(),
      className
    )} />
  );
}

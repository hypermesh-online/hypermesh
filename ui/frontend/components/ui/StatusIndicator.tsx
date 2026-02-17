// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface StatusIndicatorProps {
  status: 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'success';
  text?: string;
  showDot?: boolean;
  animate?: boolean;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function StatusIndicator({
  status,
  text,
  showDot = true,
  animate = false,
  size = 'md',
  className
}: StatusIndicatorProps) {
  const getStatusConfig = () => {
    const configs = {
      active: {
        color: 'bg-green-400',
        badge: 'bg-green-500/20 text-green-400 border-green-500/30',
        text: text || 'Active'
      },
      success: {
        color: 'bg-green-400',
        badge: 'bg-green-500/20 text-green-400 border-green-500/30',
        text: text || 'Success'
      },
      inactive: {
        color: 'bg-gray-400',
        badge: 'bg-gray-500/20 text-gray-400 border-gray-500/30',
        text: text || 'Inactive'
      },
      pending: {
        color: 'bg-yellow-400',
        badge: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
        text: text || 'Pending'
      },
      warning: {
        color: 'bg-yellow-400',
        badge: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
        text: text || 'Warning'
      },
      error: {
        color: 'bg-red-400',
        badge: 'bg-red-500/20 text-red-400 border-red-500/30',
        text: text || 'Error'
      }
    };
    return configs[status];
  };

  const getSizeClasses = () => {
    const sizes = {
      sm: 'w-1.5 h-1.5',
      md: 'w-2 h-2',
      lg: 'w-3 h-3'
    };
    return sizes[size];
  };

  const config = getStatusConfig();

  if (!text && showDot) {
    return (
      <div className={cn(
        'rounded-full',
        config.color,
        getSizeClasses(),
        animate && 'animate-pulse',
        className
      )} />
    );
  }

  return (
    <Badge variant="outline" className={cn(config.badge, className)}>
      {showDot && (
        <div className={cn(
          'rounded-full mr-1',
          config.color,
          getSizeClasses(),
          animate && 'animate-pulse'
        )} />
      )}
      {config.text}
    </Badge>
  );
}

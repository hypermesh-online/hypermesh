// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface ActivityItemProps {
  type?: 'info' | 'success' | 'warning' | 'error' | 'user' | 'system';
  message: string;
  time: string;
  details?: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function ActivityItem({
  type = 'info',
  message,
  time,
  details,
  theme = 'cyan',
  className
}: ActivityItemProps) {
  const getTypeColor = () => {
    const colors = {
      info: 'bg-cyan-400',
      success: 'bg-green-400',
      warning: 'bg-yellow-400',
      error: 'bg-red-400',
      user: 'bg-blue-400',
      system: 'bg-purple-400'
    };
    return colors[type];
  };

  const getThemeAccent = () => {
    const accents = {
      cyan: 'bg-cyan-500/10 border-cyan-500/20',
      green: 'bg-green-500/10 border-green-500/20',
      purple: 'bg-purple-500/10 border-purple-500/20',
      red: 'bg-red-500/10 border-red-500/20',
      yellow: 'bg-yellow-500/10 border-yellow-500/20'
    };
    return accents[theme];
  };

  return (
    <div className={cn(
      'flex items-center gap-4 p-3 rounded-lg transition-colors',
      getThemeAccent(),
      className
    )}>
      <div className={cn('w-2 h-2 rounded-full', getTypeColor())} />
      <div className="flex-1">
        <p className="text-sm text-white">{message}</p>
        {details && (
          <p className="text-xs text-gray-400 mt-1">{details}</p>
        )}
        <p className="text-xs text-gray-400">{time}</p>
      </div>
    </div>
  );
}

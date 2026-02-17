// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface NetworkStatusProps {
  name: string;
  status: 'connected' | 'disconnected' | 'connecting' | 'error';
  details?: string;
  showAnimation?: boolean;
  className?: string;
}

export function NetworkStatus({
  name,
  status,
  details,
  showAnimation = true,
  className
}: NetworkStatusProps) {
  const getStatusConfig = () => {
    const configs = {
      connected: {
        color: 'bg-green-400',
        badge: 'bg-green-500/20 text-green-400 border-green-500/30',
        text: 'Connected'
      },
      disconnected: {
        color: 'bg-gray-400',
        badge: 'bg-gray-500/20 text-gray-400 border-gray-500/30',
        text: 'Disconnected'
      },
      connecting: {
        color: 'bg-yellow-400',
        badge: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
        text: 'Connecting'
      },
      error: {
        color: 'bg-red-400',
        badge: 'bg-red-500/20 text-red-400 border-red-500/30',
        text: 'Error'
      }
    };
    return configs[status];
  };

  const config = getStatusConfig();

  return (
    <div className={cn(
      'flex items-center gap-2 px-3 py-1 rounded-full border',
      config.badge,
      className
    )}>
      <div className={cn(
        'w-2 h-2 rounded-full',
        config.color,
        showAnimation && (status === 'connected' || status === 'connecting') && 'animate-pulse'
      )} />
      <span className="text-xs font-medium">{name} {config.text}</span>
      {details && (
        <span className="text-xs opacity-70">• {details}</span>
      )}
    </div>
  );
}

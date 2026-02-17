// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Badge } from '@/components/ui/badge';
import { getStatusColor } from '../utils/statusHelpers';

interface StatusIndicatorProps {
  status: string;
  size?: 'sm' | 'default';
  className?: string;
}

export function StatusIndicator({ status, size = 'default', className }: StatusIndicatorProps) {
  return (
    <Badge 
      className={`${getStatusColor(status)} ${size === 'sm' ? 'text-xs' : ''} ${className || ''}`}
    >
      {status}
    </Badge>
  );
}
// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { CheckCircle, AlertTriangle, XCircle, Clock, TrendingUp, TrendingDown, Minus } from 'lucide-react';
import type { SystemStatus, MetricTrend } from './types';

export function getStatusIcon(status: SystemStatus['status']): React.ReactElement {
  switch (status) {
    case 'online':
      return <CheckCircle className="h-4 w-4 text-green-400" />;
    case 'warning':
      return <AlertTriangle className="h-4 w-4 text-yellow-400" />;
    case 'offline':
      return <XCircle className="h-4 w-4 text-red-400" />;
    case 'maintenance':
      return <Clock className="h-4 w-4 text-blue-400" />;
    default:
      return <CheckCircle className="h-4 w-4 text-gray-400" />;
  }
}

export function getStatusColor(status: SystemStatus['status']): string {
  switch (status) {
    case 'online':
      return 'text-green-400 bg-green-500/20 border-green-500/30';
    case 'warning':
      return 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30';
    case 'offline':
      return 'text-red-400 bg-red-500/20 border-red-500/30';
    case 'maintenance':
      return 'text-blue-400 bg-blue-500/20 border-blue-500/30';
    default:
      return 'text-gray-400 bg-gray-500/20 border-gray-500/30';
  }
}

export function getTrendIcon(trend: MetricTrend['trend']): React.ReactElement {
  switch (trend) {
    case 'up':
      return <TrendingUp className="h-3 w-3 text-green-400" />;
    case 'down':
      return <TrendingDown className="h-3 w-3 text-red-400" />;
    case 'stable':
      return <Minus className="h-3 w-3 text-gray-400" />;
  }
}

export function formatUptime(uptimeMs: number): string {
  const days = Math.floor(uptimeMs / (1000 * 60 * 60 * 24));
  const hours = Math.floor((uptimeMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  return `${days}d ${hours}h`;
}

export function getUptimePercentage(uptimeMs: number): number {
  const days = uptimeMs / (1000 * 60 * 60 * 24);
  const totalPossibleDays = 30;
  return Math.min((days / totalPossibleDays) * 100, 100);
}

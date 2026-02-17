// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: LucideIcon;
  trend?: {
    value: string;
    direction: 'up' | 'down' | 'neutral';
  };
  progress?: {
    value: number;
    max?: number;
    showPercentage?: boolean;
  };
  status?: 'excellent' | 'good' | 'warning' | 'critical';
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function MetricCard({
  title,
  value,
  subtitle,
  icon: Icon,
  trend,
  progress,
  status,
  theme = 'cyan',
  className
}: MetricCardProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-cyan-500/5',
        icon: 'text-cyan-400',
        value: 'text-cyan-400',
        accent: 'bg-cyan-500/10 border-cyan-500/20'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-green-500/5',
        icon: 'text-green-400',
        value: 'text-green-400',
        accent: 'bg-green-500/10 border-green-500/20'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-purple-500/5',
        icon: 'text-purple-400',
        value: 'text-purple-400',
        accent: 'bg-purple-500/10 border-purple-500/20'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-red-500/5',
        icon: 'text-red-400',
        value: 'text-red-400',
        accent: 'bg-red-500/10 border-red-500/20'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-yellow-500/5',
        icon: 'text-yellow-400',
        value: 'text-yellow-400',
        accent: 'bg-yellow-500/10 border-yellow-500/20'
      }
    };
    return themes[theme];
  };

  const getStatusIndicator = () => {
    if (!status) return null;
    
    const statusColors = {
      excellent: 'bg-green-400',
      good: 'bg-cyan-400',
      warning: 'bg-yellow-400',
      critical: 'bg-red-400'
    };

    return (
      <div className={cn("w-3 h-3 rounded-full", statusColors[status])} />
    );
  };

  const getTrendColor = () => {
    if (!trend) return '';
    return trend.direction === 'up' ? 'text-green-400' : 
           trend.direction === 'down' ? 'text-red-400' : 'text-gray-400';
  };

  const colors = getThemeColors();

  return (
    <Card className={cn(
      'bg-black/40 backdrop-blur-lg transition-all duration-300 hover:shadow-lg',
      colors.border,
      colors.bg,
      className
    )}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{title}</CardTitle>
        <div className="flex items-center gap-2">
          {getStatusIndicator()}
          {Icon && <Icon className={cn("h-4 w-4", colors.icon)} />}
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <div className={cn("text-2xl font-bold", colors.value)}>{value}</div>
          
          {subtitle && (
            <p className="text-xs text-gray-400">{subtitle}</p>
          )}
          
          {trend && (
            <p className={cn("text-xs font-medium", getTrendColor())}>
              {trend.value}
            </p>
          )}
          
          {progress && (
            <div className="space-y-1">
              <Progress value={progress.value} className="h-1" />
              {progress.showPercentage && (
                <p className="text-xs text-gray-400">
                  {progress.value.toFixed(1)}% {progress.max ? `of ${progress.max}` : 'complete'}
                </p>
              )}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

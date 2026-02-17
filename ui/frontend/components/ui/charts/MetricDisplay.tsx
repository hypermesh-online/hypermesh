// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';
import { SparklineChart } from './SparklineChart';
import { ProgressMetric } from '../ProgressMetric';

interface MetricData {
  value: number;
  timestamp?: string;
}

interface MetricDisplayProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: LucideIcon;
  trend?: {
    data: MetricData[];
    direction?: 'up' | 'down' | 'neutral';
    percentage?: string;
  };
  progress?: {
    value: number;
    max?: number;
    label?: string;
  };
  status?: 'excellent' | 'good' | 'warning' | 'critical';
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  size?: 'sm' | 'md' | 'lg';
  showSparkline?: boolean;
  className?: string;
}

export function MetricDisplay({
  title,
  value,
  subtitle,
  icon: Icon,
  trend,
  progress,
  status,
  theme = 'cyan',
  size = 'md',
  showSparkline = true,
  className
}: MetricDisplayProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-cyan-500/5',
        text: 'text-cyan-400',
        icon: 'text-cyan-400'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-green-500/5',
        text: 'text-green-400',
        icon: 'text-green-400'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-purple-500/5',
        text: 'text-purple-400',
        icon: 'text-purple-400'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-red-500/5',
        text: 'text-red-400',
        icon: 'text-red-400'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-yellow-500/5',
        text: 'text-yellow-400',
        icon: 'text-yellow-400'
      }
    };
    return themes[theme];
  };

  const getStatusColor = () => {
    if (!status) return '';
    const statusColors = {
      excellent: 'text-green-400',
      good: 'text-cyan-400',
      warning: 'text-yellow-400',
      critical: 'text-red-400'
    };
    return statusColors[status];
  };

  const getStatusIndicator = () => {
    if (!status) return null;
    const statusColors = {
      excellent: 'bg-green-400',
      good: 'bg-cyan-400',
      warning: 'bg-yellow-400',
      critical: 'bg-red-400'
    };
    return <div className={cn('w-2 h-2 rounded-full', statusColors[status])} />;
  };

  const getTrendColor = () => {
    if (!trend?.direction) return 'text-gray-400';
    return trend.direction === 'up' ? 'text-green-400' : 
           trend.direction === 'down' ? 'text-red-400' : 'text-gray-400';
  };

  const getSizeClasses = () => {
    const sizes = {
      sm: {
        title: 'text-xs',
        value: 'text-lg',
        subtitle: 'text-xs',
        icon: 'h-3 w-3',
        padding: 'p-3'
      },
      md: {
        title: 'text-sm',
        value: 'text-2xl',
        subtitle: 'text-xs',
        icon: 'h-4 w-4',
        padding: 'p-4'
      },
      lg: {
        title: 'text-base',
        value: 'text-3xl',
        subtitle: 'text-sm',
        icon: 'h-5 w-5',
        padding: 'p-6'
      }
    };
    return sizes[size];
  };

  const colors = getThemeColors();
  const sizeClasses = getSizeClasses();

  return (
    <Card className={cn(
      'bg-black/40 backdrop-blur-lg transition-all duration-300 hover:shadow-lg',
      colors.border,
      colors.bg,
      className
    )}>
      <CardHeader className={cn(
        'flex flex-row items-center justify-between space-y-0 pb-2',
        size === 'sm' ? 'p-3 pb-1' : size === 'lg' ? 'p-6 pb-3' : 'p-4 pb-2'
      )}>
        <CardTitle className={cn('font-medium text-white', sizeClasses.title)}>
          {title}
        </CardTitle>
        <div className="flex items-center gap-2">
          {getStatusIndicator()}
          {Icon && <Icon className={cn(colors.icon, sizeClasses.icon)} />}
        </div>
      </CardHeader>
      <CardContent className={cn(
        size === 'sm' ? 'p-3 pt-0' : size === 'lg' ? 'p-6 pt-0' : 'p-4 pt-0'
      )}>
        <div className="space-y-2">
          {/* Main value */}
          <div className={cn(
            'font-bold',
            status ? getStatusColor() : colors.text,
            sizeClasses.value
          )}>
            {value}
          </div>

          {/* Subtitle and trend */}
          <div className="flex items-center justify-between">
            {subtitle && (
              <p className={cn('text-gray-400', sizeClasses.subtitle)}>
                {subtitle}
              </p>
            )}
            {trend?.percentage && (
              <Badge variant="outline" className={cn(
                'text-xs border-gray-600',
                getTrendColor()
              )}>
                {trend.percentage}
              </Badge>
            )}
          </div>

          {/* Progress bar */}
          {progress && (
            <ProgressMetric
              label={progress.label || ''}
              value={progress.value}
              maxValue={progress.max}
              theme={theme}
              className="mt-2"
            />
          )}

          {/* Sparkline chart */}
          {showSparkline && trend?.data && trend.data.length > 0 && (
            <div className="mt-3">
              <SparklineChart
                data={trend.data}
                width={size === 'sm' ? 120 : size === 'lg' ? 200 : 160}
                height={size === 'sm' ? 30 : size === 'lg' ? 50 : 40}
                theme={theme}
                showArea={true}
                animate={true}
              />
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

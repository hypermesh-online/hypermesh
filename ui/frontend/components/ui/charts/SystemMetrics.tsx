// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { LucideIcon, TrendingUp, TrendingDown, Minus } from 'lucide-react';
import { GaugeChart } from './GaugeChart';
import { SparklineChart } from './SparklineChart';

export interface MetricValue {
  current: number;
  previous?: number;
  target?: number;
  threshold?: {
    warning: number;
    critical: number;
  };
  unit?: string;
  format?: 'number' | 'percentage' | 'bytes' | 'duration' | 'currency';
}

export interface SystemMetric {
  id: string;
  name: string;
  icon?: LucideIcon;
  value: MetricValue;
  trend?: {
    direction: 'up' | 'down' | 'stable';
    percentage: number;
    period: string;
  };
  history?: Array<{ timestamp: string; value: number }>;
  category?: 'performance' | 'reliability' | 'security' | 'capacity';
  priority?: 'high' | 'medium' | 'low';
  description?: string;
}

interface SystemMetricsProps {
  metrics: SystemMetric[];
  layout?: 'grid' | 'list' | 'compact';
  showTrends?: boolean;
  showHistory?: boolean;
  showGauges?: boolean;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  groupByCategory?: boolean;
  className?: string;
}

export function SystemMetrics({
  metrics,
  layout = 'grid',
  showTrends = true,
  showHistory = true,
  showGauges = false,
  theme = 'cyan',
  groupByCategory = false,
  className
}: SystemMetricsProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        primary: 'text-cyan-400',
        bg: 'bg-cyan-500/10 border-cyan-500/30',
        accent: 'bg-cyan-500/5'
      },
      green: {
        primary: 'text-green-400',
        bg: 'bg-green-500/10 border-green-500/30',
        accent: 'bg-green-500/5'
      },
      purple: {
        primary: 'text-purple-400',
        bg: 'bg-purple-500/10 border-purple-500/30',
        accent: 'bg-purple-500/5'
      },
      red: {
        primary: 'text-red-400',
        bg: 'bg-red-500/10 border-red-500/30',
        accent: 'bg-red-500/5'
      },
      yellow: {
        primary: 'text-yellow-400',
        bg: 'bg-yellow-500/10 border-yellow-500/30',
        accent: 'bg-yellow-500/5'
      }
    };
    return themes[theme];
  };

  const formatValue = (value: number, format?: string, unit?: string) => {
    let formatted: string;

    switch (format) {
      case 'percentage':
        formatted = `${value.toFixed(1)}%`;
        break;
      case 'bytes':
        if (value >= 1024 ** 3) {
          formatted = `${(value / (1024 ** 3)).toFixed(1)} GB`;
        } else if (value >= 1024 ** 2) {
          formatted = `${(value / (1024 ** 2)).toFixed(1)} MB`;
        } else if (value >= 1024) {
          formatted = `${(value / 1024).toFixed(1)} KB`;
        } else {
          formatted = `${value} B`;
        }
        break;
      case 'duration':
        if (value >= 3600000) {
          formatted = `${(value / 3600000).toFixed(1)}h`;
        } else if (value >= 60000) {
          formatted = `${(value / 60000).toFixed(1)}m`;
        } else if (value >= 1000) {
          formatted = `${(value / 1000).toFixed(1)}s`;
        } else {
          formatted = `${value}ms`;
        }
        break;
      case 'currency':
        formatted = `$${value.toLocaleString()}`;
        break;
      default:
        formatted = value >= 1000 ? `${(value / 1000).toFixed(1)}k` : value.toFixed(1);
        break;
    }

    return unit && format !== 'bytes' && format !== 'duration' && format !== 'currency' 
      ? `${formatted}${unit}` 
      : formatted;
  };

  const getStatusColor = (metric: SystemMetric) => {
    const { value } = metric;
    
    if (value.threshold) {
      if (value.current >= value.threshold.critical) return 'text-red-400';
      if (value.current >= value.threshold.warning) return 'text-yellow-400';
    }
    
    if (value.target) {
      const percentage = (value.current / value.target) * 100;
      if (percentage >= 95) return 'text-green-400';
      if (percentage >= 80) return 'text-yellow-400';
      if (percentage < 50) return 'text-red-400';
    }
    
    return getThemeColors().primary;
  };

  const getStatusBadge = (metric: SystemMetric) => {
    const { value } = metric;
    
    if (value.threshold) {
      if (value.current >= value.threshold.critical) {
        return <Badge className="bg-red-500/20 text-red-400 border-red-500/30">Critical</Badge>;
      }
      if (value.current >= value.threshold.warning) {
        return <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">Warning</Badge>;
      }
    }
    
    return <Badge className="bg-green-500/20 text-green-400 border-green-500/30">Normal</Badge>;
  };

  const getTrendIcon = (trend?: SystemMetric['trend']) => {
    if (!trend) return <Minus className="h-3 w-3 text-gray-400" />;
    
    switch (trend.direction) {
      case 'up':
        return <TrendingUp className="h-3 w-3 text-green-400" />;
      case 'down':
        return <TrendingDown className="h-3 w-3 text-red-400" />;
      default:
        return <Minus className="h-3 w-3 text-gray-400" />;
    }
  };

  const renderMetric = (metric: SystemMetric, index: number) => {
    const colors = getThemeColors();
    const Icon = metric.icon;

    if (layout === 'compact') {
      return (
        <div key={metric.id} className={cn(
          'flex items-center justify-between p-3 rounded-lg border',
          colors.bg
        )}>
          <div className="flex items-center gap-3">
            {Icon && <Icon className={cn('h-4 w-4', colors.primary)} />}
            <div>
              <p className="font-medium text-white text-sm">{metric.name}</p>
              <p className="text-xs text-gray-400">{metric.description}</p>
            </div>
          </div>
          <div className="text-right">
            <p className={cn('font-bold text-lg', getStatusColor(metric))}>
              {formatValue(metric.value.current, metric.value.format, metric.value.unit)}
            </p>
            {showTrends && metric.trend && (
              <div className="flex items-center gap-1 justify-end">
                {getTrendIcon(metric.trend)}
                <span className="text-xs text-gray-400">
                  {metric.trend.percentage}% {metric.trend.period}
                </span>
              </div>
            )}
          </div>
        </div>
      );
    }

    if (layout === 'list') {
      return (
        <Card key={metric.id} className={cn('bg-black/40 backdrop-blur-lg', colors.bg)}>
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3 flex-1">
                {Icon && <Icon className={cn('h-5 w-5', colors.primary)} />}
                <div className="flex-1">
                  <h4 className="font-medium text-white">{metric.name}</h4>
                  {metric.description && (
                    <p className="text-sm text-gray-400 mt-1">{metric.description}</p>
                  )}
                </div>
              </div>
              
              <div className="flex items-center gap-4">
                {showHistory && metric.history && (
                  <div className="w-24">
                    <SparklineChart
                      data={metric.history}
                      width={96}
                      height={24}
                      theme={theme}
                      showArea={true}
                    />
                  </div>
                )}
                
                <div className="text-right">
                  <p className={cn('font-bold text-xl', getStatusColor(metric))}>
                    {formatValue(metric.value.current, metric.value.format, metric.value.unit)}
                  </p>
                  {metric.value.target && (
                    <p className="text-sm text-gray-400">
                      / {formatValue(metric.value.target, metric.value.format, metric.value.unit)}
                    </p>
                  )}
                  {showTrends && metric.trend && (
                    <div className="flex items-center gap-1 justify-end mt-1">
                      {getTrendIcon(metric.trend)}
                      <span className="text-xs text-gray-400">
                        {metric.trend.percentage}%
                      </span>
                    </div>
                  )}
                </div>
                
                {getStatusBadge(metric)}
              </div>
            </div>
            
            {/* Progress bar for metrics with targets or thresholds */}
            {(metric.value.target || metric.value.threshold) && (
              <div className="mt-3">
                <Progress
                  value={metric.value.target 
                    ? (metric.value.current / metric.value.target) * 100
                    : metric.value.threshold
                    ? Math.min((metric.value.current / metric.value.threshold.critical) * 100, 100)
                    : 0
                  }
                  className="h-2"
                />
              </div>
            )}
          </CardContent>
        </Card>
      );
    }

    // Grid layout (default)
    return (
      <Card key={metric.id} className={cn('bg-black/40 backdrop-blur-lg', colors.bg)}>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium text-white flex items-center gap-2">
              {Icon && <Icon className={cn('h-4 w-4', colors.primary)} />}
              {metric.name}
            </CardTitle>
            {getStatusBadge(metric)}
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {/* Main value display */}
          {showGauges && metric.value.target ? (
            <GaugeChart
              value={metric.value.current}
              max={metric.value.target}
              size={120}
              theme={theme}
              showValue={true}
              unit={metric.value.unit}
            />
          ) : (
            <div>
              <div className={cn('text-2xl font-bold', getStatusColor(metric))}>
                {formatValue(metric.value.current, metric.value.format, metric.value.unit)}
              </div>
              {metric.value.target && (
                <p className="text-sm text-gray-400">
                  Target: {formatValue(metric.value.target, metric.value.format, metric.value.unit)}
                </p>
              )}
            </div>
          )}

          {/* Trend indicator */}
          {showTrends && metric.trend && (
            <div className="flex items-center gap-2 text-sm">
              {getTrendIcon(metric.trend)}
              <span className={cn(
                'font-medium',
                metric.trend.direction === 'up' ? 'text-green-400' :
                metric.trend.direction === 'down' ? 'text-red-400' : 'text-gray-400'
              )}>
                {metric.trend.percentage}%
              </span>
              <span className="text-gray-400">vs {metric.trend.period}</span>
            </div>
          )}

          {/* Historical sparkline */}
          {showHistory && metric.history && (
            <div>
              <SparklineChart
                data={metric.history}
                width={200}
                height={40}
                theme={theme}
                showArea={true}
              />
            </div>
          )}

          {/* Progress bar */}
          {metric.value.target && (
            <div className="space-y-1">
              <Progress
                value={(metric.value.current / metric.value.target) * 100}
                className="h-2"
              />
              <p className="text-xs text-gray-400">
                {((metric.value.current / metric.value.target) * 100).toFixed(1)}% of target
              </p>
            </div>
          )}

          {/* Description */}
          {metric.description && (
            <p className="text-xs text-gray-500">{metric.description}</p>
          )}
        </CardContent>
      </Card>
    );
  };

  const groupedMetrics = groupByCategory 
    ? metrics.reduce((groups, metric) => {
        const category = metric.category || 'other';
        if (!groups[category]) groups[category] = [];
        groups[category].push(metric);
        return groups;
      }, {} as Record<string, SystemMetric[]>)
    : { all: metrics };

  const categoryLabels = {
    performance: 'Performance',
    reliability: 'Reliability', 
    security: 'Security',
    capacity: 'Capacity',
    other: 'Other Metrics'
  };

  return (
    <div className={cn('space-y-6', className)}>
      {Object.entries(groupedMetrics).map(([category, categoryMetrics]) => (
        <div key={category}>
          {groupByCategory && (
            <h3 className="text-lg font-medium text-white mb-4">
              {categoryLabels[category as keyof typeof categoryLabels] || category}
            </h3>
          )}
          
          <div className={cn(
            layout === 'grid' ? 'grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' :
            layout === 'list' ? 'space-y-4' :
            'space-y-2'
          )}>
            {categoryMetrics.map(renderMetric)}
          </div>
        </div>
      ))}
    </div>
  );
}

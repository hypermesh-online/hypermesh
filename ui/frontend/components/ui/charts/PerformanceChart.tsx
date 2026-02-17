// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';
import { LineChart } from './LineChart';
import { AreaChart } from './AreaChart';
import { BarChart } from './BarChart';

interface PerformanceDataPoint {
  timestamp: string;
  value: number;
  label?: string;
  category?: string;
}

interface PerformanceMetric {
  name: string;
  data: PerformanceDataPoint[];
  unit?: string;
  target?: number;
  threshold?: {
    warning: number;
    critical: number;
  };
  color?: string;
}

interface PerformanceChartProps {
  metrics: PerformanceMetric[];
  type?: 'line' | 'area' | 'bar' | 'mixed';
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  timeRange?: '1h' | '6h' | '24h' | '7d' | '30d';
  showGrid?: boolean;
  showLegend?: boolean;
  showTargets?: boolean;
  showThresholds?: boolean;
  realtime?: boolean;
  className?: string;
}

export function PerformanceChart({
  metrics,
  type = 'line',
  height = 300,
  theme = 'cyan',
  timeRange = '24h',
  showGrid = true,
  showLegend = true,
  showTargets = true,
  showThresholds = true,
  realtime = false,
  className
}: PerformanceChartProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: ['#22d3ee', '#06b6d4', '#0891b2', '#0e7490', '#155e75'],
      green: ['#4ade80', '#22c55e', '#16a34a', '#15803d', '#166534'],
      purple: ['#a855f7', '#9333ea', '#7c3aed', '#6d28d9', '#5b21b6'],
      red: ['#f87171', '#ef4444', '#dc2626', '#b91c1c', '#991b1b'],
      yellow: ['#fbbf24', '#f59e0b', '#d97706', '#b45309', '#92400e']
    };
    return themes[theme];
  };

  const formatTimeLabel = (timestamp: string) => {
    const date = new Date(timestamp);
    switch (timeRange) {
      case '1h':
      case '6h':
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      case '24h':
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      case '7d':
        return date.toLocaleDateString([], { weekday: 'short' });
      case '30d':
        return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
      default:
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
  };

  const colors = getThemeColors();
  const width = 800;
  const padding = { top: 20, right: showLegend ? 150 : 30, bottom: 60, left: 80 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  // Combine all data points for scaling
  const allValues = metrics.flatMap(m => m.data.map(d => d.value));
  const maxValue = Math.max(...allValues, ...metrics.filter(m => m.target).map(m => m.target!));
  const minValue = Math.min(...allValues, 0);
  const valueRange = maxValue - minValue || 1;

  // Get time range for x-axis
  const allTimestamps = metrics.flatMap(m => m.data.map(d => new Date(d.timestamp).getTime()));
  const minTime = Math.min(...allTimestamps);
  const maxTime = Math.max(...allTimestamps);
  const timeRange_ms = maxTime - minTime || 1;

  const renderChart = () => {
    switch (type) {
      case 'area':
        if (metrics.length === 1) {
          return (
            <AreaChart
              data={metrics[0].data.map(d => ({ 
                label: formatTimeLabel(d.timestamp), 
                value: d.value 
              }))}
              height={height}
              theme={theme}
              showGrid={showGrid}
              className="w-full"
            />
          );
        }
        // Fall through to line for multiple metrics
      case 'line':
        return (
          <div className="relative">
            <svg width={width} height={height} className="w-full">
              {/* Grid */}
              {showGrid && (
                <g className="opacity-20">
                  {Array.from({ length: 6 }, (_, i) => (
                    <line
                      key={`h-grid-${i}`}
                      x1={padding.left}
                      y1={padding.top + (i * chartHeight) / 5}
                      x2={padding.left + chartWidth}
                      y2={padding.top + (i * chartHeight) / 5}
                      stroke={colors[0]}
                      strokeWidth={1}
                    />
                  ))}
                  {Array.from({ length: 8 }, (_, i) => (
                    <line
                      key={`v-grid-${i}`}
                      x1={padding.left + (i * chartWidth) / 7}
                      y1={padding.top}
                      x2={padding.left + (i * chartWidth) / 7}
                      y2={padding.top + chartHeight}
                      stroke={colors[0]}
                      strokeWidth={1}
                    />
                  ))}
                </g>
              )}

              {/* Threshold lines */}
              {showThresholds && metrics.map((metric, metricIndex) => 
                metric.threshold && (
                  <g key={`threshold-${metricIndex}`}>
                    {/* Warning threshold */}
                    <line
                      x1={padding.left}
                      y1={padding.top + (1 - (metric.threshold.warning - minValue) / valueRange) * chartHeight}
                      x2={padding.left + chartWidth}
                      y2={padding.top + (1 - (metric.threshold.warning - minValue) / valueRange) * chartHeight}
                      stroke="#fbbf24"
                      strokeWidth={1}
                      strokeDasharray="5,5"
                      opacity={0.6}
                    />
                    {/* Critical threshold */}
                    <line
                      x1={padding.left}
                      y1={padding.top + (1 - (metric.threshold.critical - minValue) / valueRange) * chartHeight}
                      x2={padding.left + chartWidth}
                      y2={padding.top + (1 - (metric.threshold.critical - minValue) / valueRange) * chartHeight}
                      stroke="#ef4444"
                      strokeWidth={1}
                      strokeDasharray="5,5"
                      opacity={0.6}
                    />
                  </g>
                )
              )}

              {/* Target lines */}
              {showTargets && metrics.map((metric, metricIndex) => 
                metric.target && (
                  <line
                    key={`target-${metricIndex}`}
                    x1={padding.left}
                    y1={padding.top + (1 - (metric.target - minValue) / valueRange) * chartHeight}
                    x2={padding.left + chartWidth}
                    y2={padding.top + (1 - (metric.target - minValue) / valueRange) * chartHeight}
                    stroke={metric.color || colors[metricIndex % colors.length]}
                    strokeWidth={2}
                    strokeDasharray="10,5"
                    opacity={0.8}
                  />
                )
              )}

              {/* Data lines */}
              {metrics.map((metric, metricIndex) => {
                const points = metric.data.map((point, index) => {
                  const timestamp = new Date(point.timestamp).getTime();
                  const x = padding.left + ((timestamp - minTime) / timeRange_ms) * chartWidth;
                  const y = padding.top + (1 - (point.value - minValue) / valueRange) * chartHeight;
                  return { x, y, value: point.value };
                });

                const pathData = points.reduce((path, point, index) => {
                  if (index === 0) {
                    return `M ${point.x} ${point.y}`;
                  }
                  const prevPoint = points[index - 1];
                  const controlX1 = prevPoint.x + (point.x - prevPoint.x) * 0.4;
                  const controlY1 = prevPoint.y;
                  const controlX2 = point.x - (point.x - prevPoint.x) * 0.4;
                  const controlY2 = point.y;
                  return `${path} C ${controlX1} ${controlY1}, ${controlX2} ${controlY2}, ${point.x} ${point.y}`;
                }, '');

                const color = metric.color || colors[metricIndex % colors.length];

                return (
                  <g key={metric.name}>
                    <path
                      d={pathData}
                      fill="none"
                      stroke={color}
                      strokeWidth={2}
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className="transition-all duration-300"
                    />
                    {points.map((point, pointIndex) => (
                      <circle
                        key={pointIndex}
                        cx={point.x}
                        cy={point.y}
                        r={3}
                        fill={color}
                        className="opacity-80 hover:opacity-100 transition-opacity"
                      />
                    ))}
                  </g>
                );
              })}

              {/* Y-axis labels */}
              {Array.from({ length: 6 }, (_, i) => {
                const value = maxValue - (i / 5) * valueRange;
                const y = padding.top + (i / 5) * chartHeight;
                return (
                  <text
                    key={`y-label-${i}`}
                    x={padding.left - 10}
                    y={y + 5}
                    textAnchor="end"
                    className="text-xs fill-gray-400"
                  >
                    {value.toFixed(1)}
                  </text>
                );
              })}

              {/* X-axis labels */}
              {Array.from({ length: 8 }, (_, i) => {
                const timestamp = minTime + (i / 7) * timeRange_ms;
                const x = padding.left + (i / 7) * chartWidth;
                const label = formatTimeLabel(new Date(timestamp).toISOString());
                return (
                  <text
                    key={`x-label-${i}`}
                    x={x}
                    y={height - 10}
                    textAnchor="middle"
                    className="text-xs fill-gray-400"
                  >
                    {label}
                  </text>
                );
              })}
            </svg>
          </div>
        );

      case 'bar':
        // Aggregate data by time period for bar chart
        const aggregatedData = metrics[0]?.data.map(d => ({
          label: formatTimeLabel(d.timestamp),
          value: d.value
        })) || [];

        return (
          <BarChart
            data={aggregatedData}
            height={height}
            theme={theme}
            showGrid={showGrid}
            showValues={false}
            className="w-full"
          />
        );

      default:
        return null;
    }
  };

  const getStatusColor = (value: number, metric: PerformanceMetric) => {
    if (!metric.threshold) return colors[0];
    
    if (value >= metric.threshold.critical) return '#ef4444';
    if (value >= metric.threshold.warning) return '#fbbf24';
    return colors[0];
  };

  const formatValue = (value: number, unit?: string) => {
    const formatted = value >= 1000 ? `${(value / 1000).toFixed(1)}k` : value.toFixed(1);
    return unit ? `${formatted}${unit}` : formatted;
  };

  return (
    <div className={cn('relative', className)}>
      {/* Chart */}
      {renderChart()}

      {/* Legend */}
      {showLegend && metrics.length > 1 && (
        <div className="absolute top-4 right-4 bg-black/80 border border-gray-600 rounded-lg p-3 text-xs max-w-xs">
          <h5 className="font-medium text-white mb-2">Metrics</h5>
          <div className="space-y-2">
            {metrics.map((metric, index) => {
              const color = metric.color || colors[index % colors.length];
              const latestValue = metric.data[metric.data.length - 1]?.value || 0;
              
              return (
                <div key={metric.name} className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <div 
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: color }}
                    />
                    <span className="text-gray-300">{metric.name}</span>
                  </div>
                  <div className="text-right">
                    <span 
                      className="font-medium"
                      style={{ color: getStatusColor(latestValue, metric) }}
                    >
                      {formatValue(latestValue, metric.unit)}
                    </span>
                    {metric.target && (
                      <div className="text-gray-500">
                        / {formatValue(metric.target, metric.unit)}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {/* Status indicators */}
          {showThresholds && (
            <div className="mt-3 pt-2 border-t border-gray-700">
              <div className="flex items-center gap-2">
                <div className="w-2 h-0.5 bg-yellow-400 rounded"></div>
                <span className="text-gray-400">Warning</span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <div className="w-2 h-0.5 bg-red-400 rounded"></div>
                <span className="text-gray-400">Critical</span>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Real-time indicator */}
      {realtime && (
        <div className="absolute top-4 left-4 flex items-center gap-2 bg-black/80 border border-gray-600 rounded-lg px-3 py-1 text-xs">
          <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
          <span className="text-gray-300">Live</span>
        </div>
      )}
    </div>
  );
}

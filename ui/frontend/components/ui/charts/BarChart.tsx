// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface DataPoint {
  label: string;
  value: number;
  category?: string;
}

interface BarChartProps {
  data: DataPoint[];
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  orientation?: 'vertical' | 'horizontal';
  showValues?: boolean;
  showGrid?: boolean;
  animate?: boolean;
  className?: string;
}

export function BarChart({
  data,
  height = 300,
  theme = 'cyan',
  orientation = 'vertical',
  showValues = true,
  showGrid = true,
  animate = true,
  className
}: BarChartProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        fill: '#22d3ee',
        fillHover: '#06b6d4',
        grid: 'rgba(34, 211, 238, 0.1)'
      },
      green: {
        fill: '#4ade80',
        fillHover: '#22c55e',
        grid: 'rgba(74, 222, 128, 0.1)'
      },
      purple: {
        fill: '#a855f7',
        fillHover: '#9333ea',
        grid: 'rgba(168, 85, 247, 0.1)'
      },
      red: {
        fill: '#f87171',
        fillHover: '#ef4444',
        grid: 'rgba(248, 113, 113, 0.1)'
      },
      yellow: {
        fill: '#fbbf24',
        fillHover: '#f59e0b',
        grid: 'rgba(251, 191, 36, 0.1)'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();
  const width = 800;
  const padding = { top: 20, right: 30, bottom: 60, left: 60 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  const maxValue = Math.max(...data.map(d => d.value));
  const barThickness = orientation === 'vertical' 
    ? (chartWidth / data.length) * 0.7 
    : (chartHeight / data.length) * 0.7;

  const gridLines = [];
  if (showGrid) {
    if (orientation === 'vertical') {
      for (let i = 0; i <= 5; i++) {
        const y = padding.top + (i / 5) * chartHeight;
        gridLines.push(
          <line
            key={`grid-${i}`}
            x1={padding.left}
            y1={y}
            x2={padding.left + chartWidth}
            y2={y}
            stroke={colors.grid}
            strokeWidth={1}
          />
        );
      }
    } else {
      for (let i = 0; i <= 5; i++) {
        const x = padding.left + (i / 5) * chartWidth;
        gridLines.push(
          <line
            key={`grid-${i}`}
            x1={x}
            y1={padding.top}
            x2={x}
            y2={padding.top + chartHeight}
            stroke={colors.grid}
            strokeWidth={1}
          />
        );
      }
    }
  }

  const bars = data.map((item, index) => {
    if (orientation === 'vertical') {
      const barHeight = (item.value / maxValue) * chartHeight;
      const x = padding.left + (index / data.length) * chartWidth + (chartWidth / data.length - barThickness) / 2;
      const y = padding.top + chartHeight - barHeight;

      return (
        <g key={index}>
          <rect
            x={x}
            y={y}
            width={barThickness}
            height={barHeight}
            fill={colors.fill}
            rx={4}
            className={cn(
              'transition-all duration-300',
              animate && 'hover:fill-opacity-80'
            )}
            style={{ 
              transformOrigin: `${x + barThickness/2}px ${padding.top + chartHeight}px`,
              animation: animate ? `scaleY 0.8s ease-out ${index * 0.1}s both` : undefined
            }}
          />
          {showValues && (
            <text
              x={x + barThickness / 2}
              y={y - 5}
              textAnchor="middle"
              className="text-xs fill-white"
            >
              {item.value}
            </text>
          )}
          <text
            x={x + barThickness / 2}
            y={height - 30}
            textAnchor="middle"
            className="text-xs fill-gray-400"
          >
            {item.label}
          </text>
        </g>
      );
    } else {
      const barWidth = (item.value / maxValue) * chartWidth;
      const x = padding.left;
      const y = padding.top + (index / data.length) * chartHeight + (chartHeight / data.length - barThickness) / 2;

      return (
        <g key={index}>
          <rect
            x={x}
            y={y}
            width={barWidth}
            height={barThickness}
            fill={colors.fill}
            rx={4}
            className={cn(
              'transition-all duration-300',
              animate && 'hover:fill-opacity-80'
            )}
            style={{
              transformOrigin: `${padding.left}px ${y + barThickness/2}px`,
              animation: animate ? `scaleX 0.8s ease-out ${index * 0.1}s both` : undefined
            }}
          />
          {showValues && (
            <text
              x={x + barWidth + 10}
              y={y + barThickness / 2 + 4}
              className="text-xs fill-white"
            >
              {item.value}
            </text>
          )}
          <text
            x={padding.left - 10}
            y={y + barThickness / 2 + 4}
            textAnchor="end"
            className="text-xs fill-gray-400"
          >
            {item.label}
          </text>
        </g>
      );
    }
  });

  // Axis labels
  const axisLabels = [];
  if (orientation === 'vertical') {
    for (let i = 0; i <= 5; i++) {
      const value = (maxValue / 5) * i;
      const y = padding.top + chartHeight - (i / 5) * chartHeight;
      axisLabels.push(
        <text
          key={`y-axis-${i}`}
          x={padding.left - 10}
          y={y + 4}
          textAnchor="end"
          className="text-xs fill-gray-400"
        >
          {value.toFixed(1)}
        </text>
      );
    }
  } else {
    for (let i = 0; i <= 5; i++) {
      const value = (maxValue / 5) * i;
      const x = padding.left + (i / 5) * chartWidth;
      axisLabels.push(
        <text
          key={`x-axis-${i}`}
          x={x}
          y={height - 10}
          textAnchor="middle"
          className="text-xs fill-gray-400"
        >
          {value.toFixed(1)}
        </text>
      );
    }
  }

  return (
    <div className={cn('relative', className)}>
      <svg width={width} height={height} className="w-full">
        {gridLines}
        {axisLabels}
        {bars}
      </svg>
      <style>{`
        @keyframes scaleY {
          from { transform: scaleY(0); }
          to { transform: scaleY(1); }
        }
        @keyframes scaleX {
          from { transform: scaleX(0); }
          to { transform: scaleX(1); }
        }
      `}</style>
    </div>
  );
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface DataPoint {
  label: string;
  value: number;
  timestamp?: string;
}

interface LineChartProps {
  data: DataPoint[];
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  showPoints?: boolean;
  showGrid?: boolean;
  smooth?: boolean;
  animate?: boolean;
  className?: string;
}

export function LineChart({
  data,
  height = 200,
  theme = 'cyan',
  showPoints = true,
  showGrid = true,
  smooth = true,
  animate = true,
  className
}: LineChartProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        stroke: '#22d3ee',
        point: '#06b6d4',
        grid: 'rgba(34, 211, 238, 0.1)'
      },
      green: {
        stroke: '#4ade80',
        point: '#22c55e',
        grid: 'rgba(74, 222, 128, 0.1)'
      },
      purple: {
        stroke: '#a855f7',
        point: '#9333ea',
        grid: 'rgba(168, 85, 247, 0.1)'
      },
      red: {
        stroke: '#f87171',
        point: '#ef4444',
        grid: 'rgba(248, 113, 113, 0.1)'
      },
      yellow: {
        stroke: '#fbbf24',
        point: '#f59e0b',
        grid: 'rgba(251, 191, 36, 0.1)'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();
  const width = 800;
  const padding = { top: 20, right: 30, bottom: 40, left: 40 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  const maxValue = Math.max(...data.map(d => d.value));
  const minValue = Math.min(...data.map(d => d.value));
  const valueRange = maxValue - minValue || 1;

  const points = data.map((point, index) => {
    const x = (index / (data.length - 1)) * chartWidth + padding.left;
    const y = padding.top + (1 - (point.value - minValue) / valueRange) * chartHeight;
    return { x, y, ...point };
  });

  const pathData = points.reduce((path, point, index) => {
    if (index === 0) {
      return `M ${point.x} ${point.y}`;
    }
    if (smooth) {
      const prevPoint = points[index - 1];
      const controlX1 = prevPoint.x + (point.x - prevPoint.x) * 0.4;
      const controlY1 = prevPoint.y;
      const controlX2 = point.x - (point.x - prevPoint.x) * 0.4;
      const controlY2 = point.y;
      return `${path} C ${controlX1} ${controlY1}, ${controlX2} ${controlY2}, ${point.x} ${point.y}`;
    } else {
      return `${path} L ${point.x} ${point.y}`;
    }
  }, '');

  const gridLines = [];
  if (showGrid) {
    for (let i = 0; i <= 4; i++) {
      const y = padding.top + (i / 4) * chartHeight;
      gridLines.push(
        <line
          key={`horizontal-${i}`}
          x1={padding.left}
          y1={y}
          x2={padding.left + chartWidth}
          y2={y}
          stroke={colors.grid}
          strokeWidth={1}
        />
      );
    }
    for (let i = 0; i <= 6; i++) {
      const x = padding.left + (i / 6) * chartWidth;
      gridLines.push(
        <line
          key={`vertical-${i}`}
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

  const yAxisLabels = [];
  for (let i = 0; i <= 4; i++) {
    const value = maxValue - (i / 4) * valueRange;
    const y = padding.top + (i / 4) * chartHeight;
    yAxisLabels.push(
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
  }

  return (
    <div className={cn('relative', className)}>
      <svg width={width} height={height} className="w-full">
        {gridLines}
        {yAxisLabels}

        <path
          d={pathData}
          fill="none"
          stroke={colors.stroke}
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          className={animate ? 'animate-pulse' : ''}
          style={{
            strokeDasharray: animate ? '1000' : undefined,
            strokeDashoffset: animate ? '1000' : undefined,
            animation: animate ? 'drawLine 2s ease-out forwards' : undefined
          }}
        />

        {showPoints && points.map((point, index) => (
          <circle
            key={index}
            cx={point.x}
            cy={point.y}
            r={4}
            fill={colors.point}
            className="opacity-80 hover:opacity-100 transition-opacity cursor-pointer"
            style={{
              animation: animate ? `appearPoint 0.3s ease-out ${index * 0.1 + 1}s both` : undefined
            }}
          />
        ))}

        {/* X-axis labels */}
        {data.map((point, index) => {
          if (index % Math.ceil(data.length / 6) === 0) {
            const x = padding.left + (index / (data.length - 1)) * chartWidth;
            return (
              <text
                key={`x-label-${index}`}
                x={x}
                y={height - 10}
                textAnchor="middle"
                className="text-xs fill-gray-400"
              >
                {point.label}
              </text>
            );
          }
          return null;
        })}
      </svg>
      <style>{`
        @keyframes drawLine {
          to { stroke-dashoffset: 0; }
        }
        @keyframes appearPoint {
          from { opacity: 0; transform: scale(0); }
          to { opacity: 0.8; transform: scale(1); }
        }
      `}</style>
    </div>
  );
}

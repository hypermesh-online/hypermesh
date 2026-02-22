// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface DataPoint {
  value: number;
  timestamp?: string;
}

interface SparklineChartProps {
  data: DataPoint[];
  width?: number;
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  showArea?: boolean;
  showLastPoint?: boolean;
  animate?: boolean;
  className?: string;
}

export function SparklineChart({
  data,
  width = 200,
  height = 60,
  theme = 'cyan',
  showArea = false,
  showLastPoint = true,
  animate = true,
  className
}: SparklineChartProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        stroke: '#22d3ee',
        fill: 'rgba(34, 211, 238, 0.1)',
        point: '#06b6d4'
      },
      green: {
        stroke: '#4ade80',
        fill: 'rgba(74, 222, 128, 0.1)',
        point: '#22c55e'
      },
      purple: {
        stroke: '#a855f7',
        fill: 'rgba(168, 85, 247, 0.1)',
        point: '#9333ea'
      },
      red: {
        stroke: '#f87171',
        fill: 'rgba(248, 113, 113, 0.1)',
        point: '#ef4444'
      },
      yellow: {
        stroke: '#fbbf24',
        fill: 'rgba(251, 191, 36, 0.1)',
        point: '#f59e0b'
      }
    };
    return themes[theme];
  };

  if (data.length === 0) {
    return (
      <div className={cn('flex items-center justify-center', className)} style={{ width, height }}>
        <div className="text-xs text-gray-500">No data</div>
      </div>
    );
  }

  const colors = getThemeColors();
  const padding = 4;
  const chartWidth = width - (padding * 2);
  const chartHeight = height - (padding * 2);

  const maxValue = Math.max(...data.map(d => d.value));
  const minValue = Math.min(...data.map(d => d.value));
  const valueRange = maxValue - minValue || 1;

  const points = data.map((point, index) => {
    const x = (index / (data.length - 1)) * chartWidth + padding;
    const y = padding + (1 - (point.value - minValue) / valueRange) * chartHeight;
    return { x, y, value: point.value };
  });

  const pathData = points.reduce((path, point, index) => {
    if (index === 0) {
      return `M ${point.x} ${point.y}`;
    }
    return `${path} L ${point.x} ${point.y}`;
  }, '');

  const areaPath = showArea 
    ? `${pathData} L ${points[points.length - 1].x} ${height - padding} L ${points[0].x} ${height - padding} Z`
    : '';

  const lastPoint = points[points.length - 1];

  return (
    <div className={cn('relative', className)}>
      <svg width={width} height={height} className="w-full h-full">
        {showArea && (
          <path
            d={areaPath}
            fill={colors.fill}
            className={animate ? 'animate-pulse' : ''}
          />
        )}

        <path
          d={pathData}
          fill="none"
          stroke={colors.stroke}
          strokeWidth={1.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          className="transition-all duration-300"
          style={{
            strokeDasharray: animate ? '100' : undefined,
            strokeDashoffset: animate ? '100' : undefined,
            animation: animate ? 'drawSparkline 1s ease-out forwards' : undefined
          }}
        />

        {showLastPoint && (
          <circle
            cx={lastPoint.x}
            cy={lastPoint.y}
            r={2}
            fill={colors.point}
            className="opacity-80"
            style={{
              animation: animate ? 'appearPoint 0.3s ease-out 0.8s both' : undefined
            }}
          />
        )}
      </svg>

      <style>{`
        @keyframes drawSparkline {
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

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface DataPoint {
  label: string;
  value: number;
  color?: string;
}

interface PieChartProps {
  data: DataPoint[];
  size?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  showLabels?: boolean;
  showPercentages?: boolean;
  innerRadius?: number;
  animate?: boolean;
  className?: string;
}

export function PieChart({
  data,
  size = 300,
  theme = 'cyan',
  showLabels = true,
  showPercentages = true,
  innerRadius = 0,
  animate = true,
  className
}: PieChartProps) {
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

  const colors = getThemeColors();
  const radius = (size - 20) / 2;
  const center = size / 2;
  const total = data.reduce((sum, item) => sum + item.value, 0);

  let currentAngle = -90; // Start from top
  const slices = data.map((item, index) => {
    const percentage = (item.value / total) * 100;
    const angle = (item.value / total) * 360;
    const startAngle = currentAngle;
    const endAngle = currentAngle + angle;

    const startX = center + Math.cos((startAngle * Math.PI) / 180) * radius;
    const startY = center + Math.sin((startAngle * Math.PI) / 180) * radius;
    const endX = center + Math.cos((endAngle * Math.PI) / 180) * radius;
    const endY = center + Math.sin((endAngle * Math.PI) / 180) * radius;

    const innerStartX = center + Math.cos((startAngle * Math.PI) / 180) * innerRadius;
    const innerStartY = center + Math.sin((startAngle * Math.PI) / 180) * innerRadius;
    const innerEndX = center + Math.cos((endAngle * Math.PI) / 180) * innerRadius;
    const innerEndY = center + Math.sin((endAngle * Math.PI) / 180) * innerRadius;

    const largeArc = angle > 180 ? 1 : 0;

    let pathData;
    if (innerRadius > 0) {
      // Donut chart
      pathData = [
        `M ${innerStartX} ${innerStartY}`,
        `L ${startX} ${startY}`,
        `A ${radius} ${radius} 0 ${largeArc} 1 ${endX} ${endY}`,
        `L ${innerEndX} ${innerEndY}`,
        `A ${innerRadius} ${innerRadius} 0 ${largeArc} 0 ${innerStartX} ${innerStartY}`,
        'Z'
      ].join(' ');
    } else {
      // Regular pie chart
      pathData = [
        `M ${center} ${center}`,
        `L ${startX} ${startY}`,
        `A ${radius} ${radius} 0 ${largeArc} 1 ${endX} ${endY}`,
        'Z'
      ].join(' ');
    }

    // Label position
    const labelAngle = startAngle + angle / 2;
    const labelRadius = innerRadius + (radius - innerRadius) * 0.7;
    const labelX = center + Math.cos((labelAngle * Math.PI) / 180) * labelRadius;
    const labelY = center + Math.sin((labelAngle * Math.PI) / 180) * labelRadius;

    const slice = {
      pathData,
      color: item.color || colors[index % colors.length],
      label: item.label,
      value: item.value,
      percentage: percentage.toFixed(1),
      labelX,
      labelY,
      angle
    };

    currentAngle += angle;
    return slice;
  });

  return (
    <div className={cn('flex items-center justify-center', className)}>
      <div className="relative">
        <svg width={size} height={size} className="transform -rotate-90">
          {slices.map((slice, index) => (
            <path
              key={index}
              d={slice.pathData}
              fill={slice.color}
              className={cn(
                'transition-all duration-300 hover:opacity-80 cursor-pointer',
                animate && 'hover:scale-105'
              )}
              style={{
                transformOrigin: `${center}px ${center}px`,
                animation: animate ? `growSlice 0.8s ease-out ${index * 0.1}s both` : undefined
              }}
            />
          ))}
        </svg>

        {showLabels && (
          <div className="absolute inset-0 flex items-center justify-center">
            <svg width={size} height={size}>
              {slices.map((slice, index) => (
                <g key={index}>
                  {slice.angle > 15 && ( // Only show label if slice is large enough
                    <>
                      <text
                        x={slice.labelX}
                        y={slice.labelY - 8}
                        textAnchor="middle"
                        className="text-xs font-medium fill-white"
                      >
                        {slice.label}
                      </text>
                      {showPercentages && (
                        <text
                          x={slice.labelX}
                          y={slice.labelY + 8}
                          textAnchor="middle"
                          className="text-xs fill-gray-300"
                        >
                          {slice.percentage}%
                        </text>
                      )}
                    </>
                  )}
                </g>
              ))}
            </svg>
          </div>
        )}
      </div>

      {showLabels && (
        <div className="ml-6 space-y-2">
          {data.map((item, index) => (
            <div key={index} className="flex items-center gap-2">
              <div
                className="w-3 h-3 rounded-full"
                style={{ backgroundColor: item.color || colors[index % colors.length] }}
              />
              <span className="text-sm text-white">{item.label}</span>
              <span className="text-sm text-gray-400">({item.value})</span>
            </div>
          ))}
        </div>
      )}

      <style jsx>{`
        @keyframes growSlice {
          from { transform: scale(0); }
          to { transform: scale(1); }
        }
      `}</style>
    </div>
  );
}

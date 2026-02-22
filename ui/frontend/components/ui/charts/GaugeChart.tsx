// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { cn } from '@/lib/utils';

interface GaugeChartProps {
  value: number;
  min?: number;
  max?: number;
  size?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  label?: string;
  unit?: string;
  showValue?: boolean;
  showScale?: boolean;
  animate?: boolean;
  className?: string;
}

export function GaugeChart({
  value,
  min = 0,
  max = 100,
  size = 200,
  theme = 'cyan',
  label,
  unit = '',
  showValue = true,
  showScale = true,
  animate = true,
  className
}: GaugeChartProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        primary: '#22d3ee',
        secondary: '#06b6d4',
        background: 'rgba(34, 211, 238, 0.1)'
      },
      green: {
        primary: '#4ade80',
        secondary: '#22c55e',
        background: 'rgba(74, 222, 128, 0.1)'
      },
      purple: {
        primary: '#a855f7',
        secondary: '#9333ea',
        background: 'rgba(168, 85, 247, 0.1)'
      },
      red: {
        primary: '#f87171',
        secondary: '#ef4444',
        background: 'rgba(248, 113, 113, 0.1)'
      },
      yellow: {
        primary: '#fbbf24',
        secondary: '#f59e0b',
        background: 'rgba(251, 191, 36, 0.1)'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();
  const radius = (size - 40) / 2;
  const center = size / 2;
  const strokeWidth = 8;
  
  // Gauge goes from -120 degrees to +120 degrees (240 degrees total)
  const startAngle = -120;
  const endAngle = 120;
  const totalAngle = endAngle - startAngle;
  
  const normalizedValue = Math.max(min, Math.min(max, value));
  const percentage = (normalizedValue - min) / (max - min);
  const currentAngle = startAngle + (percentage * totalAngle);
  
  const startAngleRad = (startAngle * Math.PI) / 180;
  const endAngleRad = (endAngle * Math.PI) / 180;
  const currentAngleRad = (currentAngle * Math.PI) / 180;
  
  // Background arc
  const backgroundPath = [
    `M ${center + Math.cos(startAngleRad) * radius} ${center + Math.sin(startAngleRad) * radius}`,
    `A ${radius} ${radius} 0 1 1 ${center + Math.cos(endAngleRad) * radius} ${center + Math.sin(endAngleRad) * radius}`
  ].join(' ');

  // Progress arc
  const progressPath = [
    `M ${center + Math.cos(startAngleRad) * radius} ${center + Math.sin(startAngleRad) * radius}`,
    `A ${radius} ${radius} 0 ${percentage > 0.5 ? 1 : 0} 1 ${center + Math.cos(currentAngleRad) * radius} ${center + Math.sin(currentAngleRad) * radius}`
  ].join(' ');

  // Scale marks
  const scaleMarks = [];
  if (showScale) {
    const numMarks = 5;
    for (let i = 0; i <= numMarks; i++) {
      const angle = startAngle + (i / numMarks) * totalAngle;
      const angleRad = (angle * Math.PI) / 180;
      const innerRadius = radius - 10;
      const outerRadius = radius + 5;
      
      const x1 = center + Math.cos(angleRad) * innerRadius;
      const y1 = center + Math.sin(angleRad) * innerRadius;
      const x2 = center + Math.cos(angleRad) * outerRadius;
      const y2 = center + Math.sin(angleRad) * outerRadius;
      
      scaleMarks.push(
        <line
          key={i}
          x1={x1}
          y1={y1}
          x2={x2}
          y2={y2}
          stroke={colors.background}
          strokeWidth={2}
        />
      );

      // Scale labels
      const labelRadius = radius + 20;
      const labelX = center + Math.cos(angleRad) * labelRadius;
      const labelY = center + Math.sin(angleRad) * labelRadius;
      const scaleValue = min + (i / numMarks) * (max - min);
      
      scaleMarks.push(
        <text
          key={`label-${i}`}
          x={labelX}
          y={labelY + 4}
          textAnchor="middle"
          className="text-xs fill-gray-400"
        >
          {scaleValue.toFixed(0)}
        </text>
      );
    }
  }

  // Needle
  const needleLength = radius - 20;
  const needleX = center + Math.cos(currentAngleRad) * needleLength;
  const needleY = center + Math.sin(currentAngleRad) * needleLength;

  return (
    <div className={cn('flex flex-col items-center', className)}>
      <svg width={size} height={size} className="overflow-visible">
        <defs>
          <linearGradient id={`gauge-gradient-${theme}`} x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor={colors.secondary} />
            <stop offset="100%" stopColor={colors.primary} />
          </linearGradient>
        </defs>

        {/* Background arc */}
        <path
          d={backgroundPath}
          fill="none"
          stroke={colors.background}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
        />

        {/* Progress arc */}
        <path
          d={progressPath}
          fill="none"
          stroke={`url(#gauge-gradient-${theme})`}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          className="transition-all duration-500"
          style={{
            strokeDasharray: animate ? '1000' : undefined,
            strokeDashoffset: animate ? '1000' : undefined,
            animation: animate ? 'drawGauge 1.5s ease-out forwards' : undefined
          }}
        />

        {scaleMarks}

        {/* Center dot */}
        <circle
          cx={center}
          cy={center}
          r={6}
          fill={colors.primary}
        />

        {/* Needle */}
        <line
          x1={center}
          y1={center}
          x2={needleX}
          y2={needleY}
          stroke={colors.primary}
          strokeWidth={3}
          strokeLinecap="round"
          className="transition-all duration-500"
          style={{
            transformOrigin: `${center}px ${center}px`,
            animation: animate ? `rotateNeedle 1.5s ease-out forwards` : undefined
          }}
        />
      </svg>

      <div className="text-center mt-4">
        {showValue && (
          <div className={cn('text-2xl font-bold', `text-${theme}-400`)}>
            {value.toFixed(1)}{unit}
          </div>
        )}
        {label && (
          <div className="text-sm text-gray-400 mt-1">{label}</div>
        )}
      </div>

      <style>{`
        @keyframes drawGauge {
          to { stroke-dashoffset: 0; }
        }
        @keyframes rotateNeedle {
          from { transform: rotate(${startAngle}deg); }
          to { transform: rotate(${currentAngle}deg); }
        }
      `}</style>
    </div>
  );
}

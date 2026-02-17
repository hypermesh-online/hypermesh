// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

interface AnimatedValueProps {
  value: number | string;
  duration?: number;
  suffix?: string;
  prefix?: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function AnimatedValue({
  value,
  duration = 1000,
  suffix = '',
  prefix = '',
  theme = 'cyan',
  className
}: AnimatedValueProps) {
  const [displayValue, setDisplayValue] = useState(0);
  const numericValue = typeof value === 'number' ? value : parseFloat(value.toString()) || 0;

  useEffect(() => {
    if (typeof value !== 'number') {
      setDisplayValue(numericValue);
      return;
    }

    const startTime = Date.now();
    const startValue = displayValue;
    const targetValue = numericValue;
    const difference = targetValue - startValue;

    const animate = () => {
      const currentTime = Date.now();
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      const easedProgress = 1 - Math.pow(1 - progress, 3); // Ease-out cubic
      const currentValue = startValue + (difference * easedProgress);
      
      setDisplayValue(currentValue);
      
      if (progress < 1) {
        requestAnimationFrame(animate);
      }
    };

    requestAnimationFrame(animate);
  }, [numericValue, duration]);

  const getThemeColor = () => {
    const colors = {
      cyan: 'text-cyan-400',
      green: 'text-green-400',
      purple: 'text-purple-400',
      red: 'text-red-400',
      yellow: 'text-yellow-400'
    };
    return colors[theme];
  };

  const formatValue = (val: number) => {
    if (typeof value === 'string' && value.includes('.')) {
      return val.toFixed(value.split('.')[1].length);
    }
    return Math.round(val).toString();
  };

  return (
    <span className={cn('font-bold transition-all duration-300', getThemeColor(), className)}>
      {prefix}{formatValue(displayValue)}{suffix}
    </span>
  );
}

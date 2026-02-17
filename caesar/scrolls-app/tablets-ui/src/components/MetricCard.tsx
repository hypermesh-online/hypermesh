// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { TrendingUp, TrendingDown } from 'lucide-react';
import { MetricData } from '../types';

interface MetricCardProps {
  metric: MetricData;
}

const MetricCard: React.FC<MetricCardProps> = ({ metric }) => {
  const hasChange = typeof metric.change === 'number';
  const isPositive = hasChange && metric.change! >= 0;

  return (
    <div className="metric-card">
      <div className="flex items-center justify-between mb-2">
        <p className="text-gray-400 text-sm">{metric.label}</p>
        {hasChange && (
          <div className={`flex items-center gap-1 text-sm ${
            isPositive ? 'text-green-500' : 'text-red-500'
          }`}>
            {isPositive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
            {Math.abs(metric.change!).toFixed(2)}%
          </div>
        )}
      </div>
      
      <div className="flex items-center gap-2">
        <p className="text-2xl font-bold">{metric.value}</p>
        {metric.color && (
          <div 
            className="w-3 h-3 rounded-full" 
            style={{ backgroundColor: metric.color }}
          />
        )}
      </div>
    </div>
  );
};

export default MetricCard;
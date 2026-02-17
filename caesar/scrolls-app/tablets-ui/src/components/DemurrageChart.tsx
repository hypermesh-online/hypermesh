// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import { DemurrageData } from '../types';

const DemurrageChart: React.FC = () => {
  const [demurrageData, setDemurrageData] = useState<DemurrageData[]>([]);
  const [timeframe, setTimeframe] = useState('1M');

  useEffect(() => {
    // Generate mock demurrage data
    const generateDemurrageData = (): DemurrageData[] => {
      const data: DemurrageData[] = [];
      const now = Date.now();
      const oneDay = 24 * 60 * 60 * 1000;
      
      for (let i = 30; i >= 0; i--) {
        const timestamp = now - (i * oneDay);
        const daysPassed = 30 - i;
        
        // Demurrage calculation: 2% annual rate
        const annualRate = 0.02;
        const totalBurned = daysPassed * 1000 * annualRate / 365; // Daily burn
        const demurrageMultiplier = Math.exp(-(annualRate * daysPassed) / 365);
        const effectiveSupply = 1000000 * demurrageMultiplier;
        const burnRate = annualRate / 365; // Daily rate
        
        data.push({
          timestamp,
          totalBurned,
          burnRate: burnRate * 100, // Convert to percentage
          effectiveSupply,
          demurrageMultiplier,
        });
      }
      
      return data;
    };

    setDemurrageData(generateDemurrageData());
  }, [timeframe]);

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    });
  };

  const formatNumber = (value: number) => {
    if (value >= 1000000) return `${(value / 1000000).toFixed(1)}M`;
    if (value >= 1000) return `${(value / 1000).toFixed(1)}K`;
    return value.toFixed(2);
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-xl font-semibold mb-1">Demurrage Analytics</h3>
          <p className="text-gray-400 text-sm">
            Real-time demurrage tracking and effective supply monitoring
          </p>
        </div>
        
        <div className="flex bg-caesar-gray rounded-lg p-1">
          {['1W', '1M', '3M', '1Y'].map((period) => (
            <button
              key={period}
              onClick={() => setTimeframe(period)}
              className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
                timeframe === period
                  ? 'bg-tablets-purple text-white'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {period}
            </button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Effective Supply Chart */}
        <div>
          <h4 className="text-lg font-medium mb-3 text-tablets-cyan">Effective Supply</h4>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={demurrageData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis
                  dataKey="timestamp"
                  tickFormatter={formatDate}
                  stroke="#9CA3AF"
                  fontSize={12}
                />
                <YAxis
                  tickFormatter={formatNumber}
                  stroke="#9CA3AF"
                  fontSize={12}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '8px',
                  }}
                  labelFormatter={(value) => formatDate(value as number)}
                  formatter={(value) => [formatNumber(value as number), 'Effective Supply']}
                />
                <Area
                  type="monotone"
                  dataKey="effectiveSupply"
                  stroke="#06B6D4"
                  fill="#06B6D4"
                  fillOpacity={0.2}
                  strokeWidth={2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Total Burned Chart */}
        <div>
          <h4 className="text-lg font-medium mb-3 text-tablets-emerald">Total Burned</h4>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={demurrageData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis
                  dataKey="timestamp"
                  tickFormatter={formatDate}
                  stroke="#9CA3AF"
                  fontSize={12}
                />
                <YAxis
                  tickFormatter={formatNumber}
                  stroke="#9CA3AF"
                  fontSize={12}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '8px',
                  }}
                  labelFormatter={(value) => formatDate(value as number)}
                  formatter={(value) => [formatNumber(value as number), 'Total Burned']}
                />
                <Line
                  type="monotone"
                  dataKey="totalBurned"
                  stroke="#10B981"
                  strokeWidth={2}
                  dot={false}
                  activeDot={{ r: 4, stroke: '#10B981', strokeWidth: 2 }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Demurrage Stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6 pt-6 border-t border-gray-700">
        <div className="text-center">
          <p className="text-gray-400 text-sm">Annual Rate</p>
          <p className="text-xl font-bold text-tablets-purple">2.00%</p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">Daily Burn</p>
          <p className="text-xl font-bold text-tablets-cyan">
            {formatNumber(demurrageData[demurrageData.length - 1]?.totalBurned || 0)}
          </p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">Current Multiplier</p>
          <p className="text-xl font-bold text-tablets-emerald">
            {(demurrageData[demurrageData.length - 1]?.demurrageMultiplier || 1).toFixed(6)}
          </p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">Burn Efficiency</p>
          <p className="text-xl font-bold text-caesar-gold">98.5%</p>
        </div>
      </div>
    </div>
  );
};

export default DemurrageChart;
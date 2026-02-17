// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { TrendingUp, TrendingDown, Target, AlertTriangle, Award } from 'lucide-react';
import { type GoldEconomicData } from '../services/GoldPriceService';

interface DeviationBandChartProps {
  data: GoldEconomicData;
  height?: number;
}

const DeviationBandChart: React.FC<DeviationBandChartProps> = ({ data, height = 120 }) => {
  const { deviationBand, caesarPrice, goldPrice, marketPressure } = data;
  
  // Calculate position within the band visualization (-100% to +100%)
  const maxDeviation = 15; // Show ±15% range
  const positionPercent = Math.max(-maxDeviation, Math.min(maxDeviation, deviationBand.percentage));
  const chartPosition = ((positionPercent + maxDeviation) / (2 * maxDeviation)) * 100;
  
  const getBandColor = (status: string) => {
    switch (status) {
      case 'above': return 'bg-red-500';
      case 'below': return 'bg-blue-500';
      case 'within': return 'bg-green-500';
      default: return 'bg-gray-500';
    }
  };
  
  const getStatusIcon = () => {
    switch (deviationBand.status) {
      case 'above':
        return <TrendingUp className="text-red-400" size={16} />;
      case 'below':
        return <TrendingDown className="text-blue-400" size={16} />;
      case 'within':
        return <Target className="text-green-400" size={16} />;
      default:
        return <AlertTriangle className="text-gray-400" size={16} />;
    }
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h4 className="font-semibold text-sm">Price Position vs Gold Standard</h4>
          <p className="text-xs text-gray-400">Real-time deviation from 1g gold price</p>
        </div>
        <div className="flex items-center gap-2">
          {getStatusIcon()}
          <span className="text-sm font-medium">
            {deviationBand.percentage > 0 ? '+' : ''}{deviationBand.percentage.toFixed(2)}%
          </span>
        </div>
      </div>
      
      {/* Price comparison bar */}
      <div className="mb-4">
        <div className="flex justify-between text-xs text-gray-400 mb-2">
          <span>${goldPrice.average.toFixed(2)} (Gold)</span>
          <span>${caesarPrice.current.toFixed(2)} (Caesar)</span>
        </div>
        <div className="relative">
          <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
            {/* Optimal band (green) */}
            <div 
              className="absolute h-full bg-green-500 opacity-30"
              style={{
                left: `${((5 + maxDeviation) / (2 * maxDeviation)) * 100 - 5}%`,
                width: '10%'
              }}
            ></div>
            {/* Current position indicator */}
            <div 
              className={`absolute h-full w-1 ${getBandColor(deviationBand.status)} shadow-lg`}
              style={{ left: `${chartPosition}%` }}
            ></div>
          </div>
        </div>
        <div className="flex justify-between text-xs text-gray-500 mt-1">
          <span>-{maxDeviation}%</span>
          <span>Gold Standard</span>
          <span>+{maxDeviation}%</span>
        </div>
      </div>
      
      {/* Band status details */}
      <div className={`p-3 rounded-lg border ${
        deviationBand.status === 'above' ? 'bg-red-900/30 border-red-700' :
        deviationBand.status === 'below' ? 'bg-blue-900/30 border-blue-700' :
        'bg-green-900/30 border-green-700'
      }`}>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">
            {deviationBand.status === 'above' && 'Above Optimal Band'}
            {deviationBand.status === 'below' && 'Below Optimal Band'}
            {deviationBand.status === 'within' && 'Within Optimal Band'}
          </span>
          {deviationBand.status !== 'within' && (
            <Award className="text-yellow-400" size={16} />
          )}
        </div>
        
        <div className="grid grid-cols-2 gap-4 text-xs">
          <div>
            <span className="text-gray-400">Market Pressure:</span>
            <span className="ml-1 font-medium">
              {marketPressure.pressure > 0 ? '+' : ''}{(marketPressure.pressure * 100).toFixed(2)}%
            </span>
          </div>
          <div>
            <span className="text-gray-400">Trend:</span>
            <span className={`ml-1 font-medium ${
              marketPressure.trend === 'increasing' ? 'text-red-400' :
              marketPressure.trend === 'decreasing' ? 'text-blue-400' : 'text-green-400'
            }`}>
              {marketPressure.trend}
            </span>
          </div>
          <div>
            <span className="text-gray-400">Volatility:</span>
            <span className="ml-1 font-medium">{(marketPressure.volatility * 100).toFixed(1)}%</span>
          </div>
          <div>
            <span className="text-gray-400">Liquidity:</span>
            <span className="ml-1 font-medium">{(marketPressure.liquidityRatio * 100).toFixed(1)}%</span>
          </div>
        </div>
      </div>
      
      {/* Economic impact indicator */}
      <div className="mt-3 text-xs text-gray-400">
        <div className="flex items-center justify-between">
          <span>Next rate adjustment:</span>
          <span className="font-medium">
            {deviationBand.status === 'above' ? 'Penalty increases' :
             deviationBand.status === 'below' ? 'Rewards increase' :
             'Standard rates'}
          </span>
        </div>
        <div className="text-xs text-gray-500 mt-1">
          Gold price sources: GoldAPI (${goldPrice.goldApi.toFixed(2)}) • MetalPriceAPI (${goldPrice.metalPriceApi.toFixed(2)})
        </div>
      </div>
    </div>
  );
};

export default DeviationBandChart;
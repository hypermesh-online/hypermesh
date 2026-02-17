// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { TrendingUp, TrendingDown, Minus, AlertCircle, Award } from 'lucide-react';
import GoldPriceService, { type GoldEconomicData } from '../services/GoldPriceService';

interface GoldPriceIndicatorProps {
  compact?: boolean;
}

const GoldPriceIndicator: React.FC<GoldPriceIndicatorProps> = ({ compact = false }) => {
  const [goldData, setGoldData] = useState<GoldEconomicData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  
  useEffect(() => {
    const goldService = GoldPriceService.getInstance();
    
    // Subscribe to gold price updates
    const unsubscribe = goldService.subscribe((data: GoldEconomicData) => {
      setGoldData(data);
      setIsLoading(false);
    });
    
    return unsubscribe;
  }, []);
  
  if (isLoading || !goldData) {
    return (
      <div className="animate-pulse bg-gray-800 rounded-lg p-4">
        <div className="h-4 bg-gray-700 rounded mb-2"></div>
        <div className="h-6 bg-gray-700 rounded"></div>
      </div>
    );
  }
  
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'above': return 'text-red-400';
      case 'below': return 'text-blue-400';
      case 'within': return 'text-green-400';
      default: return 'text-gray-400';
    }
  };
  
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'above': return <TrendingUp className="text-red-400" size={16} />;
      case 'below': return <TrendingDown className="text-blue-400" size={16} />;
      case 'within': return <Minus className="text-green-400" size={16} />;
      default: return <AlertCircle className="text-gray-400" size={16} />;
    }
  };
  
  const getStatusMessage = () => {
    const { status } = goldData.deviationBand;
    const { penaltyRate, rewardRate } = goldData.incentives;
    
    switch (status) {
      case 'above':
        return `Above band: ${penaltyRate.toFixed(2)}% penalties applied to reduce circulation`;
      case 'below':
        return `Below band: ${rewardRate.toFixed(2)}% rewards provided for utility usage`;
      case 'within':
        return 'Within optimal range: standard utility incentives active';
      default:
        return 'Economic status unknown';
    }
  };
  
  if (compact) {
    return (
      <div className="flex items-center gap-2 bg-gray-800 rounded-lg px-3 py-2">
        {getStatusIcon(goldData.deviationBand.status)}
        <div className="text-sm">
          <span className="font-medium">${goldData.caesarPrice.current.toFixed(2)}</span>
          <span className="text-gray-400 ml-1">
            ({goldData.deviationBand.percentage > 0 ? '+' : ''}{goldData.deviationBand.percentage.toFixed(1)}%)
          </span>
        </div>
      </div>
    );
  }
  
  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h4 className="font-semibold text-lg">Gold-Pegged Pricing</h4>
          <p className="text-gray-400 text-sm">Caesar Token economic status</p>
        </div>
        {getStatusIcon(goldData.deviationBand.status)}
      </div>
      
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <p className="text-gray-400 text-xs mb-1">Gold Reference (1g)</p>
          <p className="text-xl font-bold text-yellow-400">
            ${goldData.goldPrice.average.toFixed(2)}
          </p>
          <p className="text-xs text-gray-500">
            API: ${goldData.goldPrice.goldApi.toFixed(2)} | ${goldData.goldPrice.metalPriceApi.toFixed(2)}
          </p>
        </div>
        <div>
          <p className="text-gray-400 text-xs mb-1">Caesar Price</p>
          <p className={`text-xl font-bold ${getStatusColor(goldData.deviationBand.status)}`}>
            ${goldData.caesarPrice.current.toFixed(2)}
          </p>
          <p className="text-xs text-gray-500">
            Target: ${goldData.caesarPrice.target.toFixed(2)}
          </p>
        </div>
      </div>
      
      {/* Deviation Band Visualization */}
      <div className="mb-4">
        <div className="flex justify-between text-xs text-gray-400 mb-1">
          <span>${goldData.deviationBand.lower.toFixed(2)}</span>
          <span>Deviation: {goldData.deviationBand.percentage > 0 ? '+' : ''}{goldData.deviationBand.percentage.toFixed(1)}%</span>
          <span>${goldData.deviationBand.upper.toFixed(2)}</span>
        </div>
        <div className="relative h-2 bg-gray-700 rounded-full">
          <div className="absolute inset-0 bg-green-500 rounded-full opacity-30"></div>
          <div 
            className={`absolute h-full w-1 rounded-full ${
              goldData.deviationBand.status === 'above' ? 'bg-red-400' :
              goldData.deviationBand.status === 'below' ? 'bg-blue-400' : 'bg-green-400'
            }`}
            style={{
              left: `${Math.max(0, Math.min(100, 50 + goldData.deviationBand.percentage * 10))}%`
            }}
          ></div>
        </div>
        <div className="text-center text-xs text-gray-400 mt-1">±5% Deviation Bands</div>
      </div>
      
      {/* Economic Status */}
      <div className={`p-3 rounded-lg border ${
        goldData.deviationBand.status === 'above' ? 'bg-red-900/30 border-red-700' :
        goldData.deviationBand.status === 'below' ? 'bg-blue-900/30 border-blue-700' :
        'bg-green-900/30 border-green-700'
      }`}>
        <div className="flex items-center gap-2 mb-2">
          {goldData.deviationBand.status !== 'within' && (
            <Award className={getStatusColor(goldData.deviationBand.status)} size={16} />
          )}
          <span className={`font-medium text-sm ${getStatusColor(goldData.deviationBand.status)}`}>
            Economic Status: {goldData.deviationBand.status === 'within' ? 'Optimal' : 
                            goldData.deviationBand.status === 'above' ? 'Above Band' : 'Below Band'}
          </span>
        </div>
        <p className="text-xs text-gray-300">
          {getStatusMessage()}
        </p>
        <p className="text-xs text-gray-400 mt-1">
          Demurrage rate: {((goldData.incentives.demurrageMultiplier - 1) * 100).toFixed(1)}% adjustment
        </p>
      </div>
    </div>
  );
};

export default GoldPriceIndicator;
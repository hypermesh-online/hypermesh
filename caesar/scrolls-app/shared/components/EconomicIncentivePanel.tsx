// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Zap, TrendingDown, TrendingUp, DollarSign, AlertCircle, Shield } from 'lucide-react';
import { type GoldEconomicData } from '../services/GoldPriceService';

interface EconomicIncentivePanelProps {
  data: GoldEconomicData;
  userBalance?: number;
  totalSupply?: number;
}

const EconomicIncentivePanel: React.FC<EconomicIncentivePanelProps> = ({ 
  data, 
  userBalance = 1000,
  totalSupply = 1000000 
}) => {
  const { incentives, deviationBand, stabilityMetrics, circuitBreakers } = data;
  
  // Calculate individual impact based on proportional cost distribution
  const calculateIndividualCost = (totalCost: number) => {
    return (totalCost * userBalance) / totalSupply * incentives.proportionalCostFactor;
  };
  
  const getIncentiveColor = (value: number, isPositive: boolean = false) => {
    if (value === 0) return 'text-gray-400';
    if (isPositive) {
      return value > 0 ? 'text-green-400' : 'text-red-400';
    } else {
      return value > 0 ? 'text-red-400' : 'text-green-400';
    }
  };
  
  const formatRate = (rate: number, suffix: string = '%') => {
    return `${rate > 0 ? '+' : ''}${rate.toFixed(3)}${suffix}`;
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h4 className="font-semibold text-lg flex items-center gap-2">
            <Zap className="text-yellow-400" size={20} />
            Economic Incentives
          </h4>
          <p className="text-gray-400 text-sm">Position-based rewards and penalties</p>
        </div>
        {circuitBreakers.emergency && (
          <div className="flex items-center gap-1 text-orange-400">
            <AlertCircle size={16} />
            <span className="text-xs">Emergency</span>
          </div>
        )}
      </div>
      
      {/* Current Incentive Rates */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className={`bg-gray-900/50 rounded-lg p-3 border ${
          deviationBand.status === 'above' ? 'border-red-700' : 'border-gray-600'
        }`}>
          <div className="flex items-center gap-2 mb-2">
            <TrendingUp className="text-red-400" size={16} />
            <span className="text-sm font-medium text-red-400">Penalties</span>
          </div>
          <div className="text-xl font-bold">
            {formatRate(incentives.penaltyRate)}
          </div>
          <p className="text-xs text-gray-400">
            Applied when above band to reduce circulation
          </p>
        </div>
        
        <div className={`bg-gray-900/50 rounded-lg p-3 border ${
          deviationBand.status === 'below' ? 'border-blue-700' : 'border-gray-600'
        }`}>
          <div className="flex items-center gap-2 mb-2">
            <TrendingDown className="text-blue-400" size={16} />
            <span className="text-sm font-medium text-blue-400">Rewards</span>
          </div>
          <div className="text-xl font-bold">
            {formatRate(incentives.rewardRate)}
          </div>
          <p className="text-xs text-gray-400">
            Given when below band for utility usage
          </p>
        </div>
      </div>
      
      {/* Demurrage Rate Adjustment */}
      <div className="bg-gray-900/50 rounded-lg p-3 mb-4">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <DollarSign size={16} className="text-yellow-400" />
            <span className="text-sm font-medium">Demurrage Rate Multiplier</span>
          </div>
          <span className={`text-lg font-bold ${getIncentiveColor(incentives.demurrageMultiplier - 1)}`}>
            {incentives.demurrageMultiplier.toFixed(3)}x
          </span>
        </div>
        <div className="text-xs text-gray-400 mb-2">
          Base rate adjusted by {formatRate((incentives.demurrageMultiplier - 1) * 100)} based on band position
        </div>
        <div className="bg-gray-700 rounded-full h-1 overflow-hidden">
          <div 
            className={`h-full transition-all duration-500 ${
              incentives.demurrageMultiplier > 1 ? 'bg-red-400' : 'bg-green-400'
            }`}
            style={{ 
              width: `${Math.min(100, Math.abs(incentives.demurrageMultiplier - 1) * 100)}%` 
            }}
          ></div>
        </div>
      </div>
      
      {/* Individual Impact Calculation */}
      <div className="bg-blue-900/20 border border-blue-700 rounded-lg p-3 mb-4">
        <div className="flex items-center gap-2 mb-2">
          <Shield className="text-blue-400" size={16} />
          <span className="text-sm font-medium text-blue-400">Your Proportional Cost</span>
        </div>
        <div className="grid grid-cols-3 gap-3 text-xs">
          <div>
            <span className="text-gray-400">Balance:</span>
            <div className="font-bold">{userBalance.toLocaleString()} CAESAR</div>
            <div className="text-gray-500">({((userBalance/totalSupply)*100).toFixed(4)}% of supply)</div>
          </div>
          <div>
            <span className="text-gray-400">Cost Factor:</span>
            <div className="font-bold">{incentives.proportionalCostFactor.toFixed(3)}x</div>
            <div className="text-gray-500">Adjusted for band position</div>
          </div>
          <div>
            <span className="text-gray-400">Daily Impact:</span>
            <div className={`font-bold ${getIncentiveColor(calculateIndividualCost(10))}`}>
              ${calculateIndividualCost(10).toFixed(4)}
            </div>
            <div className="text-gray-500">Per $10 market cost</div>
          </div>
        </div>
        <div className="mt-2 p-2 bg-blue-800/30 rounded text-xs">
          <strong>Factor 1:</strong> Costs distributed proportionally by holdings, not speculation activity.
          Band position adjusts rates to encourage utility and discourage hoarding.
        </div>
      </div>
      
      {/* Stability Metrics */}
      <div className="grid grid-cols-3 gap-3 text-xs">
        <div className="text-center">
          <div className="text-gray-400 mb-1">Price Stability</div>
          <div className={`font-bold text-sm ${
            stabilityMetrics.priceStabilityIndex > 0.8 ? 'text-green-400' :
            stabilityMetrics.priceStabilityIndex > 0.6 ? 'text-yellow-400' : 'text-red-400'
          }`}>
            {(stabilityMetrics.priceStabilityIndex * 100).toFixed(1)}%
          </div>
        </div>
        <div className="text-center">
          <div className="text-gray-400 mb-1">Liquidity Health</div>
          <div className={`font-bold text-sm ${
            stabilityMetrics.liquidityHealthIndex > 0.7 ? 'text-green-400' :
            stabilityMetrics.liquidityHealthIndex > 0.5 ? 'text-yellow-400' : 'text-red-400'
          }`}>
            {(stabilityMetrics.liquidityHealthIndex * 100).toFixed(1)}%
          </div>
        </div>
        <div className="text-center">
          <div className="text-gray-400 mb-1">Network Utility</div>
          <div className={`font-bold text-sm ${
            stabilityMetrics.networkUtilityScore > 0.6 ? 'text-green-400' :
            stabilityMetrics.networkUtilityScore > 0.4 ? 'text-yellow-400' : 'text-red-400'
          }`}>
            {(stabilityMetrics.networkUtilityScore * 100).toFixed(1)}%
          </div>
        </div>
      </div>
      
      {/* Circuit Breaker Status */}
      {(circuitBreakers.halt || circuitBreakers.emergency || circuitBreakers.rebase) && (
        <div className="mt-4 p-3 bg-orange-900/30 border border-orange-700 rounded-lg">
          <div className="flex items-center gap-2 mb-2">
            <AlertCircle className="text-orange-400" size={16} />
            <span className="text-sm font-medium text-orange-400">Circuit Breaker Status</span>
          </div>
          <div className="space-y-1 text-xs">
            {circuitBreakers.halt && (
              <div className="text-red-400">🚫 Trading halted due to low liquidity</div>
            )}
            {circuitBreakers.emergency && (
              <div className="text-orange-400">⚠️ Emergency spreads active</div>
            )}
            {circuitBreakers.rebase && (
              <div className="text-yellow-400">🔄 Rebase required for large deviation</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default EconomicIncentivePanel;
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { Droplets, TrendingUp, ExternalLink } from 'lucide-react';
import { LiquidityPool } from '../types';

const LiquidityPools: React.FC = () => {
  const [pools, setPools] = useState<LiquidityPool[]>([]);

  useEffect(() => {
    // Mock liquidity pool data
    const mockPools: LiquidityPool[] = [
      {
        address: '0x1234567890abcdef1234567890abcdef12345678',
        token0: 'CAES',
        token1: 'WETH',
        reserve0: '1250000',
        reserve1: '85.5',
        tvl: '$425,678',
        volume24h: '$85,234',
        fees24h: '$255.70',
        apy: 12.45,
      },
      {
        address: '0xabcdef1234567890abcdef1234567890abcdef12',
        token0: 'CAES',
        token1: 'USDC',
        reserve0: '850000',
        reserve1: '127500',
        tvl: '$255,000',
        volume24h: '$45,123',
        fees24h: '$135.37',
        apy: 8.32,
      },
      {
        address: '0x567890abcdef1234567890abcdef1234567890ab',
        token0: 'CAES',
        token1: 'MATIC',
        reserve0: '500000',
        reserve1: '125000',
        tvl: '$150,000',
        volume24h: '$25,678',
        fees24h: '$77.03',
        apy: 15.67,
      },
    ];

    setPools(mockPools);
  }, []);

  const formatNumber = (value: string) => {
    const num = parseFloat(value.replace(/[$,]/g, ''));
    if (num >= 1000000) return `$${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `$${(num / 1000).toFixed(1)}K`;
    return value;
  };

  const formatTokenAmount = (amount: string) => {
    const num = parseFloat(amount);
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toLocaleString();
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <Droplets className="text-tablets-cyan" size={24} />
          <div>
            <h3 className="text-xl font-semibold">Liquidity Pools</h3>
            <p className="text-gray-400 text-sm">Manage and track liquidity positions</p>
          </div>
        </div>
        <button className="btn-cyan">
          Add Liquidity
        </button>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="text-gray-400 text-sm border-b border-gray-700">
              <th className="text-left py-3">Pool</th>
              <th className="text-right py-3">TVL</th>
              <th className="text-right py-3">24h Volume</th>
              <th className="text-right py-3">24h Fees</th>
              <th className="text-right py-3">APY</th>
              <th className="text-center py-3">Actions</th>
            </tr>
          </thead>
          <tbody>
            {pools.map((pool) => (
              <tr key={pool.address} className="border-b border-gray-800 hover:bg-gray-800/50">
                <td className="py-4">
                  <div className="flex items-center gap-3">
                    <div className="flex -space-x-2">
                      <img
                        src={`/${pool.token0.toLowerCase()}-logo.png`}
                        alt={pool.token0}
                        className="w-8 h-8 rounded-full border-2 border-gray-700"
                        onError={(e) => {
                          (e.target as HTMLImageElement).src = '/default-token.png';
                        }}
                      />
                      <img
                        src={`/${pool.token1.toLowerCase()}-logo.png`}
                        alt={pool.token1}
                        className="w-8 h-8 rounded-full border-2 border-gray-700"
                        onError={(e) => {
                          (e.target as HTMLImageElement).src = '/default-token.png';
                        }}
                      />
                    </div>
                    <div>
                      <p className="font-medium">{pool.token0}/{pool.token1}</p>
                      <p className="text-gray-400 text-sm">
                        {formatTokenAmount(pool.reserve0)} {pool.token0} • {formatTokenAmount(pool.reserve1)} {pool.token1}
                      </p>
                    </div>
                  </div>
                </td>
                <td className="py-4 text-right font-mono">
                  {formatNumber(pool.tvl)}
                </td>
                <td className="py-4 text-right font-mono">
                  {formatNumber(pool.volume24h)}
                </td>
                <td className="py-4 text-right font-mono text-green-500">
                  {pool.fees24h}
                </td>
                <td className="py-4 text-right">
                  <div className="flex items-center justify-end gap-1">
                    <TrendingUp size={14} className="text-green-500" />
                    <span className="font-medium text-green-500">{pool.apy.toFixed(2)}%</span>
                  </div>
                </td>
                <td className="py-4 text-center">
                  <div className="flex items-center justify-center gap-2">
                    <button className="text-tablets-purple hover:text-purple-400 transition-colors">
                      Stake
                    </button>
                    <a
                      href={`https://sepolia.etherscan.io/address/${pool.address}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-gray-400 hover:text-white transition-colors"
                    >
                      <ExternalLink size={14} />
                    </a>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pool Stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6 pt-6 border-t border-gray-700">
        <div className="text-center">
          <p className="text-gray-400 text-sm">Total TVL</p>
          <p className="text-xl font-bold text-tablets-cyan">$830.7K</p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">24h Volume</p>
          <p className="text-xl font-bold text-tablets-purple">$156.0K</p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">24h Fees</p>
          <p className="text-xl font-bold text-tablets-emerald">$468.10</p>
        </div>
        <div className="text-center">
          <p className="text-gray-400 text-sm">Avg APY</p>
          <p className="text-xl font-bold text-caesar-gold">12.15%</p>
        </div>
      </div>
    </div>
  );
};

export default LiquidityPools;
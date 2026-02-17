// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { BarChart3, TrendingUp, Users, DollarSign, Coins, Activity } from 'lucide-react';
import MetricCard from './components/MetricCard';
import DemurrageChart from './components/DemurrageChart';
import LiquidityPools from './components/LiquidityPools';
import { MetricData, TokenMetrics } from './types';

function App() {
  const [tokenMetrics, setTokenMetrics] = useState<TokenMetrics>({
    totalSupply: '1,000,000',
    circulatingSupply: '980,450',
    marketCap: '$147,067.50',
    price: 0.15,
    priceChange24h: 5.67,
    volume24h: '$85,234',
    tvl: '$830,678',
    holders: 1247,
    totalTransactions: 8932,
    demurrageRate: 2.0,
    effectiveSupply: '978,234',
  });

  const [timeframe, setTimeframe] = useState('24H');

  useEffect(() => {
    // Simulate real-time data updates
    const interval = setInterval(() => {
      setTokenMetrics(prev => ({
        ...prev,
        price: prev.price + (Math.random() - 0.5) * 0.01,
        priceChange24h: prev.priceChange24h + (Math.random() - 0.5) * 0.5,
        volume24h: `$${(parseFloat(prev.volume24h.replace(/[$,]/g, '')) + Math.random() * 1000).toLocaleString()}`,
      }));
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const metrics: MetricData[] = [
    {
      label: 'Token Price',
      value: `$${tokenMetrics.price.toFixed(4)}`,
      change: tokenMetrics.priceChange24h,
      color: '#FFD700',
    },
    {
      label: 'Market Cap',
      value: tokenMetrics.marketCap,
      change: 3.2,
      color: '#8B5CF6',
    },
    {
      label: '24h Volume',
      value: tokenMetrics.volume24h,
      change: 12.8,
      color: '#06B6D4',
    },
    {
      label: 'Total Value Locked',
      value: tokenMetrics.tvl,
      change: -1.5,
      color: '#10B981',
    },
    {
      label: 'Holders',
      value: tokenMetrics.holders.toLocaleString(),
      change: 8.4,
      color: '#F59E0B',
    },
    {
      label: 'Transactions',
      value: tokenMetrics.totalTransactions.toLocaleString(),
      change: 15.2,
      color: '#EF4444',
    },
  ];

  return (
    <div className="min-h-screen bg-caesar-dark">
      {/* Header */}
      <header className="border-b border-gray-800 p-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-3">
            <BarChart3 className="text-tablets-purple" size={32} />
            <div>
              <h1 className="text-2xl font-bold text-tablets-purple">Tablets UI</h1>
              <p className="text-sm text-gray-400">Caesar Token Analytics Dashboard</p>
            </div>
          </div>
          
          <div className="flex items-center gap-4">
            <div className="flex bg-caesar-gray rounded-lg p-1">
              {['1H', '24H', '7D', '30D'].map((period) => (
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
            
            <nav className="hidden md:flex items-center gap-4">
              <a href="http://localhost:3001" target="_blank" className="flex items-center gap-2 text-gray-300 hover:text-white transition-colors">
                <Activity size={20} />
                Agora DEX
              </a>
              <a href="http://localhost:3002" target="_blank" className="flex items-center gap-2 text-gray-300 hover:text-white transition-colors">
                <Users size={20} />
                Satchel Wallet
              </a>
            </nav>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto p-4">
        {/* Hero Section */}
        <div className="card-analytics mb-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-3xl font-bold mb-2">Caesar Token (CAES)</h2>
              <p className="text-purple-100 mb-4">
                Advanced demurrage token with cross-chain capabilities powered by LayerZero V2
              </p>
              <div className="flex items-center gap-6">
                <div>
                  <p className="text-purple-200 text-sm">Current Price</p>
                  <p className="text-2xl font-bold">${tokenMetrics.price.toFixed(4)}</p>
                </div>
                <div>
                  <p className="text-purple-200 text-sm">Effective Supply</p>
                  <p className="text-2xl font-bold">{tokenMetrics.effectiveSupply}</p>
                </div>
                <div>
                  <p className="text-purple-200 text-sm">Demurrage Rate</p>
                  <p className="text-2xl font-bold">{tokenMetrics.demurrageRate}%</p>
                </div>
              </div>
            </div>
            <div className="text-right">
              <Coins size={64} className="text-white/20" />
            </div>
          </div>
        </div>

        {/* Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-6">
          {metrics.map((metric, index) => (
            <MetricCard key={index} metric={metric} />
          ))}
        </div>

        {/* Charts Section */}
        <div className="grid grid-cols-1 xl:grid-cols-1 gap-6 mb-6">
          <DemurrageChart />
        </div>

        {/* Liquidity Pools */}
        <div className="mb-6">
          <LiquidityPools />
        </div>

        {/* Network Stats */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Cross-Chain Overview */}
          <div className="card">
            <h3 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <Activity className="text-tablets-cyan" size={20} />
              Cross-Chain Activity
            </h3>
            <div className="space-y-4">
              {[
                { chain: 'Ethereum', tvl: '$450.2K', volume: '$125.4K', color: '#627EEA' },
                { chain: 'Polygon', tvl: '$234.8K', volume: '$67.2K', color: '#8247E5' },
                { chain: 'Arbitrum', tvl: '$145.7K', volume: '$45.8K', color: '#28A0F0' },
                { chain: 'Base', tvl: '$89.3K', volume: '$28.1K', color: '#0052FF' },
              ].map((network) => (
                <div key={network.chain} className="flex items-center justify-between p-3 bg-gray-800 rounded-lg">
                  <div className="flex items-center gap-3">
                    <div 
                      className="w-4 h-4 rounded-full" 
                      style={{ backgroundColor: network.color }}
                    />
                    <span className="font-medium">{network.chain}</span>
                  </div>
                  <div className="text-right">
                    <p className="font-medium">{network.tvl}</p>
                    <p className="text-gray-400 text-sm">{network.volume} 24h vol</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Yield Farming */}
          <div className="card">
            <h3 className="text-xl font-semibold mb-4 flex items-center gap-2">
              <DollarSign className="text-tablets-emerald" size={20} />
              Yield Farming
            </h3>
            <div className="space-y-4">
              {[
                { name: 'CAES-ETH LP', apy: 24.5, tvl: '$425K', rewards: '1,250 CAES' },
                { name: 'CAES-USDC LP', apy: 18.2, tvl: '$255K', rewards: '750 CAES' },
                { name: 'CAES Staking', apy: 12.0, tvl: '$180K', rewards: '500 CAES' },
              ].map((farm) => (
                <div key={farm.name} className="flex items-center justify-between p-3 bg-gray-800 rounded-lg">
                  <div>
                    <p className="font-medium">{farm.name}</p>
                    <p className="text-gray-400 text-sm">TVL: {farm.tvl}</p>
                  </div>
                  <div className="text-right">
                    <p className="font-medium text-tablets-emerald">{farm.apy}% APY</p>
                    <p className="text-gray-400 text-sm">{farm.rewards}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-800 mt-8 p-6">
        <div className="max-w-7xl mx-auto text-center text-gray-400 text-sm">
          <p>&copy; 2025 Tablets UI. Real-time analytics for Caesar Token ecosystem.</p>
          <p className="mt-2">
            📊 Data refreshes every 5 seconds • 🔗 Multi-chain tracking enabled
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;
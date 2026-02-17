// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface MetricData {
  label: string;
  value: string | number;
  change?: number;
  changeType?: 'increase' | 'decrease';
  icon?: string;
  color?: string;
}

export interface ChartData {
  timestamp: number;
  value: number;
  volume?: number;
  tvl?: number;
  holders?: number;
}

export interface TokenMetrics {
  totalSupply: string;
  circulatingSupply: string;
  marketCap: string;
  price: number;
  priceChange24h: number;
  volume24h: string;
  tvl: string;
  holders: number;
  totalTransactions: number;
  demurrageRate: number;
  effectiveSupply: string;
}

export interface DemurrageData {
  timestamp: number;
  totalBurned: number;
  burnRate: number;
  effectiveSupply: number;
  demurrageMultiplier: number;
}

export interface NetworkStats {
  chainId: number;
  name: string;
  tvl: string;
  transactions: number;
  uniqueUsers: number;
  avgGasPrice: string;
}

export interface LiquidityPool {
  address: string;
  token0: string;
  token1: string;
  reserve0: string;
  reserve1: string;
  tvl: string;
  volume24h: string;
  fees24h: string;
  apy: number;
}

export interface YieldFarm {
  name: string;
  pair: string;
  tvl: string;
  apy: number;
  totalRewards: string;
  userStaked?: string;
  userRewards?: string;
}
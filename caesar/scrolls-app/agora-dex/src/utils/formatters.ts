// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { ethers } from 'ethers';

export const formatTokenAmount = (amount: string, decimals: number = 18): string => {
  try {
    const formatted = ethers.formatUnits(amount, decimals);
    const num = parseFloat(formatted);
    
    if (num === 0) return '0';
    if (num < 0.0001) return '< 0.0001';
    if (num < 1) return num.toFixed(6);
    if (num < 1000) return num.toFixed(4);
    if (num < 1000000) return (num / 1000).toFixed(2) + 'K';
    return (num / 1000000).toFixed(2) + 'M';
  } catch {
    return '0';
  }
};

export const formatPrice = (price: number): string => {
  if (price === 0) return '$0';
  if (price < 0.01) return `$${price.toFixed(6)}`;
  if (price < 1) return `$${price.toFixed(4)}`;
  if (price < 1000) return `$${price.toFixed(2)}`;
  if (price < 1000000) return `$${(price / 1000).toFixed(1)}K`;
  return `$${(price / 1000000).toFixed(1)}M`;
};

export const formatPercentage = (percentage: number): string => {
  const sign = percentage >= 0 ? '+' : '';
  return `${sign}${percentage.toFixed(2)}%`;
};

export const formatAddress = (address: string, startLength = 6, endLength = 4): string => {
  if (!address) return '';
  if (address.length <= startLength + endLength) return address;
  return `${address.slice(0, startLength)}...${address.slice(-endLength)}`;
};

export const formatTimestamp = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleTimeString();
};

export const formatDate = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleDateString();
};

export const parseTokenAmount = (amount: string, decimals: number = 18): string => {
  try {
    return ethers.parseUnits(amount || '0', decimals).toString();
  } catch {
    return '0';
  }
};

export const calculateDemurrage = (
  amount: string, 
  lastUpdate: number, 
  demurrageRate: number = 0.02 // 2% annual
): string => {
  const now = Math.floor(Date.now() / 1000);
  const timeElapsed = now - lastUpdate;
  const yearInSeconds = 365 * 24 * 60 * 60;
  const demurrageMultiplier = Math.exp(-(demurrageRate * timeElapsed) / yearInSeconds);
  
  const originalAmount = parseFloat(amount);
  const currentAmount = originalAmount * demurrageMultiplier;
  
  return currentAmount.toString();
};
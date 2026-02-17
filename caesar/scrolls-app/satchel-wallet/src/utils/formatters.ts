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

export const formatUSD = (amount: number): string => {
  if (amount === 0) return '$0.00';
  if (amount < 0.01) return '< $0.01';
  if (amount < 1000) return `$${amount.toFixed(2)}`;
  if (amount < 1000000) return `$${(amount / 1000).toFixed(1)}K`;
  return `$${(amount / 1000000).toFixed(1)}M`;
};

export const formatAddress = (address: string, startLength = 6, endLength = 4): string => {
  if (!address) return '';
  if (address.length <= startLength + endLength) return address;
  return `${address.slice(0, startLength)}...${address.slice(-endLength)}`;
};

export const formatGas = (gasUsed: string, gasPrice: string): string => {
  try {
    const used = ethers.formatUnits(gasUsed, 'gwei');
    const price = ethers.formatUnits(gasPrice, 'gwei');
    const total = parseFloat(used) * parseFloat(price);
    return `${total.toFixed(6)} ETH`;
  } catch {
    return 'Unknown';
  }
};

export const formatTimestamp = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleString();
};

export const parseTokenAmount = (amount: string, decimals: number = 18): string => {
  try {
    return ethers.parseUnits(amount || '0', decimals).toString();
  } catch {
    return '0';
  }
};

export const copyToClipboard = async (text: string): Promise<boolean> => {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback for older browsers
    const textarea = document.createElement('textarea');
    textarea.value = text;
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    document.body.removeChild(textarea);
    return true;
  }
};
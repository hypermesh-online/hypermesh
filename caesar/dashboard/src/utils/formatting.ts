// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Utility functions for formatting data in the economic dashboard
 */

/**
 * Format currency values
 */
export const formatCurrency = (
  value: string | number,
  currency: string = 'USD',
  decimals: number = 2
): string => {
  const numericValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (isNaN(numericValue)) {
    return '$0.00';
  }

  // Handle large numbers with appropriate suffixes
  if (numericValue >= 1e9) {
    return `$${(numericValue / 1e9).toFixed(1)}B`;
  }
  if (numericValue >= 1e6) {
    return `$${(numericValue / 1e6).toFixed(1)}M`;
  }
  if (numericValue >= 1e3) {
    return `$${(numericValue / 1e3).toFixed(1)}K`;
  }

  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency,
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(numericValue);
};

/**
 * Format percentage values
 */
export const formatPercentage = (
  value: number,
  decimals: number = 2,
  showSign: boolean = false
): string => {
  if (isNaN(value)) {
    return '0%';
  }

  const formatted = new Intl.NumberFormat('en-US', {
    style: 'percent',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
    signDisplay: showSign ? 'always' : 'auto',
  }).format(value / 100);

  return formatted;
};

/**
 * Format basis points as percentage
 */
export const formatBasisPoints = (
  basisPoints: number,
  decimals: number = 2
): string => {
  return formatPercentage(basisPoints / 100, decimals);
};

/**
 * Format timestamp to human readable format
 */
export const formatTimestamp = (
  timestamp: number,
  includeTime: boolean = true
): string => {
  const date = new Date(timestamp * 1000); // Convert from seconds to milliseconds
  
  const options: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  };

  if (includeTime) {
    options.hour = '2-digit';
    options.minute = '2-digit';
  }

  return new Intl.DateTimeFormat('en-US', options).format(date);
};

/**
 * Format time duration
 */
export const formatDuration = (seconds: number): string => {
  if (seconds < 60) {
    return `${seconds}s`;
  }
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  }
  if (seconds < 86400) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  }
  
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  return `${days}d ${hours}h`;
};

/**
 * Format large numbers with appropriate suffixes
 */
export const formatNumber = (
  value: number,
  decimals: number = 2,
  forceDecimals: boolean = false
): string => {
  if (isNaN(value)) {
    return '0';
  }

  if (value >= 1e9) {
    return `${(value / 1e9).toFixed(decimals)}B`;
  }
  if (value >= 1e6) {
    return `${(value / 1e6).toFixed(decimals)}M`;
  }
  if (value >= 1e3) {
    return `${(value / 1e3).toFixed(decimals)}K`;
  }

  if (forceDecimals || value % 1 !== 0) {
    return value.toFixed(decimals);
  }
  
  return value.toString();
};

/**
 * Format wei values to readable format
 */
export const formatWei = (
  weiValue: string | number,
  decimals: number = 18,
  displayDecimals: number = 4
): string => {
  const value = typeof weiValue === 'string' ? parseFloat(weiValue) : weiValue;
  const divisor = Math.pow(10, decimals);
  const converted = value / divisor;
  
  return formatNumber(converted, displayDecimals);
};

/**
 * Format Ethereum addresses for display
 */
export const formatAddress = (
  address: string,
  startChars: number = 6,
  endChars: number = 4
): string => {
  if (!address || address.length < startChars + endChars) {
    return address;
  }

  return `${address.slice(0, startChars)}...${address.slice(-endChars)}`;
};

/**
 * Format health scores with color coding
 */
export const formatHealthScore = (
  score: number,
  maxScore: number = 1000
): {
  formatted: string;
  percentage: number;
  color: string;
  status: string;
} => {
  const percentage = (score / maxScore) * 100;
  const formatted = formatPercentage(percentage);
  
  let color: string;
  let status: string;
  
  if (percentage >= 80) {
    color = 'text-green-600';
    status = 'Excellent';
  } else if (percentage >= 60) {
    color = 'text-blue-600';
    status = 'Good';
  } else if (percentage >= 40) {
    color = 'text-yellow-600';
    status = 'Fair';
  } else if (percentage >= 20) {
    color = 'text-orange-600';
    status = 'Poor';
  } else {
    color = 'text-red-600';
    status = 'Critical';
  }
  
  return { formatted, percentage, color, status };
};

/**
 * Format risk scores with appropriate styling
 */
export const formatRiskScore = (
  score: number,
  maxScore: number = 1000
): {
  formatted: string;
  level: string;
  color: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
} => {
  const percentage = (score / maxScore) * 100;
  const formatted = `${score}/${maxScore}`;
  
  let level: string;
  let color: string;
  let severity: 'low' | 'medium' | 'high' | 'critical';
  
  if (score <= 300) {
    level = 'Low Risk';
    color = 'text-green-600';
    severity = 'low';
  } else if (score <= 600) {
    level = 'Medium Risk';
    color = 'text-yellow-600';
    severity = 'medium';
  } else if (score <= 800) {
    level = 'High Risk';
    color = 'text-orange-600';
    severity = 'high';
  } else {
    level = 'Critical Risk';
    color = 'text-red-600';
    severity = 'critical';
  }
  
  return { formatted, level, color, severity };
};

/**
 * Format time until next action
 */
export const formatTimeUntil = (futureTimestamp: number): string => {
  const now = Date.now() / 1000;
  const diff = futureTimestamp - now;
  
  if (diff <= 0) {
    return 'Overdue';
  }
  
  return formatDuration(diff);
};

/**
 * Format relative time (time ago)
 */
export const formatTimeAgo = (pastTimestamp: number): string => {
  const now = Date.now() / 1000;
  const diff = now - pastTimestamp;
  
  if (diff < 60) {
    return 'Just now';
  }
  if (diff < 3600) {
    const minutes = Math.floor(diff / 60);
    return `${minutes}m ago`;
  }
  if (diff < 86400) {
    const hours = Math.floor(diff / 3600);
    return `${hours}h ago`;
  }
  
  const days = Math.floor(diff / 86400);
  if (days === 1) {
    return '1 day ago';
  }
  if (days < 30) {
    return `${days} days ago`;
  }
  
  const months = Math.floor(days / 30);
  if (months === 1) {
    return '1 month ago';
  }
  
  return `${months} months ago`;
};

/**
 * Format transaction hash for display
 */
export const formatTxHash = (hash: string): string => {
  return formatAddress(hash, 10, 6);
};

/**
 * Format gas prices
 */
export const formatGasPrice = (gasPrice: number, unit: 'gwei' | 'wei' = 'gwei'): string => {
  if (unit === 'gwei') {
    return `${gasPrice.toFixed(1)} gwei`;
  }
  
  const gwei = gasPrice / 1e9;
  return `${gwei.toFixed(1)} gwei`;
};

/**
 * Color utilities for charts and UI
 */
export const getHealthColor = (score: number, maxScore: number = 1000): string => {
  const percentage = (score / maxScore) * 100;
  
  if (percentage >= 80) return '#10b981'; // green-500
  if (percentage >= 60) return '#3b82f6'; // blue-500
  if (percentage >= 40) return '#f59e0b'; // yellow-500
  if (percentage >= 20) return '#f97316'; // orange-500
  return '#ef4444'; // red-500
};

/**
 * Get risk level color
 */
export const getRiskColor = (riskScore: number, maxScore: number = 1000): string => {
  if (riskScore <= 300) return '#10b981'; // green-500
  if (riskScore <= 600) return '#f59e0b'; // yellow-500
  if (riskScore <= 800) return '#f97316'; // orange-500
  return '#ef4444'; // red-500
};

/**
 * Validation utilities
 */
export const isValidNumber = (value: any): boolean => {
  return typeof value === 'number' && !isNaN(value) && isFinite(value);
};

export const isValidAddress = (address: string): boolean => {
  return /^0x[a-fA-F0-9]{40}$/.test(address);
};

export const isValidTransactionHash = (hash: string): boolean => {
  return /^0x[a-fA-F0-9]{64}$/.test(hash);
};

/**
 * Chart data formatting utilities
 */
export const formatChartData = (data: any[], xKey: string, yKey: string) => {
  return data.map(item => ({
    ...item,
    [xKey]: typeof item[xKey] === 'number' && xKey.includes('timestamp') 
      ? new Date(item[xKey] * 1000).toLocaleTimeString() 
      : item[xKey],
    [yKey]: isValidNumber(item[yKey]) ? Number(item[yKey]) : 0,
  }));
};

/**
 * Export all formatting functions as a single object for easier importing
 */
export const formatters = {
  currency: formatCurrency,
  percentage: formatPercentage,
  basisPoints: formatBasisPoints,
  timestamp: formatTimestamp,
  duration: formatDuration,
  number: formatNumber,
  wei: formatWei,
  address: formatAddress,
  healthScore: formatHealthScore,
  riskScore: formatRiskScore,
  timeUntil: formatTimeUntil,
  timeAgo: formatTimeAgo,
  txHash: formatTxHash,
  gasPrice: formatGasPrice,
  chartData: formatChartData,
};

export default formatters;
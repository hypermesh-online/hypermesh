// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Economic formulas ported from concept/formulas.py
 * Implements the sophisticated gold-based economic model with deviation bands
 */

export interface StabilityMetrics {
  priceStabilityIndex: number;
  liquidityHealthIndex: number;
  networkUtilityScore: number;
  convergenceRate: number;
}

export interface MarketMetrics {
  pressure: number;
  volatility: number;
  liquidityRatio: number;
  validatorCount: number;
  holderCount: number;
  dailyTransactions: number;
}

export const formulas = {
  /**
   * Price Stability Index (PSI)
   * Combines price deviation, market pressure and participation metrics
   */
  priceStabilityIndex(
    currentPrice: number,
    targetPrice: number,
    marketPressure: number,
    validatorParticipation: number,
    holderParticipation: number
  ): number {
    const priceDeviation = Math.abs(targetPrice - currentPrice) / targetPrice;
    const pressureFactor = 1 / (1 + Math.abs(marketPressure));
    const participationScore = Math.min(1, (validatorParticipation + holderParticipation) / 2);
    
    const baseStability = 0.3; // Minimum stability floor
    const stability = baseStability + (
      0.3 * (1 / (1 + priceDeviation)) +
      0.2 * pressureFactor +
      0.2 * participationScore
    );
    
    return Math.min(1, stability);
  },

  /**
   * Market Pressure calculation
   * Based on volume imbalance and liquidity conditions
   */
  marketPressure(
    buysVolume: number,
    sellsVolume: number,
    effectiveLiquidity: number,
    validatorCount: number
  ): number {
    const volumeImbalance = (buysVolume - sellsVolume) / Math.max(1, buysVolume + sellsVolume);
    const liquidityFactor = effectiveLiquidity / Math.max(1, validatorCount * 1000000);
    return volumeImbalance * (1 / Math.max(0.1, liquidityFactor));
  },

  /**
   * Network Utility Score (NUS)
   * Based on transaction metrics and cross-chain activity
   */
  networkUtilityScore(
    dailyTransactions: number,
    crossChainTransfers: number,
    targetTransfers: number = 500000
  ): number {
    // Base transaction utility (weight: 0.6)
    const txUtility = Math.min(1.0, dailyTransactions / (targetTransfers * 2));
    
    // Cross-chain integration utility (weight: 0.4)
    const crossChainUtility = Math.min(1.0, crossChainTransfers / targetTransfers);
    
    return (txUtility * 0.6) + (crossChainUtility * 0.4);
  },

  /**
   * Liquidity Health Index (LHI)
   * Formula: H(t) = (AP/TH) * (L(t)/0.8) * (SR(t)/RR)
   */
  liquidityHealthIndex(
    activeParticipants: number,
    totalHolders: number,
    currentLiquidity: number,
    stabilityReserve: number,
    requiredReserve: number
  ): number {
    const participationRatio = activeParticipants / Math.max(1, totalHolders);
    const liquidityRatio = currentLiquidity / 0.8; // Target liquidity ratio
    const reserveRatio = stabilityReserve / Math.max(1, requiredReserve);
    
    // Combined health score with minimum baseline
    return Math.max(0.2, Math.min(1, participationRatio * liquidityRatio * reserveRatio));
  },

  /**
   * Validator reward calculation
   * 90% of network revenue goes to validators
   */
  validatorReward(
    baseRewardRate: number,
    dailyTransactions: number,
    validatorCount: number,
    psi: number
  ): number {
    const transactionShare = dailyTransactions / Math.max(1, validatorCount);
    const marketReward = transactionShare * baseRewardRate * 0.9;
    const stabilityBonus = marketReward * psi;
    
    return marketReward + stabilityBonus;
  },

  /**
   * Holder cost with logarithmic scaling
   * D(t,L) = H * (1/L) * t * (1 - stability_index)
   */
  holderCost(
    baseRate: number,
    timeHeld: number,
    balance: number,
    priceStabilityIndex: number
  ): number {
    // Logarithmic scaling for large balances
    const balanceFactor = Math.log2(1 + balance / 1000);
    
    // Higher costs during instability
    const stabilityFactor = 1 - priceStabilityIndex;
    
    return baseRate * timeHeld * balanceFactor * stabilityFactor;
  },

  /**
   * Dynamic transaction fee calculation
   * S(v,s) = β * (1/L) * (1 + log₂(v/v₀)) * I(s)
   */
  transactionFee(
    baseFee: number,
    priceStabilityIndex: number,
    transactionSize: number,
    liquidityRatio: number
  ): number {
    // Volume scaling
    const volumeFactor = 1 + Math.log2(1 + transactionSize / 10000);
    
    // Liquidity impact
    const liquidityFactor = 1 / Math.max(0.1, liquidityRatio);
    
    // Stability adjustment
    const stabilityFactor = 1 + (1 - priceStabilityIndex);
    
    return baseFee * volumeFactor * liquidityFactor * stabilityFactor;
  },

  /**
   * Convergence rate calculation
   * dp/dt = α(p_target - p(t)) + β(pressure(t)) + γ(stability(t))
   */
  convergenceRate(
    currentPrice: number,
    targetPrice: number,
    marketPressure: number,
    stabilityIndex: number
  ): number {
    const α = 0.1;  // Price correction factor
    const β = -0.05;  // Pressure impact factor  
    const γ = 0.05;  // Stability impact factor
    
    const priceGap = targetPrice - currentPrice;
    return (α * priceGap) + (β * marketPressure) + (γ * stabilityIndex);
  },

  /**
   * Circuit breaker conditions
   */
  circuitBreakerConditions(
    liquidityRatio: number,
    currentPrice: number,
    targetPrice: number = 1,
    liquidityHealthIndex: number
  ): { halt: boolean; emergency: boolean; rebase: boolean } {
    const priceDeviation = Math.abs(currentPrice - targetPrice) / targetPrice;
    
    return {
      halt: liquidityRatio < 0.1,
      emergency: liquidityRatio < 0.2,
      rebase: priceDeviation > 0.2
    };
  },

  /**
   * Equilibrium state determination
   */
  equilibriumState(
    priceStabilityIndex: number,
    liquidityHealthIndex: number,
    networkUtilityScore: number,
    convergenceRate: number,
    thresholds: {
      psiMin: number;
      lhiMin: number;
      nusMin: number;
      convMax: number;
    } = {
      psiMin: 0.8,
      lhiMin: 0.7,
      nusMin: 0.6,
      convMax: 10
    }
  ): { isEquilibrium: boolean; failingMetrics: string[] } {
    const checks = {
      price_stability: priceStabilityIndex >= thresholds.psiMin,
      liquidity_health: liquidityHealthIndex >= thresholds.lhiMin,
      network_utility: networkUtilityScore >= thresholds.nusMin,
      convergence: Math.abs(convergenceRate) <= thresholds.convMax
    };
    
    const failing = Object.entries(checks)
      .filter(([_, passes]) => !passes)
      .map(([metric]) => metric);
    
    return {
      isEquilibrium: failing.length === 0,
      failingMetrics: failing
    };
  },

  /**
   * Dynamic spread calculation
   */
  dynamicSpread(
    baseSpreada: number,
    liquidityRatio: number,
    marketPressure: number,
    normalizedValidators: number
  ): number {
    // Increase spread when liquidity is low
    const liquidityFactor = Math.max(1, (0.8 / liquidityRatio) ** 2);
    
    // Increase spread under high market pressure
    const pressureFactor = 1 + Math.abs(marketPressure);
    
    // Decrease spread with more validators
    const validatorFactor = 1 / (0.5 + normalizedValidators);
    
    return baseSpreada * liquidityFactor * pressureFactor * validatorFactor;
  },

  /**
   * Stability reserve requirement
   */
  stabilityReserveRequirement(totalSupply: number, totalDecayPenalties: number): number {
    const baseRequirement = totalSupply * 0.1; // 10% of total supply
    const dynamicRequirement = totalDecayPenalties * 2; // 2x daily decay
    return Math.max(baseRequirement, dynamicRequirement);
  },

  /**
   * Calculate individual proportional cost (Factor 1 implementation)
   */
  individualProportionalCost(
    totalMarketCost: number,
    holderBalance: number,
    totalSupply: number,
    incentiveMultiplier: number = 1.0
  ): number {
    const baseCost = totalMarketCost * (holderBalance / totalSupply);
    return baseCost * incentiveMultiplier;
  },

  /**
   * Deviation band position calculator
   */
  deviationBandPosition(
    currentPrice: number,
    goldPrice: number,
    bandWidth: number = 0.05
  ): {
    percentage: number;
    status: 'above' | 'within' | 'below';
    upperBand: number;
    lowerBand: number;
  } {
    const upperBand = goldPrice * (1 + bandWidth);
    const lowerBand = goldPrice * (1 - bandWidth);
    const percentage = ((currentPrice - goldPrice) / goldPrice) * 100;
    
    let status: 'above' | 'within' | 'below' = 'within';
    if (currentPrice > upperBand) status = 'above';
    else if (currentPrice < lowerBand) status = 'below';
    
    return {
      percentage,
      status,
      upperBand,
      lowerBand
    };
  }
};

export default formulas;
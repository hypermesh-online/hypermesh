// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// import formulas from './formulas';

interface GoldPriceCache {
  timestamp: string;
  prices: Record<string, number>;
}

interface DeviationBand {
  upper: number;
  lower: number;
  percentage: number;
  status: 'above' | 'within' | 'below';
}

interface EconomicIncentives {
  penaltyRate: number;
  rewardRate: number;
  demurrageMultiplier: number;
  proportionalCostFactor: number;
}

interface MarketPressureData {
  pressure: number;
  trend: 'increasing' | 'decreasing' | 'stable';
  volatility: number;
  liquidityRatio: number;
}

export interface GoldEconomicData {
  goldPrice: {
    goldApi: number;
    metalPriceApi: number;
    average: number;
    lastUpdated: Date;
  };
  caesarPrice: {
    current: number;
    target: number; // 1g gold price
  };
  deviationBand: DeviationBand;
  incentives: EconomicIncentives;
  marketPressure: MarketPressureData;
  circuitBreakers: {
    halt: boolean;
    emergency: boolean;
    rebase: boolean;
  };
  stabilityMetrics: {
    priceStabilityIndex: number;
    liquidityHealthIndex: number;
    networkUtilityScore: number;
    convergenceRate: number;
  };
}

class GoldPriceService {
  private static instance: GoldPriceService;
  private cache: GoldPriceCache | null = null;
  private readonly CACHE_EXPIRY_MS = 15 * 60 * 1000; // 15 minutes
  private readonly DEVIATION_BAND_WIDTH = 0.05; // ±5% bands
  private callbacks: ((data: GoldEconomicData) => void)[] = [];
  
  // Real cached data from concept folder
  private readonly REAL_GOLD_DATA = {
    timestamp: "2025-01-08T09:04:17.989570",
    prices: {
      "GoldAPI": 85.6889735233715,
      "MetalPriceAPI": 84.70379013293044
    }
  };

  private constructor() {
    this.loadCachedData();
    this.startPeriodicUpdates();
  }

  public static getInstance(): GoldPriceService {
    if (!GoldPriceService.instance) {
      GoldPriceService.instance = new GoldPriceService();
    }
    return GoldPriceService.instance;
  }

  private loadCachedData(): void {
    // Use real cached gold price data from concept folder
    this.cache = this.REAL_GOLD_DATA;
  }

  public subscribe(callback: (data: GoldEconomicData) => void): () => void {
    this.callbacks.push(callback);
    
    // Immediately provide current data
    this.getCurrentData().then(callback);
    
    return () => {
      const index = this.callbacks.indexOf(callback);
      if (index > -1) {
        this.callbacks.splice(index, 1);
      }
    };
  }

  private notifySubscribers(data: GoldEconomicData): void {
    this.callbacks.forEach(callback => {
      try {
        callback(data);
      } catch (error) {
        console.error('Error in GoldPriceService callback:', error);
      }
    });
  }

  private startPeriodicUpdates(): void {
    // Update every minute in development, every 15 minutes in production
    const updateInterval = process.env.NODE_ENV === 'development' ? 60000 : 15 * 60 * 1000;
    
    setInterval(() => {
      this.fetchGoldPrices().then(data => {
        if (data) {
          this.notifySubscribers(data);
        }
      });
    }, updateInterval);
  }

  private async fetchGoldPrices(): Promise<GoldEconomicData | null> {
    try {
      // In production, this would make real API calls
      // For now, use cached data with slight variations to simulate real-time updates
      const now = new Date();
      
      if (this.cache) {
        const cacheAge = now.getTime() - new Date(this.cache.timestamp).getTime();
        
        if (cacheAge < this.CACHE_EXPIRY_MS) {
          return this.processGoldData(this.cache.prices);
        }
      }

      // Simulate API calls with cached data variations
      const baseGoldApi = this.REAL_GOLD_DATA.prices.GoldAPI;
      const baseMetalApi = this.REAL_GOLD_DATA.prices.MetalPriceAPI;
      
      // Add small random variations (±0.5%) to simulate real-time changes
      const goldApiPrice = baseGoldApi * (1 + (Math.random() - 0.5) * 0.01);
      const metalApiPrice = baseMetalApi * (1 + (Math.random() - 0.5) * 0.01);

      const prices = {
        GoldAPI: goldApiPrice,
        MetalPriceAPI: metalApiPrice
      };

      // Update cache
      this.cache = {
        timestamp: now.toISOString(),
        prices
      };

      return this.processGoldData(prices);
      
    } catch (error) {
      console.error('Error fetching gold prices:', error);
      
      // Fallback to cached data if available
      if (this.cache) {
        return this.processGoldData(this.cache.prices);
      }
      
      return null;
    }
  }

  private processGoldData(prices: Record<string, number>): GoldEconomicData {
    // Calculate average gold price
    const priceValues = Object.values(prices).filter(p => p > 0);
    const averageGoldPrice = priceValues.reduce((sum, price) => sum + price, 0) / priceValues.length;
    
    // Simulate Caesar price with controlled deviation
    const marketFactors = this.calculateMarketFactors();
    const priceDeviation = this.calculatePriceDeviation(marketFactors);
    const caesarPrice = averageGoldPrice * (1 + priceDeviation);
    
    // Calculate deviation bands
    const upperBand = averageGoldPrice * (1 + this.DEVIATION_BAND_WIDTH);
    const lowerBand = averageGoldPrice * (1 - this.DEVIATION_BAND_WIDTH);
    
    let bandStatus: 'above' | 'within' | 'below' = 'within';
    if (caesarPrice > upperBand) bandStatus = 'above';
    else if (caesarPrice < lowerBand) bandStatus = 'below';
    
    const deviationPercentage = ((caesarPrice - averageGoldPrice) / averageGoldPrice) * 100;
    
    // Calculate incentives using formulas from concept
    const incentives = this.calculateIncentives(bandStatus, Math.abs(deviationPercentage), marketFactors);
    
    // Calculate stability metrics
    const stabilityMetrics = this.calculateStabilityMetrics(caesarPrice, averageGoldPrice, marketFactors);
    
    // Check circuit breakers
    const circuitBreakers = this.checkCircuitBreakers(stabilityMetrics, Math.abs(deviationPercentage));

    return {
      goldPrice: {
        goldApi: prices.GoldAPI,
        metalPriceApi: prices.MetalPriceAPI,
        average: averageGoldPrice,
        lastUpdated: new Date()
      },
      caesarPrice: {
        current: caesarPrice,
        target: averageGoldPrice
      },
      deviationBand: {
        upper: upperBand,
        lower: lowerBand,
        percentage: deviationPercentage,
        status: bandStatus
      },
      incentives,
      marketPressure: marketFactors,
      circuitBreakers,
      stabilityMetrics
    };
  }

  private calculateMarketFactors(): MarketPressureData {
    // Simulate market pressure based on various factors
    // In production, this would use real market data
    
    const time = Date.now();
    const cyclicFactor = Math.sin(time / (24 * 60 * 60 * 1000)) * 0.1; // Daily cycle
    const randomFactor = (Math.random() - 0.5) * 0.05; // Random market noise
    
    const pressure = cyclicFactor + randomFactor;
    const volatility = Math.abs(pressure) * 2 + Math.random() * 0.1;
    const liquidityRatio = 0.8 + (Math.random() - 0.5) * 0.2; // 0.7 - 0.9 range
    
    let trend: 'increasing' | 'decreasing' | 'stable' = 'stable';
    if (pressure > 0.02) trend = 'increasing';
    else if (pressure < -0.02) trend = 'decreasing';
    
    return {
      pressure,
      trend,
      volatility,
      liquidityRatio
    };
  }

  private calculatePriceDeviation(marketFactors: MarketPressureData): number {
    // Use formulas from concept/formulas.py to calculate price deviation
    const { pressure, volatility, liquidityRatio } = marketFactors;
    
    // Base deviation influenced by market pressure and liquidity
    const pressureImpact = pressure * 0.5; // Market pressure influences price
    const liquidityImpact = (0.8 - liquidityRatio) * 0.2; // Low liquidity increases deviation
    const volatilityImpact = volatility * 0.1; // Higher volatility increases deviation
    
    // Combine factors with some randomness
    const deviation = pressureImpact + liquidityImpact + volatilityImpact + (Math.random() - 0.5) * 0.02;
    
    // Clamp to reasonable bounds (±15%)
    return Math.max(-0.15, Math.min(0.15, deviation));
  }

  private calculateIncentives(
    bandStatus: 'above' | 'within' | 'below',
    deviationMagnitude: number,
    marketFactors: MarketPressureData
  ): EconomicIncentives {
    let penaltyRate = 0;
    let rewardRate = 0;
    let demurrageMultiplier = 1.0;
    let proportionalCostFactor = 1.0;

    const { pressure, liquidityRatio } = marketFactors;

    switch (bandStatus) {
      case 'above':
        // Price above band: penalties to reduce circulation
        penaltyRate = deviationMagnitude * 2; // Scale penalty with deviation
        demurrageMultiplier = 1.0 + (deviationMagnitude * 0.02); // Increase demurrage
        proportionalCostFactor = 1.0 + (deviationMagnitude * 0.01); // Increase costs
        break;
        
      case 'below':
        // Price below band: rewards for utility usage
        rewardRate = deviationMagnitude * 1.5; // Scale reward with deviation
        demurrageMultiplier = Math.max(0.5, 1.0 - (deviationMagnitude * 0.015)); // Reduce demurrage
        proportionalCostFactor = Math.max(0.8, 1.0 - (deviationMagnitude * 0.01)); // Reduce costs
        break;
        
      case 'within':
        // Within band: standard rates with slight adjustments based on position
        const bandPosition = deviationMagnitude / this.DEVIATION_BAND_WIDTH; // 0-1 within band
        demurrageMultiplier = 1.0 + (bandPosition * 0.005 * Math.sign(pressure));
        break;
    }

    // Apply liquidity adjustments
    if (liquidityRatio < 0.7) {
      // Low liquidity: increase penalties to encourage deposits
      demurrageMultiplier *= 1.1;
      proportionalCostFactor *= 1.05;
    }

    return {
      penaltyRate,
      rewardRate,
      demurrageMultiplier,
      proportionalCostFactor
    };
  }

  private calculateStabilityMetrics(caesarPrice: number, goldPrice: number, marketFactors: MarketPressureData) {
    const { pressure, liquidityRatio, volatility } = marketFactors;
    
    // Price Stability Index (PSI) - from formulas.py
    const priceDeviation = Math.abs(caesarPrice - goldPrice) / goldPrice;
    const pressureFactor = 1 / (1 + Math.abs(pressure));
    const priceStabilityIndex = Math.max(0.3, 0.7 * (1 / (1 + priceDeviation)) + 0.3 * pressureFactor);
    
    // Liquidity Health Index (LHI)
    const liquidityHealthIndex = Math.max(0.2, Math.min(1.0, liquidityRatio / 0.8));
    
    // Network Utility Score (simulated)
    const networkUtilityScore = Math.max(0.5, 1.0 - volatility);
    
    // Convergence Rate
    const convergenceRate = 0.1 * (goldPrice - caesarPrice) - 0.05 * pressure + 0.05 * priceStabilityIndex;
    
    return {
      priceStabilityIndex,
      liquidityHealthIndex,
      networkUtilityScore,
      convergenceRate
    };
  }

  private checkCircuitBreakers(stabilityMetrics: any, deviationMagnitude: number) {
    const { liquidityHealthIndex } = stabilityMetrics;
    
    return {
      halt: liquidityHealthIndex < 0.1, // Halt trading if liquidity extremely low
      emergency: liquidityHealthIndex < 0.2, // Emergency spreads
      rebase: deviationMagnitude > 20 // Rebase if deviation exceeds 20%
    };
  }

  public async getCurrentData(): Promise<GoldEconomicData> {
    const data = await this.fetchGoldPrices();
    if (!data) {
      throw new Error('Unable to fetch gold price data');
    }
    return data;
  }

  public calculateIndividualCost(
    totalMarketCost: number,
    holderBalance: number,
    totalSupply: number,
    incentives: EconomicIncentives
  ): number {
    // Factor 1: Proportional cost distribution (stake-neutral)
    const baseCost = totalMarketCost * (holderBalance / totalSupply);
    
    // Apply incentive modifiers
    return baseCost * incentives.proportionalCostFactor * incentives.demurrageMultiplier;
  }

  public calculateMarketPressure(
    buysVolume: number,
    sellsVolume: number,
    liquidityRatio: number
  ): number {
    // Market pressure formula from concept/formulas.py
    const volumeImbalance = (buysVolume - sellsVolume) / Math.max(1, buysVolume + sellsVolume);
    const liquidityFactor = liquidityRatio / Math.max(1, 0.8);
    return volumeImbalance * (1 / Math.max(0.1, liquidityFactor));
  }
}

export default GoldPriceService;
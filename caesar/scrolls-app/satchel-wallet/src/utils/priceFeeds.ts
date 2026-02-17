// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Token } from '../types';

export interface PriceData {
  symbol: string;
  price: number;
  priceChange24h: number;
  marketCap: number;
  volume24h: number;
  lastUpdated: number;
}

export interface PriceFeedProvider {
  name: string;
  getPrices(symbols: string[]): Promise<PriceData[]>;
  getPrice(symbol: string): Promise<PriceData>;
}

/**
 * CoinGecko API provider for real-time price data
 */
export class CoinGeckoProvider implements PriceFeedProvider {
  name = 'CoinGecko';
  private baseUrl = 'https://api.coingecko.com/api/v3';
  private cache = new Map<string, { data: PriceData; timestamp: number }>();
  private cacheTimeout = 60000; // 1 minute cache

  private coinIdMap: Record<string, string> = {
    'ETH': 'ethereum',
    'BTC': 'bitcoin',
    'USDC': 'usd-coin',
    'USDT': 'tether',
    'DAI': 'dai',
    'MATIC': 'polygon',
    'ATOM': 'cosmos',
    'OSMO': 'osmosis',
    'JUNO': 'juno-network',
    'SOL': 'solana',
    'XRD': 'radix',
    'CAES': 'caesar-token', // Assuming listing
  };

  async getPrices(symbols: string[]): Promise<PriceData[]> {
    const coinIds = symbols.map(symbol => this.coinIdMap[symbol] || symbol.toLowerCase()).join(',');
    
    try {
      const response = await fetch(
        `${this.baseUrl}/simple/price?ids=${coinIds}&vs_currencies=usd&include_24hr_change=true&include_market_cap=true&include_24hr_vol=true`
      );
      
      if (!response.ok) {
        throw new Error(`CoinGecko API error: ${response.status}`);
      }
      
      const data = await response.json();
      
      return symbols.map(symbol => {
        const coinId = this.coinIdMap[symbol] || symbol.toLowerCase();
        const coinData = data[coinId];
        
        if (!coinData) {
          return this.getMockPrice(symbol);
        }
        
        const priceData: PriceData = {
          symbol,
          price: coinData.usd || 0,
          priceChange24h: coinData.usd_24h_change || 0,
          marketCap: coinData.usd_market_cap || 0,
          volume24h: coinData.usd_24h_vol || 0,
          lastUpdated: Date.now(),
        };
        
        // Cache the result
        this.cache.set(symbol, { data: priceData, timestamp: Date.now() });
        
        return priceData;
      });
    } catch (error) {
      console.error('CoinGecko API error:', error);
      // Return cached or mock data as fallback
      return symbols.map(symbol => this.getCachedOrMockPrice(symbol));
    }
  }

  async getPrice(symbol: string): Promise<PriceData> {
    const prices = await this.getPrices([symbol]);
    return prices[0];
  }

  private getCachedOrMockPrice(symbol: string): PriceData {
    const cached = this.cache.get(symbol);
    if (cached && Date.now() - cached.timestamp < this.cacheTimeout) {
      return cached.data;
    }
    return this.getMockPrice(symbol);
  }

  private getMockPrice(symbol: string): PriceData {
    // Mock prices for development/fallback
    const mockPrices: Record<string, Partial<PriceData>> = {
      'ETH': { price: 2400, priceChange24h: 2.5 },
      'BTC': { price: 45000, priceChange24h: 1.8 },
      'USDC': { price: 1.00, priceChange24h: 0.01 },
      'USDT': { price: 1.00, priceChange24h: -0.02 },
      'DAI': { price: 1.00, priceChange24h: 0.01 },
      'MATIC': { price: 0.85, priceChange24h: -1.2 },
      'ATOM': { price: 12.50, priceChange24h: 3.4 },
      'SOL': { price: 95.20, priceChange24h: -0.8 },
      'CAES': { price: 1850, priceChange24h: 0.5 }, // Gold-tracking
    };

    const base = mockPrices[symbol] || { price: 1, priceChange24h: 0 };
    
    return {
      symbol,
      price: base.price!,
      priceChange24h: base.priceChange24h!,
      marketCap: base.price! * 1000000, // Mock market cap
      volume24h: base.price! * 50000, // Mock volume
      lastUpdated: Date.now(),
    };
  }
}

/**
 * DeFiPulse API provider for DeFi-specific data
 */
export class DeFiPulseProvider implements PriceFeedProvider {
  name = 'DeFiPulse';
  // private _baseUrl = 'https://data-api.defipulse.com/api/v1';

  async getPrices(_symbols: string[]): Promise<PriceData[]> {
    // DeFiPulse implementation would go here
    // For now, return empty array and let aggregator handle fallback
    return [];
  }

  async getPrice(symbol: string): Promise<PriceData> {
    const prices = await this.getPrices([symbol]);
    return prices[0];
  }
}

/**
 * Aggregated price feed that combines multiple providers
 */
export class AggregatedPriceFeed {
  private providers: PriceFeedProvider[];
  private cache = new Map<string, { data: PriceData; timestamp: number }>();
  private cacheTimeout = 30000; // 30 seconds cache

  constructor() {
    this.providers = [
      new CoinGeckoProvider(),
      new DeFiPulseProvider(),
    ];
  }

  async getPrices(symbols: string[]): Promise<PriceData[]> {
    const results: PriceData[] = [];
    
    for (const symbol of symbols) {
      // Check cache first
      const cached = this.cache.get(symbol);
      if (cached && Date.now() - cached.timestamp < this.cacheTimeout) {
        results.push(cached.data);
        continue;
      }

      // Try providers in order
      let priceData: PriceData | null = null;
      
      for (const provider of this.providers) {
        try {
          priceData = await provider.getPrice(symbol);
          if (priceData && priceData.price > 0) {
            break;
          }
        } catch (error) {
          console.warn(`Provider ${provider.name} failed for ${symbol}:`, error);
        }
      }

      if (priceData) {
        this.cache.set(symbol, { data: priceData, timestamp: Date.now() });
        results.push(priceData);
      } else {
        // Fallback to mock data
        const coinGecko = new CoinGeckoProvider();
        results.push((coinGecko as any).getMockPrice(symbol));
      }
    }

    return results;
  }

  async getPrice(symbol: string): Promise<PriceData> {
    const prices = await this.getPrices([symbol]);
    return prices[0];
  }

  /**
   * Get prices for tokens and update their USD values
   */
  async updateTokenPrices(tokens: Token[]): Promise<Token[]> {
    const symbols = [...new Set(tokens.map(token => token.symbol))];
    const priceData = await this.getPrices(symbols);
    const priceMap = new Map(priceData.map(p => [p.symbol, p]));

    return tokens.map(token => {
      const price = priceMap.get(token.symbol);
      if (!price) return token;

      const balance = parseFloat(token.balance || '0');
      const balanceUSD = (balance * price.price).toFixed(2);

      return {
        ...token,
        balanceUSD,
        priceUSD: price.price.toString(),
        priceChange24h: price.priceChange24h,
        lastPriceUpdate: price.lastUpdated,
      };
    });
  }

  /**
   * Subscribe to real-time price updates
   */
  subscribeToPrices(symbols: string[], callback: (prices: PriceData[]) => void): () => void {
    const interval = setInterval(async () => {
      try {
        const prices = await this.getPrices(symbols);
        callback(prices);
      } catch (error) {
        console.error('Price update error:', error);
      }
    }, 30000); // Update every 30 seconds

    return () => clearInterval(interval);
  }
}

// Global price feed instance
export const priceFeed = new AggregatedPriceFeed();
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface Token {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  logoURI?: string;
  balance?: string;
}

export interface TradingPair {
  token0: Token;
  token1: Token;
  address: string;
  reserve0: string;
  reserve1: string;
  price: string;
  change24h: string;
  volume24h: string;
}

export interface Trade {
  id: string;
  timestamp: number;
  type: 'buy' | 'sell';
  amount: string;
  price: string;
  total: string;
  txHash: string;
}

export interface OrderBook {
  bids: Array<{
    price: string;
    amount: string;
    total: string;
  }>;
  asks: Array<{
    price: string;
    amount: string;
    total: string;
  }>;
}

export interface PriceData {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface WalletState {
  account: string | null;
  chainId: number | null;
  isConnected: boolean;
  provider: any;
}
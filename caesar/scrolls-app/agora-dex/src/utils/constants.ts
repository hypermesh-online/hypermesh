// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Caesar Token Ecosystem Contracts - Sepolia Network
export const CONTRACTS = {
  SEPOLIA: {
    CAESAR_TOKEN: '0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4',
    DEX_FACTORY: '0xAe0DfF19f44D3544139d900a3f9f6c03C6764538',
    WETH: '0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9',
  }
};

export const SUPPORTED_CHAINS = {
  SEPOLIA: {
    chainId: 11155111,
    name: 'Sepolia',
    rpc: 'https://sepolia.infura.io/v3/',
    explorer: 'https://sepolia.etherscan.io',
    nativeCurrency: {
      name: 'Sepolia ETH',
      symbol: 'ETH',
      decimals: 18,
    }
  }
};

export const DEFAULT_TOKENS = [
  {
    address: CONTRACTS.SEPOLIA.CAESAR_TOKEN,
    symbol: 'CAES',
    name: 'Caesar Token',
    decimals: 18,
    logoURI: '/caesar-logo.png'
  },
  {
    address: CONTRACTS.SEPOLIA.WETH,
    symbol: 'WETH',
    name: 'Wrapped Ether',
    decimals: 18,
    logoURI: '/weth-logo.png'
  }
];

export const TRADING_FEE = 0.003; // 0.3%
export const PROTOCOL_FEE = 0.1667; // 16.67%

export const API_ENDPOINTS = {
  PRICE_FEED: 'https://api.coingecko.com/api/v3',
  SEPOLIA_RPC: 'https://sepolia.infura.io/v3/',
};
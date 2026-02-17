// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Caesar Token Ecosystem Contract Addresses

export const CONTRACTS = {
  SEPOLIA: {
    CAESAR_TOKEN: '0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4',
    DEX_FACTORY: '0xAe0DfF19f44D3544139d900a3f9f6c03C6764538',
    WETH: '0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9',
  },
  MAINNET: {
    // Mainnet addresses will be added when deployed
    CAESAR_TOKEN: '',
    DEX_FACTORY: '',
    WETH: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
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
  },
  MAINNET: {
    chainId: 1,
    name: 'Ethereum',
    rpc: 'https://mainnet.infura.io/v3/',
    explorer: 'https://etherscan.io',
    nativeCurrency: {
      name: 'Ether',
      symbol: 'ETH',
      decimals: 18,
    }
  },
  POLYGON: {
    chainId: 137,
    name: 'Polygon',
    rpc: 'https://polygon-rpc.com',
    explorer: 'https://polygonscan.com',
    nativeCurrency: {
      name: 'MATIC',
      symbol: 'MATIC',
      decimals: 18,
    }
  },
  ARBITRUM: {
    chainId: 42161,
    name: 'Arbitrum One',
    rpc: 'https://arb1.arbitrum.io/rpc',
    explorer: 'https://arbiscan.io',
    nativeCurrency: {
      name: 'Ether',
      symbol: 'ETH',
      decimals: 18,
    }
  },
  BASE: {
    chainId: 8453,
    name: 'Base',
    rpc: 'https://mainnet.base.org',
    explorer: 'https://basescan.org',
    nativeCurrency: {
      name: 'Ether',
      symbol: 'ETH',
      decimals: 18,
    }
  }
};

export const LAYERZERO_ENDPOINTS = {
  SEPOLIA: '0x6EDCE65403992e310A62460808c4b910D972f10f',
  MAINNET: '0x1a44076050125825900e736c501f859c50fE728c',
  POLYGON: '0x1a44076050125825900e736c501f859c50fE728c',
  ARBITRUM: '0x1a44076050125825900e736c501f859c50fE728c',
  BASE: '0x1a44076050125825900e736c501f859c50fE728c',
};
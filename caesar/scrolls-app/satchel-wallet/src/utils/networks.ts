// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Network, Token } from '../types';
import { getBridgedTokensForChain } from './bridges';

export const SUPPORTED_NETWORKS: Network[] = [
  // ========== EVM NETWORKS ==========
  {
    chainId: 1,
    name: 'Ethereum Mainnet',
    symbol: 'ETH',
    rpcUrl: 'https://mainnet.infura.io/v3/',
    blockExplorer: 'https://etherscan.io',
    logoUrl: '/eth-logo.png',
    chainType: 'evm',
    features: ['0x-protocol', 'layerzero-endpoint', 'hyperlane-endpoint'],
  },
  {
    chainId: 11155111,
    name: 'Sepolia Testnet',
    symbol: 'ETH',
    rpcUrl: 'https://sepolia.infura.io/v3/',
    blockExplorer: 'https://sepolia.etherscan.io',
    logoUrl: '/eth-logo.png',
    isTestnet: true,
    chainType: 'evm',
    features: ['0x-protocol', 'layerzero-endpoint', 'hyperlane-endpoint'],
  },
  {
    chainId: 137,
    name: 'Polygon',
    symbol: 'MATIC',
    rpcUrl: 'https://polygon-rpc.com',
    blockExplorer: 'https://polygonscan.com',
    logoUrl: '/polygon-logo.png',
    chainType: 'evm',
    features: ['0x-protocol', 'layerzero-endpoint', 'hyperlane-endpoint'],
  },
  {
    chainId: 42161,
    name: 'Arbitrum One',
    symbol: 'ETH',
    rpcUrl: 'https://arb1.arbitrum.io/rpc',
    blockExplorer: 'https://arbiscan.io',
    logoUrl: '/arbitrum-logo.png',
    chainType: 'evm',
    features: ['0x-protocol', 'layerzero-endpoint', 'hyperlane-endpoint'],
  },
  {
    chainId: 8453,
    name: 'Base',
    symbol: 'ETH',
    rpcUrl: 'https://mainnet.base.org',
    blockExplorer: 'https://basescan.org',
    logoUrl: '/base-logo.png',
    chainType: 'evm',
    features: ['layerzero-endpoint', 'hyperlane-endpoint'],
  },
  {
    chainId: 10,
    name: 'Optimism',
    symbol: 'ETH',
    rpcUrl: 'https://mainnet.optimism.io',
    blockExplorer: 'https://optimistic.etherscan.io',
    logoUrl: '/optimism-logo.png',
    chainType: 'evm',
    features: ['0x-protocol', 'layerzero-endpoint', 'hyperlane-endpoint'],
  },
  
  // ========== LAYERZERO SPECIFIC ==========
  {
    chainId: 30101, // LayerZero Chain ID for Ethereum
    name: 'LayerZero Ethereum',
    symbol: 'ETH',
    rpcUrl: 'https://mainnet.infura.io/v3/',
    blockExplorer: 'https://layerzeroscan.com',
    logoUrl: '/layerzero-logo.png',
    chainType: 'layerzero',
    features: ['cross-chain-bridge', 'omnichain-dapps'],
  },
  {
    chainId: 30109, // LayerZero Chain ID for Polygon
    name: 'LayerZero Polygon',
    symbol: 'MATIC',
    rpcUrl: 'https://polygon-rpc.com',
    blockExplorer: 'https://layerzeroscan.com',
    logoUrl: '/layerzero-logo.png',
    chainType: 'layerzero',
    features: ['cross-chain-bridge', 'omnichain-dapps'],
  },
  
  // ========== COSMOS ECOSYSTEM ==========
  {
    chainId: 'cosmoshub-4',
    name: 'Cosmos Hub',
    symbol: 'ATOM',
    rpcUrl: 'https://cosmos-rpc.quickapi.com',
    blockExplorer: 'https://mintscan.io/cosmos',
    logoUrl: '/cosmos-logo.png',
    chainType: 'cosmos',
    features: ['ibc-transfers', 'staking', 'governance'],
  },
  {
    chainId: 'osmosis-1',
    name: 'Osmosis',
    symbol: 'OSMO',
    rpcUrl: 'https://osmosis-rpc.quickapi.com',
    blockExplorer: 'https://mintscan.io/osmosis',
    logoUrl: '/osmosis-logo.png',
    chainType: 'cosmos',
    features: ['dex', 'liquidity-pools', 'ibc-transfers'],
  },
  {
    chainId: 'juno-1',
    name: 'Juno',
    symbol: 'JUNO',
    rpcUrl: 'https://juno-rpc.quickapi.com',
    blockExplorer: 'https://mintscan.io/juno',
    logoUrl: '/juno-logo.png',
    chainType: 'cosmos',
    features: ['smart-contracts', 'cosmwasm', 'ibc-transfers'],
  },
  
  // ========== SOLANA ==========
  {
    chainId: 'solana-mainnet',
    name: 'Solana',
    symbol: 'SOL',
    rpcUrl: 'https://api.mainnet-beta.solana.com',
    blockExplorer: 'https://explorer.solana.com',
    logoUrl: '/solana-logo.png',
    chainType: 'solana',
    features: ['high-throughput', 'low-fees', 'spl-tokens'],
  },
  {
    chainId: 'solana-devnet',
    name: 'Solana Devnet',
    symbol: 'SOL',
    rpcUrl: 'https://api.devnet.solana.com',
    blockExplorer: 'https://explorer.solana.com?cluster=devnet',
    logoUrl: '/solana-logo.png',
    isTestnet: true,
    chainType: 'solana',
    features: ['high-throughput', 'low-fees', 'spl-tokens'],
  },
  
];

export const DEFAULT_TOKENS_BY_NETWORK: Record<number | string, any[]> = {
  // ========== EVM NETWORKS ==========
  
  // Ethereum Mainnet
  1: [
    {
      address: '0xA0b86a33E6441F8C3eF2C99Ee8A1Fa8f8bAE5B8d',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 1,
      tokenStandard: 'ERC20',
    },
    {
      address: '0xdAC17F958D2ee523a2206206994597C13D831ec7',
      symbol: 'USDT',
      name: 'Tether USD',
      decimals: 6,
      logoURI: '/usdt-logo.png',
      chainId: 1,
      tokenStandard: 'ERC20',
    },
    {
      address: '0x6B175474E89094C44Da98b954EedeAC495271d0F',
      symbol: 'DAI',
      name: 'Dai Stablecoin',
      decimals: 18,
      logoURI: '/dai-logo.png',
      chainId: 1,
      tokenStandard: 'ERC20',
    },
  ],
  
  // Sepolia Testnet (Caesar Token)
  11155111: [
    {
      address: '0x6299744254422aadb6a57183f47eaae1678cf86cc58a0c78dfc4fd2caa3ba2a4',
      symbol: 'CAES',
      name: 'Caesar Token',
      decimals: 18,
      logoURI: '/caesar-logo.png',
      chainId: 11155111,
      tokenStandard: 'ERC20',
    },
    {
      address: '0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9',
      symbol: 'WETH',
      name: 'Wrapped Ether',
      decimals: 18,
      logoURI: '/weth-logo.png',
      chainId: 11155111,
      tokenStandard: 'ERC20',
    },
  ],
  
  // Polygon
  137: [
    {
      address: '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 137,
      tokenStandard: 'ERC20',
    },
    {
      address: '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270',
      symbol: 'WMATIC',
      name: 'Wrapped Matic',
      decimals: 18,
      logoURI: '/wmatic-logo.png',
      chainId: 137,
      tokenStandard: 'ERC20',
    },
  ],
  
  // Arbitrum One
  42161: [
    {
      address: '0xaf88d065e77c8cC2239327C5EDb3A432268e5831',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 42161,
      tokenStandard: 'ERC20',
    },
    {
      address: '0x82aF49447D8a07e3bd95BD0d56f35241523fBab1',
      symbol: 'WETH',
      name: 'Wrapped Ether',
      decimals: 18,
      logoURI: '/weth-logo.png',
      chainId: 42161,
      tokenStandard: 'ERC20',
    },
  ],
  
  // Base
  8453: [
    {
      address: '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 8453,
      tokenStandard: 'ERC20',
    },
  ],
  
  // Optimism
  10: [
    {
      address: '0x7F5c764cBc14f9669B88837ca1490cCa17c31607',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 10,
      tokenStandard: 'ERC20',
    },
  ],
  
  // ========== COSMOS ECOSYSTEM ==========
  
  // Cosmos Hub
  'cosmoshub-4': [
    {
      address: 'uatom',
      symbol: 'ATOM',
      name: 'Cosmos',
      decimals: 6,
      logoURI: '/cosmos-logo.png',
      chainId: 'cosmoshub-4',
      tokenStandard: 'Native',
    },
  ],
  
  // Osmosis
  'osmosis-1': [
    {
      address: 'uosmo',
      symbol: 'OSMO',
      name: 'Osmosis',
      decimals: 6,
      logoURI: '/osmosis-logo.png',
      chainId: 'osmosis-1',
      tokenStandard: 'Native',
    },
    {
      address: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
      symbol: 'ATOM',
      name: 'Cosmos (IBC)',
      decimals: 6,
      logoURI: '/cosmos-logo.png',
      chainId: 'osmosis-1',
      tokenStandard: 'IBC',
    },
  ],
  
  // Juno
  'juno-1': [
    {
      address: 'ujuno',
      symbol: 'JUNO',
      name: 'Juno',
      decimals: 6,
      logoURI: '/juno-logo.png',
      chainId: 'juno-1',
      tokenStandard: 'Native',
    },
  ],
  
  // ========== SOLANA ==========
  
  // Solana Mainnet
  'solana-mainnet': [
    {
      address: 'So11111111111111111111111111111111111111112',
      symbol: 'SOL',
      name: 'Solana',
      decimals: 9,
      logoURI: '/solana-logo.png',
      chainId: 'solana-mainnet',
      tokenStandard: 'Native',
    },
    {
      address: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      symbol: 'USDC',
      name: 'USD Coin',
      decimals: 6,
      logoURI: '/usdc-logo.png',
      chainId: 'solana-mainnet',
      tokenStandard: 'SPL',
    },
    {
      address: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
      symbol: 'USDT',
      name: 'Tether USD',
      decimals: 6,
      logoURI: '/usdt-logo.png',
      chainId: 'solana-mainnet',
      tokenStandard: 'SPL',
    },
  ],
  
  // Solana Devnet
  'solana-devnet': [
    {
      address: 'So11111111111111111111111111111111111111112',
      symbol: 'SOL',
      name: 'Solana',
      decimals: 9,
      logoURI: '/solana-logo.png',
      chainId: 'solana-devnet',
      tokenStandard: 'Native',
    },
  ],
};

export const getNetworkByChainId = (chainId: number | string): Network | undefined => {
  return SUPPORTED_NETWORKS.find(network => network.chainId === chainId);
};

export const isTestnet = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.isTestnet || false;
};

export const getNetworksByType = (chainType: 'evm' | 'cosmos' | 'solana' | 'layerzero'): Network[] => {
  return SUPPORTED_NETWORKS.filter(network => network.chainType === chainType);
};

export const getNetworksByFeature = (feature: string): Network[] => {
  return SUPPORTED_NETWORKS.filter(network => 
    network.features && network.features.includes(feature)
  );
};

export const isEVMNetwork = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.chainType === 'evm' || typeof chainId === 'number';
};

export const isCosmosNetwork = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.chainType === 'cosmos';
};

export const isSolanaNetwork = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.chainType === 'solana';
};

export const isLayerZeroNetwork = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.chainType === 'layerzero';
};


export const supports0xProtocol = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.features?.includes('0x-protocol') || false;
};

export const supportsLayerZero = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.features?.includes('layerzero-endpoint') || 
         network?.features?.includes('cross-chain-bridge') || false;
};

export const supportsIBC = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.features?.includes('ibc-transfers') || false;
};

export const supportsHyperlane = (chainId: number | string): boolean => {
  const network = getNetworkByChainId(chainId);
  return network?.features?.includes('hyperlane-endpoint') || false;
};

/**
 * Get all tokens for a network, including native tokens and bridged tokens
 */
export const getTokensForNetwork = async (chainId: number | string): Promise<Token[]> => {
  // Get static/native tokens
  const nativeTokens = DEFAULT_TOKENS_BY_NETWORK[chainId] || [];
  
  // Get dynamic bridged tokens if it's an EVM network
  if (typeof chainId === 'number') {
    const bridgedTokens = await getBridgedTokensForChain(chainId);
    return [...nativeTokens, ...bridgedTokens];
  }
  
  return nativeTokens;
};

/**
 * Get all available bridged tokens across all networks
 */
export const getAllBridgedTokens = async (): Promise<Record<number, Token[]>> => {
  const evmNetworks = SUPPORTED_NETWORKS.filter(n => n.chainType === 'evm' && typeof n.chainId === 'number');
  const bridgedTokensByChain: Record<number, Token[]> = {};
  
  await Promise.all(
    evmNetworks.map(async (network) => {
      const bridgedTokens = await getBridgedTokensForChain(network.chainId as number);
      bridgedTokensByChain[network.chainId as number] = bridgedTokens;
    })
  );
  
  return bridgedTokensByChain;
};
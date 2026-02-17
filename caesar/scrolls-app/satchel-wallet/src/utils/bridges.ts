// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Token } from '../types';

// Hyperlane Warp Route Registry
export interface HyperlaneWarpRoute {
  chainId: number;
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  originChainId: number;
  originAddress: string;
}

// LayerZero OFT Registry
export interface LayerZeroOFT {
  chainId: number;
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  oftVersion: string;
}

/**
 * Fetch all Hyperlane Warp Routes (bridged tokens)
 * This would hit Hyperlane's registry API in production
 */
export const fetchHyperlaneTokens = async (): Promise<Token[]> => {
  try {
    // In production, this would be: https://api.hyperlane.xyz/v1/warp-routes
    const mockWarpRoutes: HyperlaneWarpRoute[] = [
      {
        chainId: 1,
        address: '0x137C31B2aD931a2F8C3E9F71b38f26E5F1c9A0e1',
        symbol: 'hXRD',
        name: 'Hyperlane Radix',
        decimals: 18,
        originChainId: 'radix-mainnet' as any,
        originAddress: 'resource_rdx1tknxxxxxxxxxradixdxrdxxxxxxxxx009923554798xxxxxxxxxradixdxrd',
      },
      {
        chainId: 137,
        address: '0x2aB8...', 
        symbol: 'hXRD',
        name: 'Hyperlane Radix',
        decimals: 18,
        originChainId: 'radix-mainnet' as any,
        originAddress: 'resource_rdx1tknxxxxxxxxxradixdxrdxxxxxxxxx009923554798xxxxxxxxxradixdxrd',
      },
      {
        chainId: 42161,
        address: '0x3cD9...',
        symbol: 'hATOM',
        name: 'Hyperlane Cosmos',
        decimals: 6,
        originChainId: 'cosmoshub-4' as any,
        originAddress: 'uatom',
      },
    ];

    return mockWarpRoutes.map(route => ({
      address: route.address,
      symbol: route.symbol,
      name: route.name,
      decimals: route.decimals,
      chainId: route.chainId,
      tokenStandard: 'ERC20' as const,
      logoURI: getTokenLogo(route.symbol),
    }));
  } catch (error) {
    console.error('Failed to fetch Hyperlane tokens:', error);
    return [];
  }
};

/**
 * Fetch all LayerZero OFTs (Omni-Fungible Tokens)
 * This would hit LayerZero's registry API in production
 */
export const fetchLayerZeroTokens = async (): Promise<Token[]> => {
  try {
    // In production: https://api.layerzero.network/v1/oft-registry
    const mockOFTs: LayerZeroOFT[] = [
      {
        chainId: 1,
        address: '0x4eF8...',
        symbol: 'lzUSDC',
        name: 'LayerZero USDC',
        decimals: 6,
        oftVersion: 'v2',
      },
      {
        chainId: 137,
        address: '0x5fG9...',
        symbol: 'lzUSDC', 
        name: 'LayerZero USDC',
        decimals: 6,
        oftVersion: 'v2',
      },
      {
        chainId: 42161,
        address: '0x6hH0...',
        symbol: 'lzETH',
        name: 'LayerZero ETH',
        decimals: 18,
        oftVersion: 'v2',
      },
    ];

    return mockOFTs.map(oft => ({
      address: oft.address,
      symbol: oft.symbol,
      name: oft.name,
      decimals: oft.decimals,
      chainId: oft.chainId,
      tokenStandard: 'ERC20' as const,
      logoURI: getTokenLogo(oft.symbol),
    }));
  } catch (error) {
    console.error('Failed to fetch LayerZero tokens:', error);
    return [];
  }
};

/**
 * Get all bridged tokens for a specific chain
 */
export const getBridgedTokensForChain = async (chainId: number): Promise<Token[]> => {
  const [hyperlaneTokens, layerZeroTokens] = await Promise.all([
    fetchHyperlaneTokens(),
    fetchLayerZeroTokens(),
  ]);

  const allBridgedTokens = [...hyperlaneTokens, ...layerZeroTokens];
  return allBridgedTokens.filter(token => token.chainId === chainId);
};

/**
 * Get token logo based on symbol
 */
const getTokenLogo = (symbol: string): string => {
  const logoMap: Record<string, string> = {
    'hXRD': '/radix-logo.png',
    'hATOM': '/cosmos-logo.png', 
    'lzUSDC': '/usdc-logo.png',
    'lzETH': '/eth-logo.png',
  };
  
  return logoMap[symbol] || '/default-token-logo.png';
};

/**
 * Check if a token is bridged via Hyperlane
 */
export const isHyperlaneToken = (symbol: string): boolean => {
  return symbol.startsWith('h') || symbol.includes('Hyperlane');
};

/**
 * Check if a token is bridged via LayerZero
 */
export const isLayerZeroToken = (symbol: string): boolean => {
  return symbol.startsWith('lz') || symbol.includes('LayerZero');
};

/**
 * Get bridge info for a token
 */
export const getBridgeInfo = (token: Token) => {
  if (isHyperlaneToken(token.symbol)) {
    return { protocol: 'Hyperlane', type: 'Warp Route' };
  }
  if (isLayerZeroToken(token.symbol)) {
    return { protocol: 'LayerZero', type: 'OFT' };
  }
  return null;
};
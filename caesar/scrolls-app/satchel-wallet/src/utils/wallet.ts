// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { createConfig, http } from 'wagmi';
import { mainnet, sepolia, polygon, arbitrum, base, optimism } from 'wagmi/chains';
import { walletConnect, injected, coinbaseWallet } from 'wagmi/connectors';

// WalletConnect Project ID - Get from https://cloud.walletconnect.com
const projectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || 'demo-project-id-replace-with-real';

export const config = createConfig({
  chains: [mainnet, sepolia, polygon, arbitrum, base, optimism],
  connectors: [
    injected({
      target: 'metaMask',
    }),
    walletConnect({
      projectId,
      metadata: {
        name: 'Satchel Wallet',
        description: 'Multi-Chain Caesar Token Wallet',
        url: 'https://satchel.wallet',
        icons: ['https://satchel.wallet/icon.png'],
      },
    }),
    coinbaseWallet({
      appName: 'Satchel Wallet',
      appLogoUrl: 'https://satchel.wallet/icon.png',
    }),
  ],
  transports: {
    [mainnet.id]: http(),
    [sepolia.id]: http(),
    [polygon.id]: http(),
    [arbitrum.id]: http(),
    [base.id]: http(),
    [optimism.id]: http(),
  },
});

export type ConnectedWallet = {
  address: string;
  chainId: number;
  connector: string;
  isConnected: boolean;
};

/**
 * Format wallet address for display
 */
export const formatAddress = (address: string): string => {
  if (!address) return '';
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

/**
 * Get network name from chain ID
 */
export const getNetworkName = (chainId: number): string => {
  const networks: Record<number, string> = {
    1: 'Ethereum',
    11155111: 'Sepolia',
    137: 'Polygon',
    42161: 'Arbitrum',
    8453: 'Base',
    10: 'Optimism',
  };
  return networks[chainId] || `Chain ${chainId}`;
};

/**
 * Get supported chain IDs
 */
export const getSupportedChainIds = (): number[] => {
  return [1, 11155111, 137, 42161, 8453, 10];
};

/**
 * Check if chain is supported
 */
export const isSupportedChain = (chainId: number): boolean => {
  return getSupportedChainIds().includes(chainId);
};
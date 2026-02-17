// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface Network {
  chainId: number | string; // Support both EVM chainIds and non-EVM identifiers
  name: string;
  symbol: string;
  rpcUrl: string;
  blockExplorer: string;
  logoUrl?: string;
  isTestnet?: boolean;
  chainType?: 'evm' | 'cosmos' | 'solana' | 'layerzero'; // Chain type identifier
  features?: string[]; // Special features like ['0x-protocol', 'layerzero-bridge']
}

export interface Token {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  logoURI?: string;
  balance?: string;
  balanceUSD?: string;
  chainId: number | string;
  tokenStandard?: 'ERC20' | 'SPL' | 'IBC' | 'Native'; // Token standard
  priceUSD?: string;
  priceChange24h?: number;
  lastPriceUpdate?: number;
}

export interface Transaction {
  hash: string;
  from: string;
  to: string;
  value: string;
  timestamp: number;
  status: 'pending' | 'confirmed' | 'failed';
  chainId: number | string;
  gasUsed?: string;
  gasPrice?: string;
  tokenSymbol?: string;
  contractAddress?: string;
  type?: 'normal' | 'internal' | 'erc20' | 'cosmos' | 'solana' | 'bridge';
  direction?: 'sent' | 'received' | 'swap';
  method?: string;
  blockNumber?: number;
  transactionIndex?: number;
  confirmations?: number;
}

export interface WalletAccount {
  address: string;
  name: string;
  balance: string;
  balanceUSD: string;
  tokens: Token[];
  isConnected: boolean;
}

export interface WalletState {
  accounts: WalletAccount[];
  currentAccount: WalletAccount | null;
  networks: Network[];
  currentNetwork: Network | null;
  isLocked: boolean;
  isConnected: boolean;
}

export interface NFT {
  tokenId: string;
  contractAddress: string;
  name: string;
  description: string;
  image: string;
  collection: string;
  chainId: number;
}
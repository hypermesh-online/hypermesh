// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Transaction, Network } from '../types';
import { SUPPORTED_NETWORKS } from './networks';

export interface TransactionHistoryProvider {
  getTransactions(address: string, chainId: number | string): Promise<Transaction[]>;
}

// EVM-compatible blockchain transaction provider using Etherscan-like APIs
class EVMTransactionProvider implements TransactionHistoryProvider {
  private apiKeys: Record<number, string> = {
    1: import.meta.env.VITE_ETHERSCAN_API_KEY || '',
    11155111: import.meta.env.VITE_ETHERSCAN_API_KEY || '', // Sepolia
    137: import.meta.env.VITE_POLYGONSCAN_API_KEY || '',
    42161: import.meta.env.VITE_ARBISCAN_API_KEY || '',
    8453: import.meta.env.VITE_BASESCAN_API_KEY || '',
    10: import.meta.env.VITE_OPTIMISMSCAN_API_KEY || '',
  };

  private apiUrls: Record<number, string> = {
    1: 'https://api.etherscan.io/api',
    11155111: 'https://api-sepolia.etherscan.io/api',
    137: 'https://api.polygonscan.com/api',
    42161: 'https://api.arbiscan.io/api',
    8453: 'https://api.basescan.org/api',
    10: 'https://api-optimistic.etherscan.io/api',
  };

  async getTransactions(address: string, chainId: number): Promise<Transaction[]> {
    const apiUrl = this.apiUrls[chainId];
    const apiKey = this.apiKeys[chainId];

    if (!apiUrl) {
      console.warn(`No API URL configured for chain ${chainId}`);
      return [];
    }

    try {
      // Fetch both normal and internal transactions
      const [normalTxs, internalTxs, erc20Txs] = await Promise.all([
        this.fetchNormalTransactions(apiUrl, address, apiKey),
        this.fetchInternalTransactions(apiUrl, address, apiKey),
        this.fetchERC20Transactions(apiUrl, address, apiKey),
      ]);

      // Combine and sort all transactions
      const allTransactions = [...normalTxs, ...internalTxs, ...erc20Txs];
      return allTransactions
        .sort((a, b) => b.timestamp - a.timestamp)
        .slice(0, 100); // Limit to 100 most recent
    } catch (error) {
      console.error(`Error fetching transactions for chain ${chainId}:`, error);
      return [];
    }
  }

  private async fetchNormalTransactions(apiUrl: string, address: string, apiKey: string): Promise<Transaction[]> {
    const url = `${apiUrl}?module=account&action=txlist&address=${address}&startblock=0&endblock=99999999&page=1&offset=50&sort=desc&apikey=${apiKey}`;
    
    const response = await fetch(url);
    const data = await response.json();

    if (data.status !== '1' || !data.result) return [];

    return data.result.map((tx: any) => ({
      hash: tx.hash,
      from: tx.from,
      to: tx.to || '',
      value: tx.value,
      timestamp: parseInt(tx.timeStamp) * 1000,
      status: tx.txreceipt_status === '1' ? 'confirmed' : 'failed',
      chainId: parseInt(tx.networkID) || 0,
      gasUsed: tx.gasUsed,
      gasPrice: tx.gasPrice,
      tokenSymbol: 'ETH', // Native token
      type: 'normal'
    }));
  }

  private async fetchInternalTransactions(apiUrl: string, address: string, apiKey: string): Promise<Transaction[]> {
    const url = `${apiUrl}?module=account&action=txlistinternal&address=${address}&startblock=0&endblock=99999999&page=1&offset=50&sort=desc&apikey=${apiKey}`;
    
    try {
      const response = await fetch(url);
      const data = await response.json();

      if (data.status !== '1' || !data.result) return [];

      return data.result.map((tx: any) => ({
        hash: tx.hash,
        from: tx.from,
        to: tx.to || '',
        value: tx.value,
        timestamp: parseInt(tx.timeStamp) * 1000,
        status: tx.isError === '0' ? 'confirmed' : 'failed',
        chainId: 0,
        gasUsed: tx.gas,
        gasPrice: '0',
        tokenSymbol: 'ETH',
        type: 'internal'
      }));
    } catch (error) {
      console.warn('Internal transactions not available or failed:', error);
      return [];
    }
  }

  private async fetchERC20Transactions(apiUrl: string, address: string, apiKey: string): Promise<Transaction[]> {
    const url = `${apiUrl}?module=account&action=tokentx&address=${address}&startblock=0&endblock=99999999&page=1&offset=50&sort=desc&apikey=${apiKey}`;
    
    try {
      const response = await fetch(url);
      const data = await response.json();

      if (data.status !== '1' || !data.result) return [];

      return data.result.map((tx: any) => ({
        hash: tx.hash,
        from: tx.from,
        to: tx.to || '',
        value: tx.value,
        timestamp: parseInt(tx.timeStamp) * 1000,
        status: 'confirmed', // ERC20 transactions are confirmed if they appear
        chainId: parseInt(tx.networkID) || 0,
        gasUsed: tx.gasUsed,
        gasPrice: tx.gasPrice,
        tokenSymbol: tx.tokenSymbol,
        contractAddress: tx.contractAddress,
        type: 'erc20'
      }));
    } catch (error) {
      console.warn('ERC20 transactions not available or failed:', error);
      return [];
    }
  }
}

// Cosmos-based blockchain transaction provider
class CosmosTransactionProvider implements TransactionHistoryProvider {
  private apiUrls: Record<string, string> = {
    'cosmoshub-4': 'https://rest.cosmos.directory/cosmoshub',
    'osmosis-1': 'https://rest.cosmos.directory/osmosis',
    'juno-1': 'https://rest.cosmos.directory/juno',
  };

  async getTransactions(address: string, chainId: string): Promise<Transaction[]> {
    const apiUrl = this.apiUrls[chainId];
    
    if (!apiUrl) {
      console.warn(`No API URL configured for Cosmos chain ${chainId}`);
      return [];
    }

    try {
      // Fetch both sent and received transactions
      const [sentTxs, receivedTxs] = await Promise.all([
        this.fetchCosmosTransactions(apiUrl, address, 'message.sender'),
        this.fetchCosmosTransactions(apiUrl, address, 'transfer.recipient'),
      ]);

      const allTransactions = [...sentTxs, ...receivedTxs];
      return allTransactions
        .sort((a, b) => b.timestamp - a.timestamp)
        .slice(0, 50);
    } catch (error) {
      console.error(`Error fetching Cosmos transactions for ${chainId}:`, error);
      return [];
    }
  }

  private async fetchCosmosTransactions(apiUrl: string, address: string, eventType: string): Promise<Transaction[]> {
    const url = `${apiUrl}/cosmos/tx/v1beta1/txs?events=${eventType}='${address}'&limit=25`;
    
    const response = await fetch(url);
    const data = await response.json();

    if (!data.tx_responses) return [];

    return data.tx_responses.map((tx: any) => ({
      hash: tx.txhash,
      from: this.extractSender(tx),
      to: this.extractRecipient(tx),
      value: this.extractAmount(tx),
      timestamp: new Date(tx.timestamp).getTime(),
      status: tx.code === 0 ? 'confirmed' : 'failed',
      chainId: tx.chain_id || '',
      gasUsed: tx.gas_used,
      gasPrice: '0',
      tokenSymbol: this.extractTokenSymbol(tx),
      type: 'cosmos'
    }));
  }

  private extractSender(tx: any): string {
    try {
      return tx.tx.body.messages[0]?.from_address || '';
    } catch {
      return '';
    }
  }

  private extractRecipient(tx: any): string {
    try {
      return tx.tx.body.messages[0]?.to_address || '';
    } catch {
      return '';
    }
  }

  private extractAmount(tx: any): string {
    try {
      const amount = tx.tx.body.messages[0]?.amount?.[0]?.amount;
      return amount || '0';
    } catch {
      return '0';
    }
  }

  private extractTokenSymbol(tx: any): string {
    try {
      const denom = tx.tx.body.messages[0]?.amount?.[0]?.denom;
      // Convert denoms to human-readable symbols
      const denomMap: Record<string, string> = {
        'uatom': 'ATOM',
        'uosmo': 'OSMO',
        'ujuno': 'JUNO',
      };
      return denomMap[denom] || denom || 'UNKNOWN';
    } catch {
      return 'UNKNOWN';
    }
  }
}

// Solana transaction provider
class SolanaTransactionProvider implements TransactionHistoryProvider {
  private rpcUrl = 'https://api.mainnet-beta.solana.com';

  async getTransactions(address: string, _chainId: string): Promise<Transaction[]> {
    try {
      const response = await fetch(this.rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'getSignaturesForAddress',
          params: [
            address,
            { limit: 50 }
          ]
        })
      });

      const data = await response.json();
      if (!data.result) return [];

      // Get transaction details for each signature
      const txPromises = data.result.slice(0, 20).map((sig: any) => 
        this.getTransactionDetails(sig.signature)
      );

      const transactions = await Promise.all(txPromises);
      return transactions.filter(tx => tx !== null) as Transaction[];
    } catch (error) {
      console.error('Error fetching Solana transactions:', error);
      return [];
    }
  }

  private async getTransactionDetails(signature: string): Promise<Transaction | null> {
    try {
      const response = await fetch(this.rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'getTransaction',
          params: [signature, { encoding: 'json' }]
        })
      });

      const data = await response.json();
      if (!data.result) return null;

      const tx = data.result;
      const meta = tx.meta;
      const message = tx.transaction.message;

      return {
        hash: signature,
        from: message.accountKeys[0] || '',
        to: message.accountKeys[1] || '',
        value: (meta.preBalances[0] - meta.postBalances[0]).toString(),
        timestamp: (tx.blockTime || 0) * 1000,
        status: meta.err ? 'failed' : 'confirmed',
        chainId: 'solana',
        gasUsed: meta.fee?.toString() || '0',
        gasPrice: '0',
        tokenSymbol: 'SOL',
        type: 'solana'
      };
    } catch (error) {
      console.warn(`Failed to get Solana transaction details for ${signature}:`, error);
      return null;
    }
  }
}

// Transaction history aggregator
export class TransactionHistoryAggregator {
  private providers: Map<string, TransactionHistoryProvider> = new Map();

  constructor() {
    this.providers.set('evm', new EVMTransactionProvider());
    this.providers.set('cosmos', new CosmosTransactionProvider());
    this.providers.set('solana', new SolanaTransactionProvider());
  }

  async getTransactionHistory(address: string, network: Network): Promise<Transaction[]> {
    const chainType = network.chainType || 'evm';
    const provider = this.providers.get(chainType);

    if (!provider) {
      console.warn(`No transaction provider for chain type: ${chainType}`);
      return this.getMockTransactions(address, network);
    }

    try {
      const transactions = await provider.getTransactions(address, network.chainId);
      
      // If no real transactions, return mock data for development
      if (transactions.length === 0) {
        return this.getMockTransactions(address, network);
      }

      return transactions;
    } catch (error) {
      console.error(`Error fetching transaction history:`, error);
      return this.getMockTransactions(address, network);
    }
  }

  // Enhanced mock data for development and testing
  private getMockTransactions(address: string, network: Network): Transaction[] {
    const now = Date.now();
    const mockTransactions: Transaction[] = [
      {
        hash: '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
        from: address,
        to: '0x742d35Cc6B88A82c36E7E85D0fd3Da31B2E3e7D9',
        value: '100000000000000000000', // 100 tokens
        timestamp: now - 2 * 60 * 60 * 1000, // 2 hours ago
        status: 'confirmed',
        chainId: typeof network.chainId === 'number' ? network.chainId : 0,
        gasUsed: '21000',
        gasPrice: '20000000000',
        tokenSymbol: 'CAES',
        type: 'erc20'
      },
      {
        hash: '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
        from: '0x8765432109876543210987654321098765432109',
        to: address,
        value: '50000000000000000', // 0.05 ETH
        timestamp: now - 24 * 60 * 60 * 1000, // 1 day ago
        status: 'confirmed',
        chainId: typeof network.chainId === 'number' ? network.chainId : 0,
        gasUsed: '21000',
        gasPrice: '25000000000',
        tokenSymbol: network.symbol,
        type: 'normal'
      },
      {
        hash: '0x9876543210abcdef9876543210abcdef9876543210abcdef9876543210abcdef',
        from: address,
        to: '0xAe0DfF19f44D3544139d900a3f9f6c03C6764538', // DEX Factory
        value: '50000000000000000000', // 50 CAES
        timestamp: now - 3 * 24 * 60 * 60 * 1000, // 3 days ago
        status: 'confirmed',
        chainId: typeof network.chainId === 'number' ? network.chainId : 0,
        gasUsed: '85000',
        gasPrice: '22000000000',
        tokenSymbol: 'CAES',
        type: 'erc20'
      },
      {
        hash: '0xdef123456789abcdef123456789abcdef123456789abcdef123456789abcdef',
        from: address,
        to: '0x1234567890123456789012345678901234567890',
        value: '1000000000000000000', // 1 ETH
        timestamp: now - 7 * 24 * 60 * 60 * 1000, // 1 week ago
        status: 'failed',
        chainId: typeof network.chainId === 'number' ? network.chainId : 0,
        gasUsed: '21000',
        gasPrice: '30000000000',
        tokenSymbol: network.symbol,
        type: 'normal'
      }
    ];

    return mockTransactions;
  }

  // Get transaction history for all supported networks
  async getAllTransactionHistory(address: string): Promise<Record<string, Transaction[]>> {
    const history: Record<string, Transaction[]> = {};

    const promises = SUPPORTED_NETWORKS.map(async (network) => {
      const transactions = await this.getTransactionHistory(address, network);
      return { networkName: network.name, transactions };
    });

    const results = await Promise.all(promises);
    
    results.forEach(({ networkName, transactions }) => {
      history[networkName] = transactions;
    });

    return history;
  }
}

// Singleton instance
export const transactionHistory = new TransactionHistoryAggregator();
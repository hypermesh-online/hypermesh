// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { writable, derived } from 'svelte/store';
import { ethers } from 'ethers';

// Wallet connection state
export const wallet_connected = writable(false);
export const wallet_address = writable('');
export const wallet_provider = writable(null);

// Token balances and data
export const token_balances = writable(new Map());
export const transaction_history = writable([]);
export const network_status = writable({
  chain_id: null,
  network_name: 'Disconnected',
  connected: false
});

// Caesar token specific data
export const caesar_token_data = writable({
  balance: '0.00',
  demurrage_rate: 0.025, // 2.5% annual
  last_activity: Date.now(),
  accumulated_demurrage: 0,
  effective_balance: '0.00'
});

// UI state
export const loading_states = writable({
  connecting: false,
  sending: false,
  loading_balance: false,
  loading_history: false
});

// Main wallet store with actions
function createWalletStore() {
  const { subscribe, set, update } = writable({
    connected: false,
    address: '',
    balance: '0.00',
    provider: null
  });

  return {
    subscribe,
    
    // Connect wallet (MetaMask, WalletConnect, etc.)
    async connectWallet() {
      try {
        loading_states.update(state => ({ ...state, connecting: true }));
        
        if (typeof window.ethereum !== 'undefined') {
          const provider = new ethers.BrowserProvider(window.ethereum);
          const accounts = await window.ethereum.request({ 
            method: 'eth_requestAccounts' 
          });
          
          if (accounts.length > 0) {
            const address = accounts[0];
            const network = await provider.getNetwork();
            
            // Update stores
            wallet_connected.set(true);
            wallet_address.set(address);
            wallet_provider.set(provider);
            network_status.set({
              chain_id: Number(network.chainId),
              network_name: network.name,
              connected: true
            });
            
            // Load initial data
            await this.loadTokenBalances();
            await this.loadTransactionHistory();
            
            set({
              connected: true,
              address,
              provider
            });
          }
        } else {
          throw new Error('No Ethereum wallet found. Please install MetaMask.');
        }
      } catch (error) {
        console.error('Wallet connection failed:', error);
        throw error;
      } finally {
        loading_states.update(state => ({ ...state, connecting: false }));
      }
    },

    // Disconnect wallet
    async disconnectWallet() {
      wallet_connected.set(false);
      wallet_address.set('');
      wallet_provider.set(null);
      network_status.set({
        chain_id: null,
        network_name: 'Disconnected',
        connected: false
      });
      
      set({
        connected: false,
        address: '',
        provider: null
      });
    },

    // Load token balances
    async loadTokenBalances() {
      try {
        loading_states.update(state => ({ ...state, loading_balance: true }));
        
        const provider = await this.getProvider();
        const address = await this.getAddress();
        
        if (!provider || !address) return;

        // Get ETH balance
        const ethBalance = await provider.getBalance(address);
        const formattedEthBalance = ethers.formatEther(ethBalance);
        
        // Get Caesar token balance (if contract exists)
        const caesarBalance = await this.getCaesarBalance(provider, address);
        
        token_balances.update(balances => {
          const newBalances = new Map(balances);
          newBalances.set('ETH', formattedEthBalance);
          newBalances.set('CAESAR', caesarBalance);
          return newBalances;
        });

        // Update Caesar specific data with demurrage calculation
        this.updateCaesarTokenData(caesarBalance);
        
      } catch (error) {
        console.error('Failed to load token balances:', error);
      } finally {
        loading_states.update(state => ({ ...state, loading_balance: false }));
      }
    },

    // Load transaction history
    async loadTransactionHistory() {
      try {
        loading_states.update(state => ({ ...state, loading_history: true }));
        
        const address = await this.getAddress();
        if (!address) return;
        
        // TODO: Implement actual transaction history fetching
        // For now, use mock data
        const mockTransactions = [
          {
            hash: '0x1234...5678',
            from: address,
            to: '0xabcd...efgh',
            value: '10.00',
            token: 'CAESAR',
            timestamp: Date.now() - 86400000,
            status: 'confirmed',
            type: 'send'
          },
          {
            hash: '0x8765...4321',
            from: '0xijkl...mnop',
            to: address,
            value: '25.50',
            token: 'CAESAR',
            timestamp: Date.now() - 172800000,
            status: 'confirmed',
            type: 'receive'
          }
        ];
        
        transaction_history.set(mockTransactions);
        
      } catch (error) {
        console.error('Failed to load transaction history:', error);
      } finally {
        loading_states.update(state => ({ ...state, loading_history: false }));
      }
    },

    // Caesar token specific methods
    async getCaesarBalance(provider, address) {
      // TODO: Implement actual Caesar token contract interaction
      // For now return mock balance
      return '150.75';
    },

    updateCaesarTokenData(balance) {
      caesar_token_data.update(data => {
        const now = Date.now();
        const timeElapsed = (now - data.last_activity) / (1000 * 60 * 60 * 24 * 365); // years
        const demurrageAmount = parseFloat(balance) * data.demurrage_rate * timeElapsed;
        const effectiveBalance = Math.max(0, parseFloat(balance) - demurrageAmount);
        
        return {
          ...data,
          balance,
          accumulated_demurrage: demurrageAmount,
          effective_balance: effectiveBalance.toFixed(2),
          last_activity: now
        };
      });
    },

    // Helper methods
    async getProvider() {
      return new Promise((resolve) => {
        wallet_provider.subscribe(provider => {
          resolve(provider);
        })();
      });
    },

    async getAddress() {
      return new Promise((resolve) => {
        wallet_address.subscribe(address => {
          resolve(address);
        })();
      });
    },

    // Load wallet data on initialization
    async loadWalletData() {
      // Check if wallet was previously connected
      if (typeof window.ethereum !== 'undefined') {
        const accounts = await window.ethereum.request({ method: 'eth_accounts' });
        if (accounts.length > 0) {
          await this.connectWallet();
        }
      }
    }
  };
}

export const walletStore = createWalletStore();

// Derived stores for computed values
export const formatted_caesar_balance = derived(
  caesar_token_data,
  $caesar => `${$caesar.effective_balance} CAESAR`
);

export const demurrage_warning = derived(
  caesar_token_data,
  $caesar => $caesar.accumulated_demurrage > 1.0
);
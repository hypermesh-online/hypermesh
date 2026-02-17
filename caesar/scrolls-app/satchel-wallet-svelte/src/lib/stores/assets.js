// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { writable, derived } from 'svelte/store';
import { ethers } from 'ethers';

// Universal Asset Management Store for HyperMesh Integration
// This replaces the crypto-focused wallet store with a comprehensive real-world economy manager

// Asset Types following HyperMesh ecosystem patterns
export const ASSET_TYPES = {
  // Physical Assets
  REAL_ESTATE: 'real_estate',
  VEHICLE: 'vehicle', 
  EQUIPMENT: 'equipment',
  INVENTORY: 'inventory',
  
  // Digital Assets
  TOKEN: 'token',
  CONTRACT: 'contract',
  SERVICE: 'service',
  DATA: 'data',
  
  // Infrastructure Assets (HyperMesh resources)
  CPU: 'cpu',
  GPU: 'gpu', 
  MEMORY: 'memory',
  STORAGE: 'storage',
  NETWORK: 'network'
};

// Privacy Levels (from HyperMesh ecosystem)
export const PRIVACY_LEVELS = {
  PRIVATE: 'private',           // Internal network only
  PRIVATE_NETWORK: 'private_network', // Specific networks/groups
  P2P: 'p2p',                  // Trusted peer sharing
  PUBLIC_NETWORK: 'public_network',   // Specific public networks  
  FULL_PUBLIC: 'full_public'    // Maximum CAESAR rewards, full HyperMesh node
};

// Asset connection state (replacing wallet_connected)
export const asset_manager_connected = writable(false);
export const user_identity = writable('');
export const hypermesh_provider = writable(null);

// Universal asset registry
export const asset_registry = writable(new Map());
export const asset_history = writable([]);
export const consensus_status = writable({
  pos_validated: false,   // Proof of Space
  post_validated: false,  // Proof of Stake  
  powk_validated: false,  // Proof of Work
  potm_validated: false,  // Proof of Time
  consensus_score: 0
});

// Active contracts and services
export const active_contracts = writable([]);
export const service_subscriptions = writable([]);
export const hypermesh_allocations = writable({
  cpu_shared: 0,      // Percentage of CPU shared
  gpu_shared: 0,      // Percentage of GPU shared  
  memory_shared: 0,   // Percentage of RAM shared
  storage_shared: 0,  // Percentage of storage shared
  earnings_rate: 0    // CAESAR tokens earned per hour
});

// Caesar economic integration
export const caesar_economy = writable({
  balance: '0.00',
  demurrage_rate: 0.025,
  utility_payments: [],    // Real utility bills paid with Caesar
  asset_rewards: 0,        // Earnings from sharing physical assets
  service_costs: 0,        // Costs for real services consumed
  anti_speculation_score: 100 // 100 = pure utility usage, 0 = speculation
});

// UI state  
export const loading_states = writable({
  connecting: false,
  loading_assets: false,
  processing_contract: false,
  validating_consensus: false,
  updating_allocation: false
});

// Privacy configuration per user
export const privacy_config = writable({
  default_level: PRIVACY_LEVELS.PRIVATE,
  asset_sharing: {
    vehicle: PRIVACY_LEVELS.P2P,
    equipment: PRIVACY_LEVELS.PRIVATE_NETWORK,
    storage: PRIVACY_LEVELS.PUBLIC_NETWORK,
    cpu: PRIVACY_LEVELS.FULL_PUBLIC
  },
  consent_duration: 24, // hours
  max_concurrent_users: 3,
  require_consensus: true // All four proofs required
});

// Main universal asset store
function createAssetStore() {
  const { subscribe, set, update } = writable({
    connected: false,
    identity: '',
    provider: null,
    total_assets: 0
  });

  return {
    subscribe,
    
    // Connect to HyperMesh ecosystem (replacing connectWallet)
    async connectAssetManager() {
      try {
        loading_states.update(state => ({ ...state, connecting: true }));
        
        // Initialize HyperMesh connection (will integrate with TrustChain/STOQ)
        if (typeof window.ethereum !== 'undefined') {
          const provider = new ethers.BrowserProvider(window.ethereum);
          const accounts = await window.ethereum.request({ 
            method: 'eth_requestAccounts' 
          });
          
          if (accounts.length > 0) {
            const identity = accounts[0]; // Will be HyperMesh identity
            
            // Update connection state
            asset_manager_connected.set(true);
            user_identity.set(identity);
            hypermesh_provider.set(provider);
            
            // Load all asset types
            await this.loadUniversalAssets();
            await this.loadActiveContracts();
            await this.loadServiceSubscriptions();
            await this.loadHyperMeshAllocations();
            
            set({
              connected: true,
              identity,
              provider,
              total_assets: 0 // Will be calculated
            });
          }
        } else {
          throw new Error('HyperMesh connection not available. Please install wallet extension.');
        }
      } catch (error) {
        console.error('Asset manager connection failed:', error);
        throw error;
      } finally {
        loading_states.update(state => ({ ...state, connecting: false }));
      }
    },

    // Disconnect from HyperMesh ecosystem
    async disconnectAssetManager() {
      asset_manager_connected.set(false);
      user_identity.set('');
      hypermesh_provider.set(null);
      asset_registry.set(new Map());
      active_contracts.set([]);
      service_subscriptions.set([]);
      
      consensus_status.set({
        pos_validated: false,
        post_validated: false,
        powk_validated: false,
        potm_validated: false,
        consensus_score: 0
      });
      
      set({
        connected: false,
        identity: '',
        provider: null,
        total_assets: 0
      });
    },

    // Load all asset types (physical + digital + infrastructure)
    async loadUniversalAssets() {
      try {
        loading_states.update(state => ({ ...state, loading_assets: true }));
        
        const identity = await this.getIdentity();
        if (!identity) return;

        // Load different asset categories
        const physicalAssets = await this.loadPhysicalAssets(identity);
        const digitalAssets = await this.loadDigitalAssets(identity);
        const infrastructureAssets = await this.loadInfrastructureAssets(identity);
        
        // Combine all assets into registry
        const allAssets = new Map();
        
        // Physical Assets
        physicalAssets.forEach(asset => allAssets.set(asset.id, asset));
        digitalAssets.forEach(asset => allAssets.set(asset.id, asset));  
        infrastructureAssets.forEach(asset => allAssets.set(asset.id, asset));
        
        asset_registry.set(allAssets);
        
        // Update Caesar economy based on asset activity
        this.updateCaesarEconomy();
        
      } catch (error) {
        console.error('Failed to load universal assets:', error);
      } finally {
        loading_states.update(state => ({ ...state, loading_assets: false }));
      }
    },

    // Load physical assets (houses, cars, equipment)
    async loadPhysicalAssets(identity) {
      // TODO: Integrate with real property/vehicle registries via HyperMesh
      return [
        {
          id: 'house-001',
          type: ASSET_TYPES.REAL_ESTATE,
          name: '123 Main Street Apartment',
          value: 285000,
          currency: 'USD',
          privacy_level: PRIVACY_LEVELS.PRIVATE,
          consensus_proofs: {
            pos: true,  // Property deed location verified
            post: true, // Ownership stake verified
            powk: false, // No computational work required
            potm: true  // Purchase timestamp validated
          },
          metadata: {
            address: '123 Main Street, City, State',
            bedrooms: 2,
            bathrooms: 1,
            square_feet: 950,
            mortgage_balance: 180000,
            monthly_payment: 1450,
            last_payment: '2024-09-01'
          }
        },
        {
          id: 'vehicle-001', 
          type: ASSET_TYPES.VEHICLE,
          name: '2019 Toyota Camry',
          value: 18500,
          currency: 'USD', 
          privacy_level: PRIVACY_LEVELS.P2P,
          consensus_proofs: {
            pos: true,  // Vehicle location tracked
            post: true, // Title ownership verified
            powk: false, // No computational work
            potm: true  // Registration timestamp
          },
          sharing_config: {
            available_hours: 8, // Hours per day available for sharing
            hourly_rate: 12,    // CAESAR tokens per hour
            max_distance: 50,   // Miles from owner
            requires_deposit: true
          },
          metadata: {
            vin: 'ABC123XYZ789',
            mileage: 67500,
            insurance_expires: '2025-03-15',
            last_maintenance: '2024-08-15'
          }
        }
      ];
    },

    // Load digital assets (tokens, contracts, data)
    async loadDigitalAssets(identity) {
      return [
        {
          id: 'caesar-token',
          type: ASSET_TYPES.TOKEN,
          name: 'CAESAR Token',
          value: 150.75,
          currency: 'CAESAR',
          privacy_level: PRIVACY_LEVELS.PUBLIC_NETWORK,
          consensus_proofs: {
            pos: true,  // Blockchain location verified
            post: true, // Ownership stake verified  
            powk: true, // Mining/validation work
            potm: true  // Transaction timestamps
          },
          demurrage: {
            rate: 0.025,
            accumulated: 2.31,
            effective_balance: 148.44
          }
        },
        {
          id: 'employment-contract-001',
          type: ASSET_TYPES.CONTRACT,
          name: 'Software Engineer Employment',
          value: 85000,
          currency: 'USD',
          privacy_level: PRIVACY_LEVELS.PRIVATE_NETWORK,
          consensus_proofs: {
            pos: true,  // Contract stored location
            post: true, // Employment agreement verified
            powk: false, // No computational work
            potm: true  // Contract execution timestamps
          },
          execution_status: 'active',
          next_payment: '2024-09-30',
          metadata: {
            employer: 'TechCorp Inc',
            start_date: '2023-01-15',
            annual_salary: 85000,
            benefits_value: 12000,
            remote_work: true
          }
        }
      ];
    },

    // Load HyperMesh infrastructure assets (CPU, GPU, RAM, Storage)  
    async loadInfrastructureAssets(identity) {
      return [
        {
          id: 'cpu-resource',
          type: ASSET_TYPES.CPU,
          name: 'AMD Ryzen 9 5900X (12 cores)',
          value: 0, // Value in compute hours available
          currency: 'COMPUTE_HOURS',
          privacy_level: PRIVACY_LEVELS.FULL_PUBLIC,
          consensus_proofs: {
            pos: true,  // Physical CPU location
            post: true, // Ownership of hardware
            powk: true, // Computational work capability
            potm: true  // Usage timestamps
          },
          allocation: {
            total_cores: 12,
            shared_cores: 8,    // 67% shared for CAESAR earnings
            reserved_cores: 4,  // Reserved for owner
            current_users: 2,
            earnings_per_hour: 0.75 // CAESAR per hour
          }
        },
        {
          id: 'storage-resource',
          type: ASSET_TYPES.STORAGE,
          name: 'NVMe SSD Pool (2TB)',
          value: 0,
          currency: 'STORAGE_GB_HOURS',
          privacy_level: PRIVACY_LEVELS.PUBLIC_NETWORK,
          consensus_proofs: {
            pos: true,  // Storage device location
            post: true, // Ownership verified
            powk: false, // No computational work for storage
            potm: true  // Access timestamps
          },
          allocation: {
            total_space: 2048, // GB
            shared_space: 1500, // GB available for sharing
            used_space: 450,   // GB currently used by others
            encryption: 'KYBER_1024', // Quantum-resistant
            earnings_per_gb_hour: 0.001 // CAESAR per GB per hour
          }
        }
      ];
    },

    // Load active contracts (employment, utilities, services)
    async loadActiveContracts() {
      const mockContracts = [
        {
          id: 'electric-utility-001',
          type: 'utility',
          provider: 'Metro Electric Co',
          service: 'Electricity',
          monthly_cost: 95.50,
          payment_method: 'CAESAR',
          auto_pay: true,
          next_due: '2024-10-15',
          last_payment: {
            amount: 87.25,
            date: '2024-09-15',
            caesar_rate: 1.15 // USD per CAESAR at time of payment
          }
        },
        {
          id: 'internet-service-001', 
          type: 'telecommunications',
          provider: 'FastNet ISP',
          service: 'Internet (1Gbps)',
          monthly_cost: 79.99,
          payment_method: 'CAESAR',
          auto_pay: true,
          next_due: '2024-10-10',
          usage_sharing: {
            bandwidth_shared: 500, // Mbps shared via HyperMesh
            earnings_per_month: 15.0 // CAESAR earned from sharing
          }
        }
      ];
      
      active_contracts.set(mockContracts);
    },

    // Load service subscriptions
    async loadServiceSubscriptions() {
      const mockServices = [
        {
          id: 'cloud-storage-001',
          provider: 'HyperMesh Storage Network',
          service: 'Distributed Cloud Storage',
          plan: '500GB Encrypted',
          monthly_cost: 8.99,
          payment_method: 'CAESAR',
          features: ['Quantum-resistant encryption', 'Global CDN', 'Automatic sharding'],
          usage: {
            used: 347, // GB
            available: 500 // GB
          }
        }
      ];
      
      service_subscriptions.set(mockServices);
    },

    // Load HyperMesh resource allocations
    async loadHyperMeshAllocations() {
      const allocation = {
        cpu_shared: 67,     // 67% of CPU cores shared
        gpu_shared: 0,      // GPU not shared (privacy setting)
        memory_shared: 45,  // 45% of RAM shared
        storage_shared: 73, // 73% of storage shared
        earnings_rate: 1.85 // CAESAR tokens per hour from all sharing
      };
      
      hypermesh_allocations.set(allocation);
    },

    // Update Caesar economy based on real asset activity
    updateCaesarEconomy() {
      caesar_economy.update(economy => {
        // Calculate anti-speculation score based on utility vs speculation activity
        const utility_ratio = economy.utility_payments.length / 
                             (economy.utility_payments.length + 1); // +1 to avoid division by zero
        const anti_speculation_score = Math.floor(utility_ratio * 100);
        
        return {
          ...economy,
          anti_speculation_score
        };
      });
    },

    // Process real-world contract execution
    async executeContract(contractId, parameters) {
      try {
        loading_states.update(state => ({ ...state, processing_contract: true }));
        
        // TODO: Integrate with Catalog JuliaVM for secure contract execution
        console.log(`Executing contract ${contractId} with parameters:`, parameters);
        
        // Validate consensus proofs
        await this.validateConsensusProofs(contractId);
        
        // Execute via JuliaVM (placeholder)
        const result = await this.juliaVMExecution(contractId, parameters);
        
        return result;
        
      } catch (error) {
        console.error('Contract execution failed:', error);
        throw error;
      } finally {
        loading_states.update(state => ({ ...state, processing_contract: false }));
      }
    },

    // Validate all four consensus proofs (PoSpace, PoStake, PoWork, PoTime)
    async validateConsensusProofs(assetId) {
      try {
        loading_states.update(state => ({ ...state, validating_consensus: true }));
        
        // TODO: Integrate with HyperMesh consensus system
        const proofs = {
          pos_validated: true,  // Proof of Space validated
          post_validated: true, // Proof of Stake validated  
          powk_validated: true, // Proof of Work validated
          potm_validated: true, // Proof of Time validated
          consensus_score: 100
        };
        
        consensus_status.set(proofs);
        return proofs;
        
      } catch (error) {
        console.error('Consensus validation failed:', error);
        throw error;
      } finally {
        loading_states.update(state => ({ ...state, validating_consensus: false }));
      }
    },

    // JuliaVM execution placeholder
    async juliaVMExecution(contractId, parameters) {
      // TODO: Integrate with Catalog JuliaVM
      return {
        status: 'success',
        result: 'Contract executed successfully',
        gas_used: 0.05, // CAESAR gas cost
        execution_time: 125 // milliseconds
      };
    },

    // Update HyperMesh resource allocation
    async updateResourceAllocation(resourceType, percentage) {
      try {
        loading_states.update(state => ({ ...state, updating_allocation: true }));
        
        hypermesh_allocations.update(allocations => ({
          ...allocations,
          [`${resourceType}_shared`]: percentage
        }));
        
        // Recalculate earnings rate
        this.recalculateEarnings();
        
      } catch (error) {
        console.error('Failed to update resource allocation:', error);
        throw error;
      } finally {
        loading_states.update(state => ({ ...state, updating_allocation: false }));
      }
    },

    // Recalculate CAESAR earnings from shared resources
    recalculateEarnings() {
      hypermesh_allocations.update(allocations => {
        const cpuEarnings = (allocations.cpu_shared / 100) * 0.75;    // 0.75 CAESAR/hour per 100% CPU
        const memoryEarnings = (allocations.memory_shared / 100) * 0.25; // 0.25 CAESAR/hour per 100% RAM  
        const storageEarnings = (allocations.storage_shared / 100) * 0.85; // 0.85 CAESAR/hour per 100% storage
        
        const totalEarnings = cpuEarnings + memoryEarnings + storageEarnings;
        
        return {
          ...allocations,
          earnings_rate: parseFloat(totalEarnings.toFixed(2))
        };
      });
    },

    // Helper methods
    async getIdentity() {
      return new Promise((resolve) => {
        user_identity.subscribe(identity => {
          resolve(identity);
        })();
      });
    },

    async getProvider() {
      return new Promise((resolve) => {
        hypermesh_provider.subscribe(provider => {
          resolve(provider);
        })();
      });
    },

    // Initialize asset manager
    async initializeAssetManager() {
      if (typeof window.ethereum !== 'undefined') {
        const accounts = await window.ethereum.request({ method: 'eth_accounts' });
        if (accounts.length > 0) {
          await this.connectAssetManager();
        }
      }
    }
  };
}

export const assetStore = createAssetStore();

// Derived stores for computed values
export const total_asset_value = derived(
  asset_registry,
  $assets => {
    let total = 0;
    $assets.forEach(asset => {
      if (asset.currency === 'USD') {
        total += asset.value;
      } else if (asset.currency === 'CAESAR') {
        total += asset.value * 1.15; // Convert CAESAR to USD at current rate
      }
    });
    return total.toFixed(2);
  }
);

export const shared_resources_earnings = derived(
  hypermesh_allocations,
  $allocations => `${$allocations.earnings_rate} CAESAR/hour`
);

export const consensus_health = derived(
  consensus_status,
  $consensus => $consensus.consensus_score >= 75 ? 'healthy' : 'degraded'
);

export const real_economy_activity = derived(
  [active_contracts, service_subscriptions],
  ([$contracts, $services]) => $contracts.length + $services.length
);
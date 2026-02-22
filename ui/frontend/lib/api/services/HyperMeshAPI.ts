// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * HyperMesh API - Asset management, consensus validation, and Byzantine detection
 * 
 * Provides typed interface for HyperMesh service operations:
 * - Universal asset management (CPU, GPU, Memory, Storage)
 * - Four-proof consensus system (PoSp, PoSt, PoWk, PoTm)
 * - Byzantine fault detection and recovery
 * - Remote proxy/NAT addressing system
 */

import { web3ApiClient } from '../index';
import type { ServiceType } from '../Web3APIClient';

export type AssetType = 'cpu' | 'gpu' | 'memory' | 'storage' | 'network' | 'service' | 'container' | 'vm' | 'application' | 'compute';

export type PrivacyLevel = 'private' | 'private_network' | 'p2p' | 'public_network' | 'full_public' | 'federated' | 'public';

export type ProofType = 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';

export interface Asset {
  id: string;
  type: AssetType;
  name: string;
  description?: string;
  owner: string;
  status: 'available' | 'allocated' | 'busy' | 'maintenance' | 'offline' | 'active';
  privacyLevel: PrivacyLevel;
  location: {
    nodeId: string;
    address: string;
    region?: string;
  };
  specifications: Record<string, any>;
  metadata?: Record<string, any>;
  allocation: {
    totalCapacity: number;
    allocatedCapacity: number;
    availableCapacity: number;
    unit: string;
  };
  proxyAddress?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AssetAllocation {
  id: string;
  assetId: string;
  requesterId: string;
  amount: number;
  unit: string;
  duration: number;
  startTime: string;
  endTime: string;
  status: 'pending' | 'active' | 'completed' | 'cancelled' | 'failed';
  consensusProofs: ConsensusProof[];
  proxyAddress?: string;
}

export interface ConsensusProof {
  type: ProofType;
  data: any;
  validatedAt: string;
  validator: string;
  signature: string;
  valid: boolean;
}

export interface FourProofConsensus {
  blockId: string;
  assetId: string;
  proofs: ConsensusProof[];
  combinedProof: {
    hash: string;
    signature: string;
    validatedAt: string;
    consensusReached: boolean;
  };
  status: 'pending' | 'validated' | 'rejected' | 'failed';
  timestamp: string;
  validationTime: number; // ms
}

export interface ByzantineDetection {
  id: string;
  nodeId: string;
  detectedAt: string;
  behaviour: 'double_spending' | 'invalid_proof' | 'consensus_attack' | 'network_partition' | 'timing_attack';
  behaviorType: 'double_spending' | 'invalid_proof' | 'consensus_attack' | 'network_partition' | 'timing_attack';
  severity: 'low' | 'medium' | 'high' | 'critical';
  confidence: number; // 0-100
  evidence: {
    conflictingProofs?: ConsensusProof[];
    invalidOperations?: string[];
    networkAnomalies?: any[];
  };
  status: 'detected' | 'investigating' | 'confirmed' | 'resolved' | 'false_positive';
  action?: string;
  timestamp: string;
  mitigation?: {
    actions: string[];
    executedAt: string;
    successful: boolean;
  };
}

export interface RemoteProxy {
  id: string;
  assetId: string;
  address: string;
  type: 'memory' | 'storage' | 'compute' | 'network';
  targetAssetId: string;
  natMapping: {
    localAddress: string;
    remoteAddress: string;
    port?: number;
    protocol: 'tcp' | 'udp' | 'quic';
  };
  trust: {
    level: number; // 0-100
    validatedBy: string[];
    lastValidation: string;
  };
  performance: {
    latency: number;
    throughput: number;
    availability: number;
  };
  status: 'active' | 'inactive' | 'validating' | 'failed';
}

export interface NodeHealth {
  nodeId: string;
  status: 'healthy' | 'warning' | 'critical' | 'offline';
  overall: 'healthy' | 'warning' | 'critical' | 'offline';
  metrics: {
    cpuUsage: number;
    memoryUsage: number;
    diskUsage: number;
    networkLatency: number;
    uptime: number;
  };
  consensusMetrics: {
    proofsValidated: number;
    consensusParticipation: number;
    byzantineDetections: number;
  };
  lastHeartbeat: string;
}

export interface VMAsset extends Asset {
  type: 'vm' | 'application';
  vmConfig: {
    runtime: 'julia' | 'python' | 'node' | 'wasm' | 'docker';
    entrypoint: string;
    environment: Record<string, string>;
    dependencies: string[];
    resourceLimits: {
      maxCpu: number;
      maxMemory: string;
      maxStorage: string;
      maxExecutionTime: number;
    };
    securityPolicy: {
      allowNetworkAccess: boolean;
      allowFileSystem: boolean;
      allowedUrls?: string[];
      trustedDomains?: string[];
    };
  };
  catalogMetadata?: {
    catalogId: string;
    version: string;
    author: string;
    description: string;
    tags: string[];
    downloadCount: number;
    rating: number;
  };
}

export interface VMExecution {
  id: string;
  vmAssetId: string;
  allocationId: string;
  status: 'queued' | 'starting' | 'running' | 'completed' | 'failed' | 'cancelled';
  operation?: string;
  startTime?: string;
  request: {
    operation: string;
    parameters: any;
    timeout: number;
    requiresConsensus: boolean;
  };
  execution: {
    startTime?: string;
    endTime?: string;
    exitCode?: number;
    output?: string;
    error?: string;
    resourceUsage?: {
      cpuTime: number;
      memoryPeak: number;
      networkBytes: number;
      storageIO: number;
    };
  };
  result?: { output: string; exitCode: number; duration: number };
  consensusProofs?: ConsensusProof[];
  proxyAddress?: string;
}

export interface CatalogApplication {
  id: string;
  name: string;
  version: string;
  type: 'Application' | 'Library' | 'Runtime' | 'Service' | 'Data';
  adapter: 'Docker' | 'WASM' | 'Native' | 'Python' | 'Node.js' | 'Julia';
  status: 'Available' | 'Installed' | 'Installing' | 'Failed' | 'Updating';
  description: string;
  category?: string;
  requirements: {
    cpu?: number;
    memory?: number;
    storage?: number;
    network?: boolean;
  };
  dependencies: string[];
  author: string;
  downloads: number;
  downloadCount?: number;
  rating: number;
  size: string;
  lastUpdated: string;
  tags?: string[];
  performance?: { latency: number; throughput: number };
  // HyperMesh integration
  assetId?: string; // Links to HyperMesh asset when installed
  privacyLevel?: PrivacyLevel;
}

export class HyperMeshAPI {
  private readonly service: ServiceType = 'hypermesh';

  /**
   * Get all assets with optional filtering
   */
  async getAssets(filters?: {
    type?: AssetType;
    status?: string;
    privacyLevel?: PrivacyLevel;
    owner?: string;
  }): Promise<Asset[]> {
    const params = new URLSearchParams();
    if (filters) {
      Object.entries(filters).forEach(([key, value]) => {
        if (value) params.append(key, value);
      });
    }
    
    const endpoint = params.toString() ? `/api/v1/hypermesh/assets?${params}` : '/api/v1/hypermesh/assets';
    return web3ApiClient.request<Asset[]>(this.service, endpoint);
  }

  /**
   * Get specific asset by ID
   */
  async getAsset(assetId: string): Promise<Asset> {
    return web3ApiClient.request<Asset>(this.service, `/api/v1/hypermesh/assets/${assetId}`);
  }

  /**
   * Create new asset
   */
  async createAsset(assetData: Omit<Asset, 'id' | 'createdAt' | 'updatedAt'>): Promise<Asset> {
    return web3ApiClient.request<Asset>(this.service, '/api/v1/hypermesh/assets', {
      method: 'POST',
      body: assetData
    });
  }

  /**
   * Update asset
   */
  async updateAsset(assetId: string, updates: Partial<Asset>): Promise<Asset> {
    return web3ApiClient.request<Asset>(this.service, `/api/v1/hypermesh/assets/${assetId}`, {
      method: 'PUT',
      body: updates
    });
  }

  /**
   * Delete asset
   */
  async deleteAsset(assetId: string): Promise<void> {
    await web3ApiClient.request(this.service, `/api/v1/hypermesh/assets/${assetId}`, {
      method: 'DELETE'
    });
  }

  /**
   * Request asset allocation
   */
  async requestAllocation(request: {
    assetId: string;
    amount: number;
    duration: number;
    requirements?: Record<string, any>;
  }): Promise<AssetAllocation> {
    return web3ApiClient.request<AssetAllocation>(this.service, '/api/v1/hypermesh/allocations', {
      method: 'POST',
      body: request
    });
  }

  /**
   * Get asset allocations
   */
  async getAllocations(assetId?: string): Promise<AssetAllocation[]> {
    const endpoint = assetId ? `/api/v1/hypermesh/allocations?assetId=${assetId}` : '/api/v1/hypermesh/allocations';
    return web3ApiClient.request<AssetAllocation[]>(this.service, endpoint);
  }

  /**
   * Release allocation
   */
  async releaseAllocation(allocationId: string): Promise<void> {
    await web3ApiClient.request(this.service, `/api/v1/hypermesh/allocations/${allocationId}/release`, {
      method: 'POST'
    });
  }

  /**
   * Validate four-proof consensus
   */
  async validateConsensus(assetId: string, blockId: string): Promise<FourProofConsensus> {
    return web3ApiClient.request<FourProofConsensus>(this.service, `/api/v1/hypermesh/consensus/validate`, {
      method: 'POST',
      body: { assetId, blockId }
    });
  }

  /**
   * Get consensus history for asset
   */
  async getConsensusHistory(assetId: string, limit: number = 100): Promise<FourProofConsensus[]> {
    return web3ApiClient.request<FourProofConsensus[]>(this.service, 
      `/api/v1/hypermesh/consensus/history/${assetId}?limit=${limit}`);
  }

  /**
   * Submit proof for consensus
   */
  async submitProof(proof: {
    assetId: string;
    blockId: string;
    type: ProofType;
    data: any;
    signature: string;
  }): Promise<{ accepted: boolean; reason?: string }> {
    return web3ApiClient.request(this.service, '/api/v1/hypermesh/consensus/proof', {
      method: 'POST',
      body: proof
    });
  }

  /**
   * Get Byzantine detection results
   */
  async getByzantineDetections(nodeId?: string): Promise<ByzantineDetection[]> {
    const endpoint = nodeId ? `/api/v1/hypermesh/byzantine/detections?nodeId=${nodeId}` : '/api/v1/hypermesh/byzantine/detections';
    return web3ApiClient.request<ByzantineDetection[]>(this.service, endpoint);
  }

  /**
   * Report Byzantine behavior
   */
  async reportByzantineBehavior(report: {
    nodeId: string;
    behavior: ByzantineDetection['behaviour'];
    evidence: any;
    description: string;
  }): Promise<ByzantineDetection> {
    return web3ApiClient.request<ByzantineDetection>(this.service, '/api/v1/hypermesh/byzantine/report', {
      method: 'POST',
      body: report
    });
  }

  /**
   * Get remote proxies
   */
  async getRemoteProxies(assetId?: string): Promise<RemoteProxy[]> {
    const endpoint = assetId ? `/api/v1/hypermesh/proxy/list?assetId=${assetId}` : '/api/v1/hypermesh/proxy/list';
    return web3ApiClient.request<RemoteProxy[]>(this.service, endpoint);
  }

  /**
   * Create remote proxy for asset
   */
  async createRemoteProxy(config: {
    assetId: string;
    type: RemoteProxy['type'];
    remoteAddress: string;
    protocol: 'tcp' | 'udp' | 'quic';
    port?: number;
    virtualAddress?: string;
  }): Promise<RemoteProxy> {
    return web3ApiClient.request<RemoteProxy>(this.service, '/api/v1/hypermesh/proxy/create', {
      method: 'POST',
      body: config
    });
  }

  /**
   * Update remote proxy configuration
   */
  async updateRemoteProxy(proxyId: string, updates: Partial<RemoteProxy>): Promise<RemoteProxy> {
    return web3ApiClient.request<RemoteProxy>(this.service, `/api/v1/hypermesh/proxy/${proxyId}`, {
      method: 'PUT',
      body: updates
    });
  }

  /**
   * Validate proxy trust
   */
  async validateProxyTrust(proxyId: string): Promise<{
    trustLevel: number;
    validators: string[];
    validationResults: Array<{
      validator: string;
      result: boolean;
      reason?: string;
    }>;
  }> {
    return web3ApiClient.request(this.service, `/api/v1/hypermesh/proxy/${proxyId}/validate-trust`);
  }

  /**
   * Get node health status
   */
  async getNodeHealth(nodeId?: string): Promise<NodeHealth | NodeHealth[]> {
    const endpoint = nodeId ? `/api/v1/hypermesh/nodes/${nodeId}/health` : '/api/v1/hypermesh/nodes/health';
    return web3ApiClient.request(this.service, endpoint);
  }

  /**
   * Get network topology
   */
  async getNetworkTopology(): Promise<{
    nodes: Array<{
      id: string;
      address: string;
      status: string;
      connections: string[];
      region?: string;
    }>;
    connections: Array<{
      from: string;
      to: string;
      latency: number;
      bandwidth: number;
      status: string;
    }>;
    clusters: Array<{
      id: string;
      nodes: string[];
      region: string;
    }>;
  }> {
    return web3ApiClient.request(this.service, '/api/v1/hypermesh/network/topology');
  }

  /**
   * Execute remote operation through proxy
   */
  async executeRemoteOperation(operation: {
    proxyId: string;
    operation: string;
    parameters: any;
    timeout?: number;
    proxyAddress?: string;
  }): Promise<{
    success: boolean;
    result?: any;
    error?: string;
    executionTime: number;
  }> {
    return web3ApiClient.request(this.service, '/api/v1/hypermesh/proxy/execute', {
      method: 'POST',
      body: operation
    });
  }

  /**
   * Get HyperMesh system status
   */
  async getSystemStatus(): Promise<{
    status: 'healthy' | 'degraded' | 'critical';
    totalAssets: number;
    activeAllocations: number;
    consensusHealth: number;
    byzantineDetections: number;
    networkNodes: number;
    proxyConnections: number;
    lastConsensus: string;
    uptime: number;
  }> {
    return web3ApiClient.request(this.service, '/api/v1/hypermesh/system/status');
  }

  /**
   * Create VM asset from Catalog application
   */
  async createVMAsset(request: {
    catalogApp: CatalogApplication;
    config: {
      privacyLevel: PrivacyLevel;
      resourceLimits?: Partial<VMAsset['vmConfig']['resourceLimits']>;
      securityPolicy?: Partial<VMAsset['vmConfig']['securityPolicy']>;
    };
    name?: string;
    type?: AssetType;
    privacyLevel?: PrivacyLevel;
  }): Promise<VMAsset> {
    const catalogApp = request.catalogApp;
    const config = request.config;
    const vmAssetData: Omit<VMAsset, 'id' | 'createdAt' | 'updatedAt'> = {
      type: 'vm',
      name: `VM: ${catalogApp.name}`,
      description: catalogApp.description,
      owner: '', // Will be set by backend from auth context
      status: 'available',
      privacyLevel: config.privacyLevel,
      location: {
        nodeId: '',
        address: '',
        region: 'local'
      },
      specifications: {
        runtime: catalogApp.adapter.toLowerCase(),
        catalogVersion: catalogApp.version,
        catalogId: catalogApp.id
      },
      allocation: {
        totalCapacity: 1,
        allocatedCapacity: 0,
        availableCapacity: 1,
        unit: 'instances'
      },
      vmConfig: {
        runtime: this.mapAdapterToRuntime(catalogApp.adapter),
        entrypoint: 'main', // Default, can be overridden
        environment: {},
        dependencies: catalogApp.dependencies,
        resourceLimits: {
          maxCpu: catalogApp.requirements.cpu || 1,
          maxMemory: `${catalogApp.requirements.memory || 1}GB`,
          maxStorage: `${catalogApp.requirements.storage || 1}GB`,
          maxExecutionTime: 300, // 5 minutes default
          ...config.resourceLimits
        },
        securityPolicy: {
          allowNetworkAccess: catalogApp.requirements.network || false,
          allowFileSystem: true,
          allowedUrls: [],
          trustedDomains: [],
          ...config.securityPolicy
        }
      },
      catalogMetadata: {
        catalogId: catalogApp.id,
        version: catalogApp.version,
        author: catalogApp.author,
        description: catalogApp.description,
        tags: [catalogApp.type.toLowerCase()],
        downloadCount: catalogApp.downloads,
        rating: catalogApp.rating
      }
    };

    return web3ApiClient.request<VMAsset>(this.service, '/api/v1/hypermesh/assets/vm', {
      method: 'POST',
      body: vmAssetData
    });
  }

  /**
   * Execute VM asset through HyperMesh allocation system
   */
  async executeVMAsset(request: {
    vmAssetId: string;
    operation: string;
    parameters: any;
    timeout?: number;
    requiresConsensus?: boolean;
    allocationDuration?: number;
  }): Promise<VMExecution> {
    return web3ApiClient.request<VMExecution>(this.service, '/api/v1/hypermesh/vm/execute', {
      method: 'POST',
      body: {
        vmAssetId: request.vmAssetId,
        operation: request.operation,
        parameters: request.parameters,
        timeout: request.timeout || 300,
        requiresConsensus: request.requiresConsensus || true,
        allocationDuration: request.allocationDuration || 3600 // 1 hour default
      }
    });
  }

  /**
   * Get VM execution status and results
   */
  async getVMExecution(executionId: string): Promise<VMExecution> {
    return web3ApiClient.request<VMExecution>(this.service, `/api/v1/hypermesh/vm/executions/${executionId}`);
  }

  /**
   * List VM executions for an asset or all executions
   */
  async getVMExecutions(vmAssetId?: string): Promise<VMExecution[]> {
    const endpoint = vmAssetId ? `/api/v1/hypermesh/vm/executions?vmAssetId=${vmAssetId}` : '/api/v1/hypermesh/vm/executions';
    return web3ApiClient.request<VMExecution[]>(this.service, endpoint);
  }

  /**
   * Cancel VM execution
   */
  async cancelVMExecution(executionId: string): Promise<{ cancelled: boolean; reason?: string }> {
    return web3ApiClient.request(this.service, `/api/v1/hypermesh/vm/executions/${executionId}/cancel`, {
      method: 'POST'
    });
  }

  /**
   * Get VM asset details (typed version of getAsset for VM assets)
   */
  async getVMAsset(assetId: string): Promise<VMAsset> {
    return web3ApiClient.request<VMAsset>(this.service, `/api/v1/hypermesh/assets/${assetId}`);
  }

  /**
   * Update VM asset configuration
   */
  async updateVMAsset(assetId: string, updates: Partial<VMAsset>): Promise<VMAsset> {
    return web3ApiClient.request<VMAsset>(this.service, `/api/v1/hypermesh/assets/${assetId}`, {
      method: 'PUT',
      body: updates
    });
  }

  /**
   * Get Catalog applications (bridge to Catalog service)
   */
  async getCatalogApplications(filters?: {
    type?: string;
    adapter?: string;
    status?: string;
  }): Promise<CatalogApplication[]> {
    const params = new URLSearchParams();
    if (filters) {
      Object.entries(filters).forEach(([key, value]) => {
        if (value) params.append(key, value);
      });
    }
    
    const endpoint = params.toString() ? `/api/v1/hypermesh/catalog/applications?${params}` : '/api/v1/hypermesh/catalog/applications';
    return web3ApiClient.request<CatalogApplication[]>(this.service, endpoint);
  }

  /**
   * Install Catalog application as HyperMesh VM asset
   */
  async installCatalogApplication(catalogId: string, config: {
    privacyLevel: PrivacyLevel;
    autoStart?: boolean;
    resourceLimits?: Partial<VMAsset['vmConfig']['resourceLimits']>;
  }): Promise<{
    vmAsset: VMAsset;
    installation: {
      status: 'installing' | 'completed' | 'failed';
      progress: number;
      logs: string[];
    };
  }> {
    return web3ApiClient.request(this.service, '/api/v1/hypermesh/catalog/install', {
      method: 'POST',
      body: {
        catalogId,
        privacyLevel: config.privacyLevel,
        autoStart: config.autoStart || false,
        resourceLimits: config.resourceLimits
      }
    });
  }

  /**
   * Map Catalog adapter to VM runtime
   */
  private mapAdapterToRuntime(adapter: string): VMAsset['vmConfig']['runtime'] {
    switch (adapter.toLowerCase()) {
      case 'julia': return 'julia';
      case 'python': return 'python';
      case 'node.js': return 'node';
      case 'wasm': return 'wasm';
      case 'docker': return 'docker';
      default: return 'docker'; // fallback
    }
  }
}

// Singleton instance
export const hyperMeshAPI = new HyperMeshAPI();
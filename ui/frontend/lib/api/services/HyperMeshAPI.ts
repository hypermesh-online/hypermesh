// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * HyperMesh API - Asset management, state proof verification, and Byzantine detection
 *
 * Provides typed interface for HyperMesh service operations:
 * - Universal asset management (CPU, GPU, Memory, Storage)
 * - Four-proof state verification system (PoSp, PoSt, PoWk, PoTm)
 * - Byzantine fault detection and recovery
 * - Remote proxy/NAT addressing system
 */

import { get, post, put, del } from '../../api';

// Re-export all types from the types file for backward compatibility
export type {
  AssetType,
  PrivacyLevel,
  ProofType,
  Asset,
  AssetAllocation,
  StateProof,
  FourProofStateVerification,
  ByzantineDetection,
  RemoteProxy,
  NodeHealth,
  VMAsset,
  VMExecution,
  CatalogApplication
} from './HyperMeshTypes';

import type {
  Asset,
  AssetType,
  PrivacyLevel,
  ProofType,
  FourProofStateVerification,
  AssetAllocation,
  ByzantineDetection,
  RemoteProxy,
  NodeHealth,
  VMAsset,
  VMExecution,
  CatalogApplication
} from './HyperMeshTypes';

export class HyperMeshAPI {
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
    return get<Asset[]>(endpoint);
  }

  async getAsset(assetId: string): Promise<Asset> {
    return get<Asset>(`/api/v1/hypermesh/assets/${assetId}`);
  }

  async createAsset(assetData: Omit<Asset, 'id' | 'createdAt' | 'updatedAt'>): Promise<Asset> {
    return post<Asset>('/api/v1/hypermesh/assets', assetData);
  }

  async updateAsset(assetId: string, updates: Partial<Asset>): Promise<Asset> {
    return put<Asset>(`/api/v1/hypermesh/assets/${assetId}`, updates);
  }

  async deleteAsset(assetId: string): Promise<void> {
    await del(`/api/v1/hypermesh/assets/${assetId}`);
  }

  async requestAllocation(request: {
    assetId: string;
    amount: number;
    duration: number;
    requirements?: Record<string, any>;
  }): Promise<AssetAllocation> {
    return post<AssetAllocation>('/api/v1/hypermesh/allocations', request);
  }

  async getAllocations(assetId?: string): Promise<AssetAllocation[]> {
    const endpoint = assetId ? `/api/v1/hypermesh/allocations?assetId=${assetId}` : '/api/v1/hypermesh/allocations';
    return get<AssetAllocation[]>(endpoint);
  }

  async releaseAllocation(allocationId: string): Promise<void> {
    await post(`/api/v1/hypermesh/allocations/${allocationId}/release`);
  }

  async validateStateProof(assetId: string, blockId: string): Promise<FourProofStateVerification> {
    return post<FourProofStateVerification>('/api/v1/hypermesh/state-proof/validate', { assetId, blockId });
  }

  async getStateProofHistory(assetId: string, limit: number = 100): Promise<FourProofStateVerification[]> {
    return get<FourProofStateVerification[]>(`/api/v1/hypermesh/state-proof/history/${assetId}?limit=${limit}`);
  }

  async submitProof(proof: {
    assetId: string;
    blockId: string;
    type: ProofType;
    data: any;
    signature: string;
  }): Promise<{ accepted: boolean; reason?: string }> {
    return post('/api/v1/hypermesh/state-proof/submit', proof);
  }

  async getByzantineDetections(nodeId?: string): Promise<ByzantineDetection[]> {
    const endpoint = nodeId ? `/api/v1/hypermesh/byzantine/detections?nodeId=${nodeId}` : '/api/v1/hypermesh/byzantine/detections';
    return get<ByzantineDetection[]>(endpoint);
  }

  async reportByzantineBehavior(report: {
    nodeId: string;
    behavior: ByzantineDetection['behaviour'];
    evidence: any;
    description: string;
  }): Promise<ByzantineDetection> {
    return post<ByzantineDetection>('/api/v1/hypermesh/byzantine/report', report);
  }

  async getRemoteProxies(assetId?: string): Promise<RemoteProxy[]> {
    const endpoint = assetId ? `/api/v1/hypermesh/proxy/list?assetId=${assetId}` : '/api/v1/hypermesh/proxy/list';
    return get<RemoteProxy[]>(endpoint);
  }

  async createRemoteProxy(config: {
    assetId: string;
    type: RemoteProxy['type'];
    remoteAddress: string;
    protocol: 'tcp' | 'udp' | 'quic';
    port?: number;
    virtualAddress?: string;
  }): Promise<RemoteProxy> {
    return post<RemoteProxy>('/api/v1/hypermesh/proxy/create', config);
  }

  async updateRemoteProxy(proxyId: string, updates: Partial<RemoteProxy>): Promise<RemoteProxy> {
    return put<RemoteProxy>(`/api/v1/hypermesh/proxy/${proxyId}`, updates);
  }

  async validateProxyTrust(proxyId: string): Promise<{
    trustLevel: number;
    validators: string[];
    validationResults: Array<{
      validator: string;
      result: boolean;
      reason?: string;
    }>;
  }> {
    return get(`/api/v1/hypermesh/proxy/${proxyId}/validate-trust`);
  }

  async getNodeHealth(nodeId?: string): Promise<NodeHealth | NodeHealth[]> {
    const endpoint = nodeId ? `/api/v1/hypermesh/nodes/${nodeId}/health` : '/api/v1/hypermesh/nodes/health';
    return get(endpoint);
  }

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
    return get('/api/v1/hypermesh/network/topology');
  }

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
    return post('/api/v1/hypermesh/proxy/execute', operation);
  }

  async getSystemStatus(): Promise<{
    status: 'healthy' | 'degraded' | 'critical';
    totalAssets: number;
    activeAllocations: number;
    stateProofHealth: number;
    byzantineDetections: number;
    networkNodes: number;
    proxyConnections: number;
    lastStateProof: string;
    uptime: number;
  }> {
    return get('/api/v1/hypermesh/system/status');
  }

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
      owner: '',
      status: 'available',
      privacyLevel: config.privacyLevel,
      location: { nodeId: '', address: '', region: 'local' },
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
        entrypoint: 'main',
        environment: {},
        dependencies: catalogApp.dependencies,
        resourceLimits: {
          maxCpu: catalogApp.requirements.cpu || 1,
          maxMemory: `${catalogApp.requirements.memory || 1}GB`,
          maxStorage: `${catalogApp.requirements.storage || 1}GB`,
          maxExecutionTime: 300,
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

    return post<VMAsset>('/api/v1/hypermesh/assets/vm', vmAssetData);
  }

  async executeVMAsset(request: {
    vmAssetId: string;
    operation: string;
    parameters: any;
    timeout?: number;
    requiresStateProof?: boolean;
    allocationDuration?: number;
  }): Promise<VMExecution> {
    return post<VMExecution>('/api/v1/hypermesh/vm/execute', {
      vmAssetId: request.vmAssetId,
      operation: request.operation,
      parameters: request.parameters,
      timeout: request.timeout || 300,
      requiresStateProof: request.requiresStateProof || true,
      allocationDuration: request.allocationDuration || 3600
    });
  }

  async getVMExecution(executionId: string): Promise<VMExecution> {
    return get<VMExecution>(`/api/v1/hypermesh/vm/executions/${executionId}`);
  }

  async getVMExecutions(vmAssetId?: string): Promise<VMExecution[]> {
    const endpoint = vmAssetId ? `/api/v1/hypermesh/vm/executions?vmAssetId=${vmAssetId}` : '/api/v1/hypermesh/vm/executions';
    return get<VMExecution[]>(endpoint);
  }

  async cancelVMExecution(executionId: string): Promise<{ cancelled: boolean; reason?: string }> {
    return post(`/api/v1/hypermesh/vm/executions/${executionId}/cancel`);
  }

  async getVMAsset(assetId: string): Promise<VMAsset> {
    return get<VMAsset>(`/api/v1/hypermesh/assets/${assetId}`);
  }

  async updateVMAsset(assetId: string, updates: Partial<VMAsset>): Promise<VMAsset> {
    return put<VMAsset>(`/api/v1/hypermesh/assets/${assetId}`, updates);
  }

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
    return get<CatalogApplication[]>(endpoint);
  }

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
    return post('/api/v1/hypermesh/catalog/install', {
      catalogId,
      privacyLevel: config.privacyLevel,
      autoStart: config.autoStart || false,
      resourceLimits: config.resourceLimits
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

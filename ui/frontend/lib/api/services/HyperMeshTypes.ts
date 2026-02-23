// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * HyperMesh API Types - Type definitions for asset management, consensus, and proxy systems
 */

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

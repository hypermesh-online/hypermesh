// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Web3 Mock Data - Development mode mock responses
 *
 * Provides mock API responses when the backend is unavailable.
 * Used by Web3APIClient in development/fallback mode.
 */

import type { ServiceType } from './Web3APIClient';
import type { APIResponse } from './Web3APIClient';

/**
 * Create mock response for development mode
 */
export function createMockResponse(service: ServiceType, endpoint: string, method: string): APIResponse<any> {
  console.log(`[MOCK] ${service} ${method} ${endpoint}`);

  let mockData: any;

  if (service === 'trustchain') {
    mockData = getTrustChainMockData(endpoint, method);
  } else if (service === 'hypermesh') {
    mockData = getHyperMeshMockData(endpoint, method);
  } else if (service === 'stoq') {
    mockData = getStoqMockData(endpoint, method);
  } else {
    mockData = { message: 'Mock response', service, endpoint, method };
  }

  return {
    data: mockData,
    status: 200,
    headers: {
      'content-type': 'application/json',
      'x-mock-response': 'true'
    },
    timestamp: new Date()
  };
}

function getTrustChainMockData(endpoint: string, method: string): any {
  if (endpoint === '/api/v1/trustchain/health') {
    return {
      status: 'healthy',
      timestamp: new Date().toISOString(),
      version: '1.0.0',
      services: { ca: true, ct: true, dns: true, stateProof: true }
    };
  }

  if (endpoint === '/api/v1/trustchain/stats') {
    return {
      requests_total: 1234,
      requests_successful: 1200,
      requests_failed: 34,
      ca_requests: 456,
      ct_requests: 234,
      dns_requests: 123,
      average_response_time_ms: 45.2,
      active_connections: 12,
      rate_limited_requests: 2,
      last_update: new Date().toISOString()
    };
  }

  if (endpoint === '/api/v1/trustchain/status') {
    return {
      server_id: 'trustchain-dev',
      uptime_seconds: 86400,
      stats: {
        requests_total: 1234,
        requests_successful: 1200,
        requests_failed: 34,
        ca_requests: 456,
        ct_requests: 234,
        dns_requests: 123,
        average_response_time_ms: 45.2,
        active_connections: 12,
        rate_limited_requests: 2,
        last_update: new Date().toISOString()
      },
      configuration: {
        bind_address: '::1',
        port: 8443,
        tls_enabled: true,
        rate_limit_per_minute: 60
      }
    };
  }

  if (endpoint === '/api/v1/trustchain/certificates') {
    return [
      {
        id: 'cert-001',
        subject: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
        issuer: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
        serialNumber: '1234567890ABCDEF',
        validFrom: new Date(Date.now() - 86400000 * 365).toISOString(),
        validTo: new Date(Date.now() + 86400000 * 365 * 2).toISOString(),
        fingerprint: 'SHA256:ABCD1234...',
        publicKey: '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhki...',
        status: 'active',
        trustLevel: 'root'
      },
      {
        id: 'cert-002',
        subject: 'CN=node-001.hypermesh.network, O=HyperMesh, C=US',
        issuer: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
        serialNumber: '2345678901BCDEF0',
        validFrom: new Date(Date.now() - 86400000 * 30).toISOString(),
        validTo: new Date(Date.now() + 86400000 * 90).toISOString(),
        fingerprint: 'SHA256:BCDE2345...',
        publicKey: '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhki...',
        status: 'active',
        trustLevel: 'leaf'
      }
    ];
  }

  if (endpoint === '/api/v1/trustchain/trust/hierarchy') {
    return {
      rootCA: {
        id: 'cert-001',
        subject: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
        issuer: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
        serialNumber: '1234567890ABCDEF',
        validFrom: new Date(Date.now() - 86400000 * 365).toISOString(),
        validTo: new Date(Date.now() + 86400000 * 365 * 2).toISOString(),
        fingerprint: 'SHA256:ABCD1234...',
        publicKey: '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhki...',
        status: 'active',
        trustLevel: 'root'
      },
      intermediates: [],
      leaves: [
        {
          id: 'cert-002',
          subject: 'CN=node-001.hypermesh.network, O=HyperMesh, C=US',
          issuer: 'CN=TrustChain Root CA, O=HyperMesh, C=US',
          serialNumber: '2345678901BCDEF0',
          validFrom: new Date(Date.now() - 86400000 * 30).toISOString(),
          validTo: new Date(Date.now() + 86400000 * 90).toISOString(),
          fingerprint: 'SHA256:BCDE2345...',
          publicKey: '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhki...',
          status: 'active',
          trustLevel: 'leaf'
        }
      ],
      validationChain: ['cert-001', 'cert-002'],
      lastValidated: new Date().toISOString()
    };
  }

  return { message: 'Mock response for TrustChain', endpoint, method };
}

function getHyperMeshMockData(endpoint: string, method: string): any {
  if (endpoint === '/api/v1/hypermesh/system/status') {
    return {
      status: 'healthy',
      totalAssets: 42,
      activeAllocations: 12,
      stateProofHealth: 98.5,
      byzantineDetections: 0,
      networkNodes: 8,
      proxyConnections: 24,
      lastStateProof: new Date(Date.now() - 5000).toISOString(),
      uptime: 99.8
    };
  }

  if (endpoint === '/api/v1/hypermesh/assets') {
    return [
      {
        id: 'asset-cpu-001',
        type: 'cpu',
        name: 'High-Performance CPU Pool',
        description: 'Intel Xeon E5-2699 v4 cluster',
        owner: 'system',
        status: 'available',
        privacyLevel: 'public_network',
        location: { nodeId: 'node-001', address: '2001:db8::1', region: 'us-west-1' },
        specifications: { cores: 44, threads: 88, frequency: '2.2GHz', architecture: 'x86_64' },
        allocation: { totalCapacity: 100, allocatedCapacity: 25, availableCapacity: 75, unit: 'percentage' },
        proxyAddress: '2001:db8:proxy::cpu:001',
        createdAt: new Date(Date.now() - 86400000).toISOString(),
        updatedAt: new Date().toISOString()
      },
      {
        id: 'asset-gpu-001',
        type: 'gpu',
        name: 'NVIDIA H100 GPU Farm',
        description: 'High-throughput GPU compute cluster',
        owner: 'system',
        status: 'allocated',
        privacyLevel: 'private_network',
        location: { nodeId: 'node-002', address: '2001:db8::2', region: 'us-west-1' },
        specifications: { model: 'H100', memory: '80GB HBM3', cores: 16896, frequency: '1980MHz' },
        allocation: { totalCapacity: 8, allocatedCapacity: 6, availableCapacity: 2, unit: 'units' },
        proxyAddress: '2001:db8:proxy::gpu:001',
        createdAt: new Date(Date.now() - 172800000).toISOString(),
        updatedAt: new Date().toISOString()
      }
    ];
  }

  if (endpoint === '/api/v1/hypermesh/allocations') {
    return [
      {
        id: 'alloc-001',
        assetId: 'asset-cpu-001',
        requesterId: 'user-123',
        amount: 25,
        unit: 'percentage',
        duration: 3600,
        startTime: new Date(Date.now() - 1800000).toISOString(),
        endTime: new Date(Date.now() + 1800000).toISOString(),
        status: 'active',
        stateProofs: [],
        proxyAddress: '2001:db8:proxy::cpu:001/user-123'
      }
    ];
  }

  if (endpoint === '/api/v1/hypermesh/byzantine/detections') {
    return [];
  }

  if (endpoint === '/api/v1/hypermesh/node/health') {
    return {
      nodeId: 'node-001',
      status: 'healthy',
      uptime: 99.8,
      resources: {
        cpu: { usage: 25, temperature: 65, status: 'normal' },
        memory: { usage: 60, available: '128GB', status: 'normal' },
        storage: { usage: 45, available: '2TB', status: 'normal' },
        network: { bandwidth: '10Gbps', latency: '2ms', status: 'optimal' }
      },
      stateProof: { participation: 100, validations: 1234, errors: 0 },
      lastUpdate: new Date().toISOString()
    };
  }

  if (endpoint === '/api/v1/hypermesh/remote-proxies') {
    return [
      {
        id: 'proxy-001',
        assetId: 'asset-cpu-001',
        virtualAddress: '2001:db8:proxy::cpu:001',
        physicalAddress: '2001:db8::1:8080',
        accessLevel: 'federated',
        bandwidth: 1000,
        latency: 5.2,
        validationStatus: 'verified' as const
      }
    ];
  }

  return {
    status: 'healthy',
    assets: [],
    stateProof: { validations: 0 },
    uptime: 99.5
  };
}

function getStoqMockData(endpoint: string, method: string): any {
  if (endpoint === '/api/v1/stoq/system/health') {
    return {
      status: 'degraded',
      version: '1.0.0',
      uptime: 99.8,
      performance: {
        globalThroughput: 2950,
        targetThroughput: 40000,
        achievementPercentage: 7.375,
        bottlenecks: [
          'QUIC implementation optimization needed',
          'Hardware acceleration underutilized',
          'Stream multiplexing inefficiencies',
          'Connection pooling suboptimal'
        ]
      },
      connections: {
        total: 156,
        active: 42,
        failed: 8,
        averagePerformance: 2.95
      },
      certificates: {
        total: 12,
        valid: 12,
        expiring: 0,
        errors: 0
      },
      nodes: {
        connected: 8,
        synchronized: 8,
        stateProof: 'healthy'
      }
    };
  }

  return {
    status: 'optimal',
    connections: 42,
    throughput: 2950,
    latency: 35.2,
    uptime: 99.8,
    version: '1.0.0'
  };
}

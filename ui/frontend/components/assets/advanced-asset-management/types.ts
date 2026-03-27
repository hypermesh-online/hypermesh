// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface AssetMetrics {
  totalAssets: number;
  activeAssets: number;
  allocatedResources: number;
  utilizationRate: number;
  performanceScore: number;
  proxyConnections: number;
}

export interface ProxyAddress {
  id: string;
  assetId: string;
  virtualAddress: string;
  physicalAddress: string;
  accessLevel: 'private' | 'federated' | 'public';
  bandwidth: number;
  latency: number;
  validationStatus: 'verified' | 'rejected';
}

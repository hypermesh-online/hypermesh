// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface ProxyMapping {
  id: string;
  assetId: string;
  localAddress: string;
  proxyAddress: string;
  virtualAddress: string;
  trustLevel: 'low' | 'medium' | 'high';
  accessLevel: string;
  bandwidth: number;
  latency: number;
  status: 'active' | 'inactive' | 'connecting' | 'error';
  connectionCount: number;
  lastSeen: string;
}

export interface ProxyNode {
  id: string;
  address: string;
  validationStatus: 'verified' | 'rejected';
  location: string;
  bandwidth: number;
  latency: number;
  uptime: number;
  capabilities: string[];
  securityLevel: 'basic' | 'standard' | 'enhanced' | 'quantum';
}

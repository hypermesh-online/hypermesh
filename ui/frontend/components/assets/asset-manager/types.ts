// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { LucideIcon } from 'lucide-react';

export interface AssetCreationStep {
  id: string;
  title: string;
  description: string;
  completed: boolean;
  current: boolean;
}

export interface AssetControlMetrics {
  cpuUsage: number;
  memoryUsage: number;
  storageUsage: number;
  networkUsage: number;
  performanceScore: number;
  efficiency: number;
}

export interface NewAssetConfig {
  name: string;
  type: 'compute' | 'storage' | 'network' | 'vm';
  privacyLevel: 'private' | 'federated' | 'public';
  resourceLimits: {
    cpu: number;
    memory: string;
    storage: string;
    network: string;
  };
}

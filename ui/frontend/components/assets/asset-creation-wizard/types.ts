// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type React from 'react';

export interface WizardStep {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<any>;
}

export interface AssetConfiguration {
  name: string;
  type: 'cpu' | 'storage' | 'network' | 'vm';
  privacyLevel: 'private' | 'private_network' | 'p2p' | 'public_network' | 'full_public';
  resourceLimits: {
    cpu: number;
    memory: string;
    storage: string;
    network: string;
  };
  vmConfig?: {
    runtime: 'julia' | 'python' | 'javascript' | 'rust';
    environmentVariables: Record<string, string>;
  };
}

export interface AssetCreationWizardProps {
  onComplete?: (assetId: string) => void;
  onCancel?: () => void;
  initialConfig?: Partial<AssetConfiguration>;
}

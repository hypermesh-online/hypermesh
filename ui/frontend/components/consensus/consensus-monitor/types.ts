// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface ProofValidation {
  id: string;
  type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
  assetId: string;
  blockId: string;
  status: 'validating' | 'validated' | 'failed' | 'pending';
  timestamp: string;
  validationTime: number;
  confidence: number;
  validatorNode: string;
}

export interface ConsensusOperation {
  id: string;
  type: 'asset_creation' | 'asset_allocation' | 'consensus_validation' | 'proxy_setup';
  assetId: string;
  requiredProofs: ('PoSp' | 'PoSt' | 'PoWk' | 'PoTm')[];
  completedProofs: ('PoSp' | 'PoSt' | 'PoWk' | 'PoTm')[];
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  startTime: string;
  estimatedCompletion?: string;
  priority: 'low' | 'medium' | 'high' | 'critical';
}

export interface ConsensusHealth {
  overallHealth: number;
  validationRate: number;
  networkParticipation: number;
  byzantineDetections: number;
  averageValidationTime: number;
  lastBlockTime: number;
  consensusStrength: number;
}

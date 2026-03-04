// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface ProofData {
  id: string;
  type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
  assetId: string;
  blockId: string;
  status: 'validated' | 'failed' | 'pending' | 'validating';
  timestamp: string;
  validationTime: number;
  confidence: number;
  validatorNode: string;
  proofStructure: {
    challenge: string;
    response: string;
    signature: string;
    merkleRoot?: string;
    difficulty?: number;
    nonce?: string;
    timestamp?: number;
  };
  validationSteps: {
    step: string;
    status: 'completed' | 'failed' | 'pending';
    duration: number;
    details: string;
  }[];
}

// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Target, Shield } from 'lucide-react';
import { getProofIcon } from './utils';
import type { StateProofHealth } from './types';

interface ProofsTabProps {
  stateProofHealth: StateProofHealth;
  firstAssetId: string | undefined;
  onSubmitProof: (type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm') => void;
  onValidateStateProof: () => void;
  isSubmitting: boolean;
  isValidating: boolean;
}

export function ProofsTab({
  stateProofHealth,
  firstAssetId,
  onSubmitProof,
  onValidateStateProof,
  isSubmitting,
  isValidating
}: ProofsTabProps) {
  return (
    <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Target className="h-5 w-5 text-yellow-400" />
          Interactive Proof Management
        </CardTitle>
        <CardDescription className="text-gray-400">
          Submit and validate proofs for the four-proof state verification system
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-6 md:grid-cols-2">
          <ProofSubmissionPanel
            onSubmitProof={onSubmitProof}
            isSubmitting={isSubmitting}
          />
          <StateProofControlPanel
            stateProofHealth={stateProofHealth}
            firstAssetId={firstAssetId}
            onValidateStateProof={onValidateStateProof}
            isValidating={isValidating}
          />
        </div>
      </CardContent>
    </Card>
  );
}

function ProofSubmissionPanel({
  onSubmitProof,
  isSubmitting
}: {
  onSubmitProof: (type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm') => void;
  isSubmitting: boolean;
}) {
  const proofTypes = [
    { type: 'PoSp' as const, name: 'Proof of Space', desc: 'WHERE - Storage location validation', color: 'blue' },
    { type: 'PoSt' as const, name: 'Proof of Stake', desc: 'WHO - Ownership validation', color: 'green' },
    { type: 'PoWk' as const, name: 'Proof of Work', desc: 'WHAT/HOW - Computation validation', color: 'yellow' },
    { type: 'PoTm' as const, name: 'Proof of Time', desc: 'WHEN - Temporal validation', color: 'purple' }
  ];

  return (
    <div className="space-y-4">
      <h4 className="text-white font-medium">Submit Proofs</h4>
      <div className="grid gap-3">
        {proofTypes.map((proof) => {
          const ProofIcon = getProofIcon(proof.type);

          return (
            <div key={proof.type} className="p-3 bg-gray-800/50 rounded-lg border border-gray-600/30">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <ProofIcon className={cn(
                    'h-4 w-4',
                    proof.color === 'blue' ? 'text-blue-400' :
                    proof.color === 'green' ? 'text-green-400' :
                    proof.color === 'yellow' ? 'text-yellow-400' :
                    'text-purple-400'
                  )} />
                  <span className="text-white font-medium text-sm">{proof.name}</span>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onSubmitProof(proof.type)}
                  disabled={isSubmitting}
                  className={cn(
                    'text-xs',
                    proof.color === 'blue' ? 'border-blue-500/30 text-blue-400' :
                    proof.color === 'green' ? 'border-green-500/30 text-green-400' :
                    proof.color === 'yellow' ? 'border-yellow-500/30 text-yellow-400' :
                    'border-purple-500/30 text-purple-400'
                  )}
                >
                  {isSubmitting ? 'Submitting...' : 'Submit'}
                </Button>
              </div>
              <p className="text-xs text-gray-400">{proof.desc}</p>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function StateProofControlPanel({
  stateProofHealth,
  firstAssetId,
  onValidateStateProof,
  isValidating
}: {
  stateProofHealth: StateProofHealth;
  firstAssetId: string | undefined;
  onValidateStateProof: () => void;
  isValidating: boolean;
}) {
  return (
    <div className="space-y-4">
      <h4 className="text-white font-medium">State Proof Verification</h4>
      <div className="space-y-3">
        <Card className="bg-gray-800/50 border-gray-600/30">
          <CardContent className="p-4">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Shield className="h-4 w-4 text-green-400" />
                <span className="text-white font-medium text-sm">State Proof Validation</span>
              </div>
              <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                Ready
              </Badge>
            </div>
            <p className="text-xs text-gray-400 mb-3">
              Validate state proofs across all four proof types for the selected asset
            </p>
            <Button
              onClick={onValidateStateProof}
              disabled={isValidating || !firstAssetId}
              className="w-full bg-gradient-to-r from-green-500 to-blue-600 hover:from-green-400 hover:to-blue-500 text-black text-sm"
            >
              {isValidating ? 'Validating...' : 'Initiate Validation'}
            </Button>
          </CardContent>
        </Card>

        <Card className="bg-gray-800/50 border-gray-600/30">
          <CardContent className="p-4">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-gray-400 text-sm">System Health:</span>
                <span className={cn(
                  'font-medium text-sm',
                  stateProofHealth.overallHealth >= 90 ? 'text-green-400' :
                  stateProofHealth.overallHealth >= 70 ? 'text-yellow-400' :
                  'text-red-400'
                )}>
                  {stateProofHealth.overallHealth.toFixed(1)}%
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400 text-sm">Avg Validation Time:</span>
                <span className="text-white font-mono text-sm">
                  {stateProofHealth.averageValidationTime.toFixed(1)}ms
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400 text-sm">Last Block:</span>
                <span className="text-white font-mono text-sm">
                  {Math.floor(stateProofHealth.lastBlockTime / 1000)}s ago
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

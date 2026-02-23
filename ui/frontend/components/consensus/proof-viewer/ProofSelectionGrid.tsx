// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Eye } from 'lucide-react';
import { getProofIcon, getProofColor } from './utils';
import type { ProofData } from './types';

interface ProofSelectionGridProps {
  proofData: ProofData[];
  selectedProof: string | null;
  setSelectedProof: (id: string) => void;
}

export function ProofSelectionGrid({
  proofData,
  selectedProof,
  setSelectedProof
}: ProofSelectionGridProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Eye className="h-5 w-5 text-purple-400" />
          Four-Proof Detailed Analysis
        </CardTitle>
        <CardDescription className="text-gray-400">
          Examine individual consensus proofs and their validation processes
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-4 max-h-64 overflow-y-auto">
          {proofData.map((proof) => {
            const ProofIcon = getProofIcon(proof.type);
            const color = getProofColor(proof.type);
            const isSelected = selectedProof === proof.id;

            return (
              <div
                key={proof.id}
                onClick={() => setSelectedProof(proof.id)}
                className={cn(
                  'p-3 rounded-lg border cursor-pointer transition-all',
                  isSelected ?
                    `bg-${color}-500/10 border-${color}-500/40 ring-2 ring-${color}-500/30` :
                    'bg-gray-800/50 border-gray-600/30 hover:border-gray-500/50'
                )}
              >
                <div className="flex items-center gap-2 mb-2">
                  <ProofIcon className={cn(
                    'h-4 w-4',
                    color === 'blue' ? 'text-blue-400' :
                    color === 'green' ? 'text-green-400' :
                    color === 'yellow' ? 'text-yellow-400' :
                    'text-purple-400'
                  )} />
                  <span className="text-white font-medium text-sm">{proof.type}</span>
                  <Badge variant="outline" className={cn(
                    'text-xs',
                    proof.status === 'validated' ? 'bg-green-500/20 text-green-400' :
                    proof.status === 'validating' ? 'bg-yellow-500/20 text-yellow-400' :
                    proof.status === 'failed' ? 'bg-red-500/20 text-red-400' :
                    'bg-gray-500/20 text-gray-400'
                  )}>
                    {proof.status}
                  </Badge>
                </div>
                <div className="text-xs text-gray-400">
                  Block: {proof.blockId.slice(0, 8)}...
                </div>
                <div className="text-xs text-gray-400">
                  {proof.validationTime.toFixed(0)}ms | {proof.confidence.toFixed(1)}%
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

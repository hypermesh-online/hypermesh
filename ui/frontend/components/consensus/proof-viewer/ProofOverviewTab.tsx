// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { getProofIcon, getProofDescription, getProofColor } from './utils';
import type { ProofData } from './types';

interface ProofOverviewTabProps {
  proof: ProofData;
}

export function ProofOverviewTab({ proof }: ProofOverviewTabProps) {
  const color = getProofColor(proof.type);
  const iconColorClass = color === 'blue' ? 'text-blue-400' :
    color === 'green' ? 'text-green-400' :
    color === 'yellow' ? 'text-yellow-400' :
    'text-purple-400';

  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          {React.createElement(getProofIcon(proof.type), {
            className: `h-5 w-5 ${iconColorClass}`
          })}
          {proof.type} Proof Analysis
        </CardTitle>
        <CardDescription className="text-gray-400">
          {getProofDescription(proof.type)}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Proof Metadata */}
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-3">
            <MetadataField label="Proof ID" value={`${proof.id.slice(0, 16)}...`} mono />
            <MetadataField label="Asset ID" value={`${proof.assetId.slice(0, 12)}...`} mono />
            <MetadataField label="Block ID" value={`${proof.blockId.slice(0, 12)}...`} mono />
            <MetadataField label="Validator Node" value={proof.validatorNode} mono />
          </div>
          <div className="space-y-3">
            <div>
              <span className="text-gray-400 text-sm">Status:</span>
              <div className="flex items-center gap-2 mt-1">
                <Badge variant="outline" className={cn(
                  'text-xs',
                  proof.status === 'validated' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                  proof.status === 'validating' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                  proof.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                  'bg-gray-500/20 text-gray-400 border-gray-500/30'
                )}>
                  {proof.status}
                </Badge>
              </div>
            </div>
            <MetadataField label="Validation Time" value={`${proof.validationTime.toFixed(2)}ms`} mono />
            <div>
              <span className="text-gray-400 text-sm">Confidence Score:</span>
              <div className={cn(
                'font-mono text-sm',
                proof.confidence >= 90 ? 'text-green-400' :
                proof.confidence >= 70 ? 'text-yellow-400' :
                'text-red-400'
              )}>
                {proof.confidence.toFixed(1)}%
              </div>
            </div>
            <MetadataField label="Timestamp" value={new Date(proof.timestamp).toLocaleString()} />
          </div>
        </div>

        {/* Confidence Progress */}
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-gray-400">Validation Confidence</span>
            <span className={cn(
              'font-medium',
              proof.confidence >= 90 ? 'text-green-400' :
              proof.confidence >= 70 ? 'text-yellow-400' :
              'text-red-400'
            )}>
              {proof.confidence.toFixed(1)}%
            </span>
          </div>
          <Progress value={proof.confidence} className="h-2" />
        </div>
      </CardContent>
    </Card>
  );
}

function MetadataField({
  label,
  value,
  mono = false
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <span className="text-gray-400 text-sm">{label}:</span>
      <div className={cn('text-white text-sm', mono && 'font-mono')}>{value}</div>
    </div>
  );
}

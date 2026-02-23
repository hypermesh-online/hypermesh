// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { Layers } from 'lucide-react';
import { getOperationIcon, getProofIcon } from './utils';
import type { ConsensusOperation } from './types';

interface OperationsTabProps {
  activeOperations: ConsensusOperation[];
  selectedOperation: string | null;
  setSelectedOperation: (id: string | null) => void;
}

export function OperationsTab({
  activeOperations,
  selectedOperation,
  setSelectedOperation
}: OperationsTabProps) {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Layers className="h-5 w-5 text-cyan-400" />
          Asset Operations Tracking
        </CardTitle>
        <CardDescription className="text-gray-400">
          Monitor asset operations requiring four-proof consensus validation
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {activeOperations.map((operation) => {
            const OperationIcon = getOperationIcon(operation.type);
            const completionRate = (operation.completedProofs.length / operation.requiredProofs.length) * 100;

            return (
              <div
                key={operation.id}
                className={cn(
                  'p-4 rounded-lg border cursor-pointer transition-all',
                  operation.status === 'completed' ? 'bg-green-500/5 border-green-500/30' :
                  operation.status === 'in_progress' ? 'bg-blue-500/5 border-blue-500/30' :
                  operation.status === 'failed' ? 'bg-red-500/5 border-red-500/30' :
                  'bg-gray-500/5 border-gray-600/30',
                  selectedOperation === operation.id ? 'ring-2 ring-cyan-500/30' : ''
                )}
                onClick={() => setSelectedOperation(
                  selectedOperation === operation.id ? null : operation.id
                )}
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <OperationIcon className="h-5 w-5 text-cyan-400" />
                    <div>
                      <h4 className="text-white font-medium">
                        {operation.type.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())}
                      </h4>
                      <p className="text-sm text-gray-400">
                        Asset: {operation.assetId.slice(0, 12)}...
                      </p>
                    </div>
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      operation.status === 'completed' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                      operation.status === 'in_progress' ? 'bg-blue-500/20 text-blue-400 border-blue-500/30' :
                      operation.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                      'bg-gray-500/20 text-gray-400 border-gray-500/30'
                    )}>
                      {operation.status.replace('_', ' ')}
                    </Badge>
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      operation.priority === 'critical' ? 'bg-red-500/20 text-red-400' :
                      operation.priority === 'high' ? 'bg-orange-500/20 text-orange-400' :
                      operation.priority === 'medium' ? 'bg-yellow-500/20 text-yellow-400' :
                      'bg-gray-500/20 text-gray-400'
                    )}>
                      {operation.priority}
                    </Badge>
                  </div>
                  <div className="text-right">
                    <div className="text-cyan-400 font-medium">
                      {completionRate.toFixed(0)}%
                    </div>
                    <div className="text-xs text-gray-400">
                      {operation.completedProofs.length}/{operation.requiredProofs.length} proofs
                    </div>
                  </div>
                </div>

                <div className="space-y-3">
                  <Progress value={completionRate} className="h-2" />

                  <div className="flex items-center justify-between">
                    <div className="flex gap-2">
                      {operation.requiredProofs.map((proofType) => {
                        const ProofIcon = getProofIcon(proofType);
                        const isCompleted = operation.completedProofs.includes(proofType);

                        return (
                          <div
                            key={proofType}
                            className={cn(
                              'flex items-center gap-1 px-2 py-1 rounded text-xs',
                              isCompleted ? 'bg-green-500/20 text-green-400' : 'bg-gray-500/20 text-gray-400'
                            )}
                          >
                            <ProofIcon className="h-3 w-3" />
                            {proofType}
                          </div>
                        );
                      })}
                    </div>
                    <div className="text-xs text-gray-400">
                      Started: {new Date(operation.startTime).toLocaleTimeString()}
                      {operation.estimatedCompletion && (
                        <span className="ml-2">
                          ETA: {new Date(operation.estimatedCompletion).toLocaleTimeString()}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

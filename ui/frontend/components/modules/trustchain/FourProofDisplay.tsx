// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Eye } from 'lucide-react';
import { cn } from '@/lib/utils';

interface StateProofMetrics {
  proofCoverage: {
    space: number;
    stake: number;
    work: number;
    time: number;
  };
}

interface ProofType {
  type: 'space' | 'stake' | 'work' | 'time';
  name: string;
  description: string;
  color: string;
  bgColor: string;
}

interface FourProofDisplayProps {
  stateProofMetrics: StateProofMetrics;
  onViewDetails: (proofType: ProofType['type']) => void;
}

const proofTypes: Record<string, ProofType> = {
  space: {
    type: 'space',
    name: 'Proof of Space (PoSp)',
    description: 'WHERE - Storage location and physical/network location validation',
    color: 'text-blue-600',
    bgColor: 'bg-blue-50'
  },
  stake: {
    type: 'stake',
    name: 'Proof of Stake (PoSt)',
    description: 'WHO - Ownership, access rights, and economic stake validation',
    color: 'text-green-600',
    bgColor: 'bg-green-50'
  },
  work: {
    type: 'work',
    name: 'Proof of Work (PoWk)',
    description: 'WHAT/HOW - Computational resources and processing validation',
    color: 'text-purple-600',
    bgColor: 'bg-purple-50'
  },
  time: {
    type: 'time',
    name: 'Proof of Time (PoTm)',
    description: 'WHEN - Temporal ordering and timestamp validation',
    color: 'text-yellow-600',
    bgColor: 'bg-yellow-50'
  }
};

const getHealthStatus = (coverage: number) => {
  if (coverage >= 95) return { status: 'Excellent', color: 'text-green-600', bg: 'bg-green-100' };
  if (coverage >= 85) return { status: 'Good', color: 'text-blue-600', bg: 'bg-blue-100' };
  if (coverage >= 70) return { status: 'Warning', color: 'text-yellow-600', bg: 'bg-yellow-100' };
  return { status: 'Critical', color: 'text-red-600', bg: 'bg-red-100' };
};

export function FourProofDisplay({ stateProofMetrics, onViewDetails }: FourProofDisplayProps) {
  const averageProofCoverage = Object.values(stateProofMetrics.proofCoverage)
    .reduce((acc, val) => acc + val, 0) / 4;

  return (
    <Card className="bg-gradient-to-r from-quantum-50 to-purple-50 border-quantum-200">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Proof of State Four-Proof System</CardTitle>
            <CardDescription>
              Every asset requires ALL FOUR proofs for state verification
            </CardDescription>
          </div>
          <div className="text-right">
            <div className="text-3xl font-bold text-quantum-600">
              {averageProofCoverage.toFixed(1)}%
            </div>
            <div className="text-sm text-muted-foreground">Average Coverage</div>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {Object.entries(proofTypes).map(([key, proof]) => {
            const coverage = stateProofMetrics.proofCoverage[key as keyof typeof stateProofMetrics.proofCoverage];
            const health = getHealthStatus(coverage);
            
            return (
              <div 
                key={key}
                className={cn("p-4 rounded-lg border-2 cursor-pointer transition-all hover:shadow-md", proof.bgColor)}
                onClick={() => onViewDetails(proof.type)}
              >
                <div className="flex items-center justify-between mb-2">
                  <Badge className={cn("border-white", proof.color, "bg-white")}>
                    {key.toUpperCase()}
                  </Badge>
                  <span className={cn("font-bold text-lg", proof.color)}>
                    {coverage.toFixed(1)}%
                  </span>
                </div>
                <h4 className="font-medium text-sm mb-1">{proof.name}</h4>
                <p className="text-xs text-muted-foreground mb-2">{proof.description}</p>
                <div className="space-y-1">
                  <Progress value={coverage} className="h-2" />
                  <div className="flex justify-between items-center">
                    <Badge variant="outline" className={cn("text-xs", health.color)}>
                      {health.status}
                    </Badge>
                    <Button variant="ghost" size="sm" className="h-6 px-2 text-xs">
                      <Eye className="h-3 w-3 mr-1" />
                      Details
                    </Button>
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
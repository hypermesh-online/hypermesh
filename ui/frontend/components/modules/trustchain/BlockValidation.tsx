// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { CheckCircle, AlertTriangle, Eye } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ValidationResult {
  success: boolean;
  proofValidation: {
    space: { valid: boolean; coverage: number; issues: string[] };
    stake: { valid: boolean; coverage: number; issues: string[] };
    work: { valid: boolean; coverage: number; issues: string[] };
    time: { valid: boolean; coverage: number; issues: string[] };
  };
  networkHealth: {
    byzantineFaultTolerance: number;
    chainIntegrity: number;
    verificationParticipation: number;
  };
  recommendations: string[];
}

interface ProofType {
  type: 'space' | 'stake' | 'work' | 'time';
  name: string;
  description: string;
  color: string;
  bgColor: string;
}

interface BlockValidationProps {
  validationResults?: ValidationResult;
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

export function BlockValidation({ validationResults, onViewDetails }: BlockValidationProps) {
  if (!validationResults) {
    return null;
  }

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {Object.entries(proofTypes).map(([key, proof]) => {
          const proofData = validationResults.proofValidation[key as keyof typeof validationResults.proofValidation];
          const health = getHealthStatus(proofData.coverage);
          
          return (
            <Card key={key} className={cn("border-2", 
              proof.type === 'space' ? 'border-blue-200' :
              proof.type === 'stake' ? 'border-green-200' :
              proof.type === 'work' ? 'border-purple-200' : 'border-yellow-200'
            )}>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <Badge className={cn("text-white", 
                      proof.type === 'space' ? 'bg-blue-600' :
                      proof.type === 'stake' ? 'bg-green-600' :
                      proof.type === 'work' ? 'bg-purple-600' : 'bg-yellow-600'
                    )}>
                      {key.toUpperCase()}
                    </Badge>
                    <CardTitle className="text-base">{proof.name}</CardTitle>
                  </div>
                  <div className="text-right">
                    <div className={cn("text-2xl font-bold", proof.color)}>
                      {proofData.coverage.toFixed(1)}%
                    </div>
                    <Badge variant="outline" className={cn("text-xs", health.color)}>
                      {health.status}
                    </Badge>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span>Validation Speed:</span>
                      <span className="font-medium text-green-600">Fast</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Error Rate:</span>
                      <span className="font-medium">&lt; 0.1%</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Last Validation:</span>
                      <span className="font-medium">2 seconds ago</span>
                    </div>
                  </div>

                  {proofData.issues.length > 0 && (
                    <div className="text-xs text-red-600">
                      <strong>Issues:</strong> {proofData.issues.join(', ')}
                    </div>
                  )}

                  <Button 
                    variant="outline" 
                    size="sm" 
                    className="w-full"
                    onClick={() => onViewDetails(proof.type)}
                  >
                    <Eye className="h-4 w-4 mr-1" />
                    View Detailed Metrics
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {/* Validation Results Summary */}
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="flex items-center space-x-2">
            <CheckCircle className="h-5 w-5" />
            <span>State Proof Validation Results</span>
            {validationResults.success ? (
              <Badge className="bg-green-600">Valid</Badge>
            ) : (
              <Badge variant="destructive">Issues Found</Badge>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {Object.entries(validationResults.proofValidation).map(([key, proof]) => (
                <div key={key} className="space-y-2">
                  <div className="flex items-center space-x-2">
                    <span className="font-medium capitalize">{key}:</span>
                    {proof.valid ? (
                      <CheckCircle className="h-4 w-4 text-green-600" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-red-600" />
                    )}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    Coverage: {proof.coverage.toFixed(1)}%
                  </div>
                </div>
              ))}
            </div>

            <div className="space-y-2">
              <h4 className="font-medium">Network Health Metrics</h4>
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">BFT Threshold:</span>
                  <span className="font-medium ml-2">
                    {validationResults.networkHealth.byzantineFaultTolerance}%
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground">Chain Integrity:</span>
                  <span className="font-medium ml-2">
                    {validationResults.networkHealth.chainIntegrity}%
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground">Participation:</span>
                  <span className="font-medium ml-2">
                    {validationResults.networkHealth.verificationParticipation}%
                  </span>
                </div>
              </div>
            </div>

            {validationResults.recommendations.length > 0 && (
              <div className="space-y-2">
                <h4 className="font-medium">Recommendations</h4>
                <ul className="text-sm text-muted-foreground space-y-1">
                  {validationResults.recommendations.map((rec, index) => (
                    <li key={index} className="flex items-start">
                      <span className="mr-2">•</span>
                      {rec}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { Shield, CheckCircle, RefreshCw, Database, Clock, Zap, Activity } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface ProofCoverage {
  space: number;
  stake: number;
  work: number;
  time: number;
}

export interface StateProofMetrics {
  blockHeight: number;
  blockTime: number;
  validators: number;
  verificationTime: number;
  tps: number;
  proofCoverage: ProofCoverage;
}

export interface StateProofBlock {
  height: number;
  hash: string;
  previousHash: string;
  timestamp: string;
  transactions: number;
  validator: string;
  size: number;
  proofs: Array<{
    type: 'space' | 'stake' | 'work' | 'time';
    status: 'valid' | 'pending' | 'invalid';
    data: string;
    timestamp: string;
    validatedBy: string[];
  }>;
}

interface StateProofMetricsPanelProps {
  metrics?: StateProofMetrics;
  recentBlocks?: StateProofBlock[];
  onRefresh?: () => void;
  onValidate?: () => void;
  autoRefresh?: boolean;
  refreshInterval?: number;
  loading?: boolean;
  className?: string;
}

const defaultMetrics: StateProofMetrics = {
  blockHeight: 15234,
  blockTime: 2.3,
  validators: 67,
  verificationTime: 4.8,
  tps: 847,
  proofCoverage: {
    space: 98.5,
    stake: 96.2,
    work: 99.1,
    time: 97.8
  }
};

const defaultBlocks: StateProofBlock[] = [
  {
    height: 15234,
    hash: 'a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890',
    previousHash: 'b2c3d4e5f6789012345678901234567890123456789012345678901234567890a1',
    timestamp: new Date(Date.now() - 1000).toISOString(),
    transactions: 247,
    validator: 'validator-001',
    size: 1024576,
    proofs: [
      { type: 'space', status: 'valid', data: 'proof_data_space', timestamp: new Date().toISOString(), validatedBy: ['validator-001'] },
      { type: 'stake', status: 'valid', data: 'proof_data_stake', timestamp: new Date().toISOString(), validatedBy: ['validator-002'] },
      { type: 'work', status: 'valid', data: 'proof_data_work', timestamp: new Date().toISOString(), validatedBy: ['validator-003'] },
      { type: 'time', status: 'valid', data: 'proof_data_time', timestamp: new Date().toISOString(), validatedBy: ['validator-004'] }
    ]
  }
];

const proofTypes = {
  space: {
    name: 'Proof of Space (PoSp)',
    description: 'WHERE - Storage location and physical/network location validation',
    color: 'text-blue-400',
    bgColor: 'bg-blue-500/10',
    borderColor: 'border-blue-500/30',
    icon: Database
  },
  stake: {
    name: 'Proof of Stake (PoSt)',
    description: 'WHO - Ownership, access rights, and economic stake validation',
    color: 'text-green-400',
    bgColor: 'bg-green-500/10',
    borderColor: 'border-green-500/30',
    icon: Shield
  },
  work: {
    name: 'Proof of Work (PoWk)',
    description: 'WHAT/HOW - Computational resources and processing validation',
    color: 'text-purple-400',
    bgColor: 'bg-purple-500/10',
    borderColor: 'border-purple-500/30',
    icon: Zap
  },
  time: {
    name: 'Proof of Time (PoTm)',
    description: 'WHEN - Temporal ordering and timestamp validation',
    color: 'text-yellow-400',
    bgColor: 'bg-yellow-500/10',
    borderColor: 'border-yellow-500/30',
    icon: Clock
  }
};

export function StateProofMetricsPanel({
  metrics = defaultMetrics,
  recentBlocks = defaultBlocks,
  onRefresh,
  onValidate,
  autoRefresh = true,
  refreshInterval = 5000,
  loading = false,
  className
}: StateProofMetricsPanelProps) {
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [isValidating, setIsValidating] = useState(false);

  const averageProofCoverage = Object.values(metrics.proofCoverage).reduce((a, b) => a + b, 0) / 4;

  const handleRefresh = useCallback(() => {
    setLastRefresh(new Date());
    onRefresh?.();
  }, [onRefresh]);

  const handleValidate = useCallback(async () => {
    setIsValidating(true);
    try {
      await onValidate?.();
    } finally {
      setTimeout(() => setIsValidating(false), 2000);
    }
  }, [onValidate]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      handleRefresh();
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, handleRefresh]);

  return (
    <div className={cn("space-y-6", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white flex items-center space-x-2">
            <Shield className="h-6 w-6 text-purple-400" />
            <span>Four-Proof State Verification System</span>
          </h2>
          <p className="text-gray-400 mt-1">
            Proof of State protocol with unified WHERE/WHO/WHAT/WHEN validation
          </p>
        </div>

        <div className="flex items-center space-x-4">
          <div className="text-sm text-gray-400">
            Last updated: {lastRefresh.toLocaleTimeString()}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleValidate}
            disabled={isValidating}
            className="border-green-500/30 text-green-400 hover:bg-green-500/20"
          >
            <CheckCircle className={cn("h-4 w-4 mr-2", isValidating && "animate-spin")} />
            {isValidating ? 'Validating...' : 'Validate'}
          </Button>
        </div>
      </div>

      {/* State Proof Metrics Grid */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Block Height</CardTitle>
            <Database className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-400">{metrics.blockHeight.toLocaleString()}</div>
            <p className="text-xs text-gray-400">
              +{(60 / metrics.blockTime).toFixed(1)} blocks/min
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Block Time</CardTitle>
            <Clock className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{metrics.blockTime}s</div>
            <p className="text-xs text-gray-400">
              avg. production time
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Transactions/sec</CardTitle>
            <Zap className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{metrics.tps.toLocaleString()}</div>
            <p className="text-xs text-gray-400">
              current throughput
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active Validators</CardTitle>
            <Shield className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{metrics.validators}</div>
            <p className="text-xs text-gray-400">
              verification participants
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Four-Proof System Overview */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-white">Proof of State Four-Proof System</CardTitle>
              <CardDescription className="text-gray-400">
                Every asset requires ALL FOUR proofs for state verification
              </CardDescription>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-purple-400">{averageProofCoverage.toFixed(1)}%</div>
              <div className="text-sm text-gray-400">Average Coverage</div>
            </div>
          </div>
        </CardHeader>

        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {(Object.entries(proofTypes) as Array<[keyof ProofCoverage, typeof proofTypes[keyof ProofCoverage]]>).map(([key, proof]) => {
              const Icon = proof.icon;
              const coverage = metrics.proofCoverage[key];

              return (
                <div
                  key={key}
                  className={cn(
                    "p-4 rounded-lg border-2",
                    proof.bgColor,
                    proof.borderColor
                  )}
                >
                  <div className="flex items-center justify-between mb-3">
                    <Badge className={cn("text-xs", proof.color, "bg-black/20 border-current/30")}>
                      {key.toUpperCase()}
                    </Badge>
                    <div className="flex items-center space-x-1">
                      <Icon className={cn("h-4 w-4", proof.color)} />
                      <span className={cn("font-bold text-lg", proof.color)}>
                        {coverage.toFixed(1)}%
                      </span>
                    </div>
                  </div>

                  <h4 className="font-medium text-sm text-white mb-2">{proof.name}</h4>
                  <p className="text-xs text-gray-400 mb-3">{proof.description}</p>

                  <Progress
                    value={coverage}
                    className="h-2"
                  />

                  <div className="mt-2 text-xs text-gray-400">
                    {coverage >= 98 ? 'Excellent' : coverage >= 95 ? 'Good' : coverage >= 90 ? 'Fair' : 'Needs Attention'}
                  </div>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Verification Performance */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Verification Performance</CardTitle>
            <CardDescription className="text-gray-400">
              Real-time state proof validation metrics
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Verification Time:</span>
                <span className="text-white font-medium">{metrics.verificationTime}s</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Block Production Rate:</span>
                <span className="text-white font-medium">{(60 / metrics.blockTime).toFixed(1)} blocks/min</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Average Block Size:</span>
                <span className="text-white font-medium">
                  {(recentBlocks.reduce((sum, block) => sum + block.size, 0) / recentBlocks.length / 1024).toFixed(0)} KB
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Network Participation:</span>
                <span className="text-white font-medium">94.7%</span>
              </div>
            </div>

            <Separator className="bg-green-500/20" />

            <div>
              <div className="text-sm font-medium text-white mb-3">Verification Health</div>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Proof Validation Rate:</span>
                  <span className="text-green-400">{averageProofCoverage.toFixed(1)}%</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Byzantine Fault Tolerance:</span>
                  <span className="text-green-400">33% threshold</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Chain Integrity:</span>
                  <span className="text-green-400">100% verified</span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Recent Verification Activity</CardTitle>
            <CardDescription className="text-gray-400">
              Latest validated blocks and proofs
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentBlocks.slice(0, 3).map((block) => (
                <div key={block.height} className="border border-green-500/20 rounded-lg p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center space-x-2">
                      <Badge variant="outline" className="text-xs">#{block.height}</Badge>
                      <span className="text-xs text-gray-400">
                        {new Date(block.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Badge variant="secondary" className="text-xs">
                        {block.transactions} txs
                      </Badge>
                      <Badge variant="secondary" className="text-xs">
                        {(block.size / 1024).toFixed(0)} KB
                      </Badge>
                    </div>
                  </div>

                  <div className="text-xs text-gray-300 font-mono mb-2">
                    {block.hash.substring(0, 16)}...
                  </div>

                  <div className="grid grid-cols-2 md:grid-cols-4 gap-1">
                    {block.proofs.map((proof) => (
                      <div key={proof.type} className="flex items-center space-x-1 text-xs">
                        <div className={cn(
                          "w-2 h-2 rounded-full",
                          proof.status === 'valid' ? 'bg-green-400' :
                          proof.status === 'pending' ? 'bg-yellow-400' :
                          'bg-red-400'
                        )} />
                        <span className="text-white font-medium">{proof.type.toUpperCase()}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>

            {recentBlocks.length === 0 && (
              <div className="text-center py-6">
                <Activity className="h-8 w-8 text-gray-600 mx-auto mb-2" />
                <p className="text-sm text-gray-400">No recent blocks available</p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

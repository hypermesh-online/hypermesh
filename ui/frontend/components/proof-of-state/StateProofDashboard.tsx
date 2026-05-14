// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * State Proof Dashboard - Four-Proof state verification overview.
 *
 * HyperMesh uses Proof of State (bilateral binary authentication), NOT consensus.
 * Every asset requires all four proofs:
 *   PoSp (WHERE), PoSt (WHO), PoWk (WHAT/HOW), PoTm (WHEN).
 *
 * This dashboard shows node-level PoS status from the real IPC API.
 * Per-asset proof history is not yet exposed via IPC -- those sections show
 * "Coming soon" rather than fake data.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useNodeStatus,
  useChainValidation,
  useBlockchainHeight,
  useTrustchainIdentity,
} from '@/lib/hooks/useBlockMatrix';
import {
  Shield,
  Zap,
  Clock,
  HardDrive,
  Users,
  AlertTriangle,
  CheckCircle,
  Info,
} from 'lucide-react';

interface ProofTypeInfo {
  type: string;
  name: string;
  description: string;
  icon: React.ElementType;
  color: string;
}

const PROOF_TYPES: ProofTypeInfo[] = [
  {
    type: 'PoSp',
    name: 'Proof of Space',
    description: 'WHERE - Storage location and physical/network location verification',
    icon: HardDrive,
    color: 'blue',
  },
  {
    type: 'PoSt',
    name: 'Proof of Stake',
    description: 'WHO - Ownership, access rights, and economic stake validation',
    icon: Users,
    color: 'green',
  },
  {
    type: 'PoWk',
    name: 'Proof of Work',
    description: 'WHAT/HOW - Computational resources and processing validation',
    icon: Zap,
    color: 'yellow',
  },
  {
    type: 'PoTm',
    name: 'Proof of Time',
    description: 'WHEN - Temporal ordering and timestamp validation',
    icon: Clock,
    color: 'purple',
  },
];

function proofColor(color: string): string {
  switch (color) {
    case 'blue': return 'text-blue-400';
    case 'green': return 'text-green-400';
    case 'yellow': return 'text-yellow-400';
    case 'purple': return 'text-purple-400';
    default: return 'text-gray-400';
  }
}

function proofBorder(color: string): string {
  switch (color) {
    case 'blue': return 'border-blue-500/30';
    case 'green': return 'border-green-500/30';
    case 'yellow': return 'border-yellow-500/30';
    case 'purple': return 'border-purple-500/30';
    default: return 'border-gray-500/30';
  }
}

export function StateProofDashboard() {
  const { data: nodeStatus, isLoading: nodeLoading, error: nodeError } = useNodeStatus();
  const { data: chainValid, isLoading: chainLoading } = useChainValidation();
  const { data: heightData } = useBlockchainHeight();
  const { data: identity } = useTrustchainIdentity();

  if (nodeLoading) return <ModuleLoading />;

  if (nodeError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{nodeError.message}</p>
        </CardContent>
      </Card>
    );
  }

  const isValid = chainValid?.valid ?? false;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center py-4">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          Four-Proof State Verification
        </h1>
        <p className="text-gray-400 max-w-3xl mx-auto">
          Every asset requires all four proofs: PoSp (WHERE), PoSt (WHO), PoWk (WHAT/HOW),
          and PoTm (WHEN). Binary authentication -- something is either authentic or it is not.
        </p>
      </div>

      {/* System health cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Chain Validity</CardTitle>
            {isValid ? (
              <CheckCircle className="h-4 w-4 text-green-400" />
            ) : (
              <AlertTriangle className="h-4 w-4 text-yellow-400" />
            )}
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${isValid ? 'text-green-400' : 'text-yellow-400'}`}>
              {chainLoading ? '...' : isValid ? 'Valid' : 'Checking'}
            </div>
            <p className="text-xs text-gray-400">
              {heightData ? `${heightData.height} blocks verified` : 'Local blockchain'}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Privacy Mode</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">
              {nodeStatus?.privacy_mode ?? '--'}
            </div>
            <p className="text-xs text-gray-400">Transport privacy tier</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Identity</CardTitle>
            <Shield className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-lg font-bold text-cyan-400 font-mono truncate">
              {identity?.falcon.key_algorithm ?? 'FALCON-1024'}
            </div>
            <p className="text-xs text-gray-400">
              {identity?.node_id
                ? `Node: ${identity.node_id.slice(0, 12)}...`
                : 'Signing algorithm'}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Four Proof Types */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Shield className="h-5 w-5 text-cyan-400" />
            The Four Proofs
          </CardTitle>
          <CardDescription className="text-gray-400">
            Each proof dimension that every asset claim must satisfy
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2">
            {PROOF_TYPES.map((proof) => {
              const Icon = proof.icon;
              return (
                <div
                  key={proof.type}
                  className={`p-4 rounded-lg border ${proofBorder(proof.color)} bg-black/20`}
                >
                  <div className="flex items-center gap-3 mb-2">
                    <Icon className={`h-5 w-5 ${proofColor(proof.color)}`} />
                    <h4 className="text-white font-medium">
                      {proof.name}{' '}
                      <span className={`text-sm ${proofColor(proof.color)}`}>({proof.type})</span>
                    </h4>
                  </div>
                  <p className="text-sm text-gray-400">{proof.description}</p>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Per-asset proof history -- not yet wired */}
      <Card className="bg-black/40 border-cyan-500/20 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Info className="h-5 w-5 text-gray-400" />
            Per-Asset Proof History
          </CardTitle>
          <CardDescription className="text-gray-400">
            Detailed validation history per asset
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8">
            <Info className="h-10 w-10 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">Coming Soon</h3>
            <p className="text-gray-400 text-sm max-w-md mx-auto">
              Per-asset proof history will be available once the IPC daemon exposes
              state proof validation logs. Use the CLI for now:{' '}
              <code className="text-cyan-400 bg-black/40 px-1 rounded">
                hypermesh blockchain validate
              </code>
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

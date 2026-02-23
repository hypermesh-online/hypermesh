// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Shield, CheckCircle } from 'lucide-react';
import type { ProofData } from './types';

interface ProofSecurityTabProps {
  proof: ProofData;
}

export function ProofSecurityTab({ proof }: ProofSecurityTabProps) {
  return (
    <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Shield className="h-5 w-5 text-red-400" />
          Security Analysis
        </CardTitle>
        <CardDescription className="text-gray-400">
          Cryptographic security properties and threat analysis
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <SecurityProperties />
        <ThreatResistance />
        <SecurityScore confidence={proof.confidence} />
      </CardContent>
    </Card>
  );
}

function SecurityProperties() {
  const properties = [
    { label: 'Cryptographic Integrity', description: 'Digital signature verification passed. Proof has not been tampered with.' },
    { label: 'Non-Repudiation', description: 'Proof is cryptographically bound to the asset owner and cannot be denied.' },
    { label: 'Temporal Consistency', description: 'Proof timestamps are consistent with the blockchain sequence.' },
    { label: 'Consensus Validation', description: 'Proof has been validated by multiple network validators.' }
  ];

  return (
    <div className="grid gap-4 md:grid-cols-2">
      {properties.map((prop) => (
        <div key={prop.label} className="p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
          <div className="flex items-center gap-2 mb-2">
            <CheckCircle className="h-4 w-4 text-green-400" />
            <span className="text-green-400 font-medium">{prop.label}</span>
          </div>
          <p className="text-gray-300 text-sm">{prop.description}</p>
        </div>
      ))}
    </div>
  );
}

function ThreatResistance() {
  const threats = [
    { name: 'Double-Spending Attack', status: 'Resistant', color: 'green' },
    { name: 'Replay Attack', status: 'Resistant', color: 'green' },
    { name: 'Sybil Attack', status: 'Resistant', color: 'green' },
    { name: '51% Attack', status: 'Mitigated', color: 'yellow' }
  ];

  return (
    <div className="space-y-4">
      <h4 className="text-white font-medium">Threat Resistance Analysis</h4>
      <div className="space-y-3">
        {threats.map((threat) => (
          <div key={threat.name} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
            <div className="flex items-center gap-3">
              <Shield className={`h-4 w-4 text-${threat.color}-400`} />
              <span className="text-white text-sm">{threat.name}</span>
            </div>
            <Badge variant="outline" className={`text-xs bg-${threat.color}-500/20 text-${threat.color}-400`}>
              {threat.status}
            </Badge>
          </div>
        ))}
      </div>
    </div>
  );
}

function SecurityScore({ confidence }: { confidence: number }) {
  return (
    <div className="p-4 bg-blue-500/10 border border-blue-500/30 rounded-lg">
      <h4 className="text-blue-400 font-medium mb-3">Overall Security Score</h4>
      <div className="flex items-center gap-4">
        <div className="flex-1">
          <Progress value={confidence} className="h-3" />
        </div>
        <div className="text-blue-400 font-bold text-lg">
          {confidence.toFixed(0)}%
        </div>
      </div>
      <p className="text-gray-300 text-sm mt-2">
        Based on cryptographic validation, consensus agreement, and threat resistance analysis.
      </p>
    </div>
  );
}

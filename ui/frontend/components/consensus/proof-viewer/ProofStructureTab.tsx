// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Code, Hash, Key, Lock, Binary, Cpu, Timer } from 'lucide-react';
import type { ProofData } from './types';

interface ProofStructureTabProps {
  proof: ProofData;
}

export function ProofStructureTab({ proof }: ProofStructureTabProps) {
  return (
    <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Code className="h-5 w-5 text-blue-400" />
          Cryptographic Proof Structure
        </CardTitle>
        <CardDescription className="text-gray-400">
          Detailed breakdown of the proof's cryptographic components
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <CryptoField
          icon={<Hash className="h-4 w-4 text-blue-400" />}
          label="Challenge"
          value={proof.proofStructure.challenge}
          description="Cryptographic challenge issued by the validator"
          colorClass="text-blue-400"
        />

        <CryptoField
          icon={<Key className="h-4 w-4 text-green-400" />}
          label="Response"
          value={proof.proofStructure.response}
          description="Proof response demonstrating satisfaction of the challenge"
          colorClass="text-green-400"
        />

        <CryptoField
          icon={<Lock className="h-4 w-4 text-purple-400" />}
          label="Digital Signature"
          value={proof.proofStructure.signature}
          description="Cryptographic signature ensuring proof authenticity"
          colorClass="text-purple-400"
        />

        {proof.proofStructure.merkleRoot && (
          <CryptoField
            icon={<Binary className="h-4 w-4 text-cyan-400" />}
            label="Merkle Root (PoSp)"
            value={proof.proofStructure.merkleRoot}
            description="Merkle tree root proving storage commitment"
            colorClass="text-cyan-400"
          />
        )}

        {proof.proofStructure.difficulty && (
          <div className="grid gap-4 md:grid-cols-2">
            <div className="p-4 bg-gray-800/50 rounded-lg">
              <div className="flex items-center gap-2 mb-2">
                <Cpu className="h-4 w-4 text-yellow-400" />
                <span className="text-white font-medium">Difficulty (PoWk)</span>
              </div>
              <div className="text-yellow-400 font-mono text-sm">
                {proof.proofStructure.difficulty.toLocaleString()}
              </div>
              <p className="text-gray-400 text-xs mt-2">
                Computational difficulty requirement
              </p>
            </div>
            <div className="p-4 bg-gray-800/50 rounded-lg">
              <div className="flex items-center gap-2 mb-2">
                <Hash className="h-4 w-4 text-yellow-400" />
                <span className="text-white font-medium">Nonce</span>
              </div>
              <div className="text-yellow-400 font-mono text-sm">
                {proof.proofStructure.nonce}
              </div>
              <p className="text-gray-400 text-xs mt-2">
                Solution nonce for the work puzzle
              </p>
            </div>
          </div>
        )}

        {proof.proofStructure.timestamp && (
          <div className="p-4 bg-gray-800/50 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <Timer className="h-4 w-4 text-purple-400" />
              <span className="text-white font-medium">Proof Timestamp (PoTm)</span>
            </div>
            <div className="text-purple-400 font-mono text-sm">
              {new Date(proof.proofStructure.timestamp).toISOString()}
            </div>
            <p className="text-gray-400 text-xs mt-2">
              Precise timestamp for temporal ordering proof
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function CryptoField({
  icon,
  label,
  value,
  description,
  colorClass
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  description: string;
  colorClass: string;
}) {
  return (
    <div className="p-4 bg-gray-800/50 rounded-lg">
      <div className="flex items-center gap-2 mb-2">
        {icon}
        <span className="text-white font-medium">{label}</span>
      </div>
      <div className={`${colorClass} font-mono text-xs break-all`}>
        {value}
      </div>
      <p className="text-gray-400 text-xs mt-2">{description}</p>
    </div>
  );
}

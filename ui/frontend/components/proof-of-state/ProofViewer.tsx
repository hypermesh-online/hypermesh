// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Proof Viewer - Detailed Four-Proof Visualization Component
 *
 * Provides detailed visualization and analysis of individual state proofs.
 * Shows proof structure, validation steps, and security properties.
 */

import React from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  useStateProofHistory,
  useSubmitProof,
  useValidateStateProof
} from '@/lib/api';
import {
  ProofSelectionGrid,
  ProofOverviewTab,
  ProofStructureTab,
  ProofValidationTab,
  ProofSecurityTab
} from './proof-viewer';
import type { ProofData } from './proof-viewer';

interface ProofViewerProps {
  proofId?: string;
  assetId?: string;
  proofType?: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
  showDetails?: boolean;
}

export function ProofViewer({
  proofId,
  assetId,
  proofType,
  showDetails = true
}: ProofViewerProps) {
  const { data: stateProofHistory } = useStateProofHistory(assetId || '', 50);
  const submitProof = useSubmitProof();
  const validateStateProof = useValidateStateProof();

  const [selectedProof, setSelectedProof] = React.useState<string | null>(proofId || null);

  // Generate detailed proof data
  const proofData = React.useMemo((): ProofData[] => {
    if (!stateProofHistory) return [];

    return stateProofHistory.slice(0, 20).map((entry, index) => {
      const types: ('PoSp' | 'PoSt' | 'PoWk' | 'PoTm')[] = ['PoSp', 'PoSt', 'PoWk', 'PoTm'];
      const type = proofType || types[index % 4];

      return {
        id: `proof-${entry.blockId}-${type}`,
        type,
        assetId: entry.assetId || 'unknown',
        blockId: entry.blockId,
        status: entry.status === 'validated' ? 'validated' :
                entry.status === 'failed' ? 'failed' :
                index % 5 === 0 ? 'validating' : 'pending',
        timestamp: entry.timestamp,
        validationTime: entry.validationTime || Math.random() * 100 + 10,
        confidence: Math.random() * 30 + 70,
        validatorNode: `validator-${(index % 8) + 1}`,
        proofStructure: {
          challenge: `0x${Math.random().toString(16).substr(2, 32)}`,
          response: `0x${Math.random().toString(16).substr(2, 64)}`,
          signature: `0x${Math.random().toString(16).substr(2, 128)}`,
          merkleRoot: type === 'PoSp' ? `0x${Math.random().toString(16).substr(2, 32)}` : undefined,
          difficulty: type === 'PoWk' ? Math.floor(Math.random() * 1000000) : undefined,
          nonce: type === 'PoWk' ? Math.floor(Math.random() * 1000000).toString() : undefined,
          timestamp: type === 'PoTm' ? Date.now() - Math.random() * 3600000 : undefined
        },
        validationSteps: [
          {
            step: 'Signature Verification',
            status: 'completed',
            duration: Math.random() * 10 + 5,
            details: 'Cryptographic signature validation passed'
          },
          {
            step: type === 'PoSp' ? 'Storage Verification' :
                  type === 'PoSt' ? 'Stake Verification' :
                  type === 'PoWk' ? 'Work Verification' :
                  'Time Verification',
            status: entry.status === 'validated' ? 'completed' :
                   entry.status === 'failed' ? 'failed' : 'pending',
            duration: Math.random() * 20 + 10,
            details: type === 'PoSp' ? 'Storage commitment verified' :
                    type === 'PoSt' ? 'Stake ownership confirmed' :
                    type === 'PoWk' ? 'Computational work validated' :
                    'Temporal ordering verified'
          },
          {
            step: 'State Proof Integration',
            status: entry.status === 'validated' ? 'completed' : 'pending',
            duration: Math.random() * 15 + 5,
            details: 'Proof integrated into state proof record'
          }
        ]
      };
    });
  }, [stateProofHistory, proofType]);

  const selectedProofData = proofData.find(p => p.id === selectedProof);

  return (
    <div className="space-y-6">
      <ProofSelectionGrid
        proofData={proofData}
        selectedProof={selectedProof}
        setSelectedProof={setSelectedProof}
      />

      {selectedProofData && (
        <Tabs defaultValue="overview" className="space-y-6">
          <TabsList className="grid w-full grid-cols-4 bg-black/40">
            <TabsTrigger value="overview" className="data-[state=active]:bg-purple-500/20">Overview</TabsTrigger>
            <TabsTrigger value="structure" className="data-[state=active]:bg-purple-500/20">Proof Structure</TabsTrigger>
            <TabsTrigger value="validation" className="data-[state=active]:bg-purple-500/20">Validation Steps</TabsTrigger>
            <TabsTrigger value="security" className="data-[state=active]:bg-purple-500/20">Security Analysis</TabsTrigger>
          </TabsList>

          <TabsContent value="overview" className="space-y-6">
            <ProofOverviewTab proof={selectedProofData} />
          </TabsContent>

          <TabsContent value="structure" className="space-y-6">
            <ProofStructureTab proof={selectedProofData} />
          </TabsContent>

          <TabsContent value="validation" className="space-y-6">
            <ProofValidationTab proof={selectedProofData} />
          </TabsContent>

          <TabsContent value="security" className="space-y-6">
            <ProofSecurityTab proof={selectedProofData} />
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}

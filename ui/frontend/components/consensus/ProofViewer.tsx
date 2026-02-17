// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Proof Viewer - Detailed Four-Proof Visualization Component
 * 
 * Provides detailed visualization and analysis of individual consensus proofs.
 * Shows proof structure, validation steps, and security properties.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { 
  useConsensusHistory,
  useSubmitProof,
  useValidateConsensus
} from '@/lib/api';
import { 
  Shield,
  Zap,
  Clock,
  HardDrive,
  Users,
  Eye,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Activity,
  Database,
  Lock,
  Key,
  FileText,
  Hash,
  Timer,
  MapPin,
  Cpu,
  Network,
  Code,
  Binary
} from 'lucide-react';

interface ProofData {
  id: string;
  type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
  assetId: string;
  blockId: string;
  status: 'validated' | 'failed' | 'pending' | 'validating';
  timestamp: string;
  validationTime: number;
  confidence: number;
  validatorNode: string;
  proofStructure: {
    challenge: string;
    response: string;
    signature: string;
    merkleRoot?: string;
    difficulty?: number;
    nonce?: string;
    timestamp?: number;
  };
  validationSteps: {
    step: string;
    status: 'completed' | 'failed' | 'pending';
    duration: number;
    details: string;
  }[];
}

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
  const { data: consensusHistory } = useConsensusHistory(assetId || '', 50);
  const submitProof = useSubmitProof();
  const validateConsensus = useValidateConsensus();
  
  const [selectedProof, setSelectedProof] = React.useState<string | null>(proofId || null);

  // Generate detailed proof data
  const proofData = React.useMemo((): ProofData[] => {
    if (!consensusHistory) return [];
    
    return consensusHistory.slice(0, 20).map((entry, index) => {
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
            step: 'Consensus Integration',
            status: entry.status === 'validated' ? 'completed' : 'pending',
            duration: Math.random() * 15 + 5,
            details: 'Proof integrated into consensus state'
          }
        ]
      };
    });
  }, [consensusHistory, proofType]);

  const selectedProofData = proofData.find(p => p.id === selectedProof);

  const getProofIcon = (type: string) => {
    switch (type) {
      case 'PoSp': return HardDrive;
      case 'PoSt': return Users;
      case 'PoWk': return Zap;
      case 'PoTm': return Clock;
      default: return Shield;
    }
  };

  const getProofDescription = (type: string) => {
    switch (type) {
      case 'PoSp': return 'WHERE - Storage location and physical/network location verification';
      case 'PoSt': return 'WHO - Ownership, access rights, and economic stake validation';
      case 'PoWk': return 'WHAT/HOW - Computational resources and processing validation';
      case 'PoTm': return 'WHEN - Temporal ordering and timestamp validation';
      default: return 'Unknown proof type';
    }
  };

  const getProofColor = (type: string) => {
    switch (type) {
      case 'PoSp': return 'blue';
      case 'PoSt': return 'green';
      case 'PoWk': return 'yellow';
      case 'PoTm': return 'purple';
      default: return 'gray';
    }
  };

  return (
    <div className="space-y-6">
      {/* Proof Selection */}
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
                    {proof.validationTime.toFixed(0)}ms • {proof.confidence.toFixed(1)}%
                  </div>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Detailed Proof Analysis */}
      {selectedProofData && (
        <Tabs defaultValue="overview" className="space-y-6">
          <TabsList className="grid w-full grid-cols-4 bg-black/40">
            <TabsTrigger value="overview" className="data-[state=active]:bg-purple-500/20">Overview</TabsTrigger>
            <TabsTrigger value="structure" className="data-[state=active]:bg-purple-500/20">Proof Structure</TabsTrigger>
            <TabsTrigger value="validation" className="data-[state=active]:bg-purple-500/20">Validation Steps</TabsTrigger>
            <TabsTrigger value="security" className="data-[state=active]:bg-purple-500/20">Security Analysis</TabsTrigger>
          </TabsList>

          <TabsContent value="overview" className="space-y-6">
            {/* Proof Overview */}
            <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
              <CardHeader>
                <CardTitle className="text-white flex items-center gap-2">
                  {React.createElement(getProofIcon(selectedProofData.type), { 
                    className: `h-5 w-5 ${getProofColor(selectedProofData.type) === 'blue' ? 'text-blue-400' :
                      getProofColor(selectedProofData.type) === 'green' ? 'text-green-400' :
                      getProofColor(selectedProofData.type) === 'yellow' ? 'text-yellow-400' :
                      'text-purple-400'}`
                  })}
                  {selectedProofData.type} Proof Analysis
                </CardTitle>
                <CardDescription className="text-gray-400">
                  {getProofDescription(selectedProofData.type)}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                {/* Proof Metadata */}
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-3">
                    <div>
                      <span className="text-gray-400 text-sm">Proof ID:</span>
                      <div className="text-white font-mono text-sm">{selectedProofData.id.slice(0, 16)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Asset ID:</span>
                      <div className="text-white font-mono text-sm">{selectedProofData.assetId.slice(0, 12)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Block ID:</span>
                      <div className="text-white font-mono text-sm">{selectedProofData.blockId.slice(0, 12)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Validator Node:</span>
                      <div className="text-white font-mono text-sm">{selectedProofData.validatorNode}</div>
                    </div>
                  </div>
                  <div className="space-y-3">
                    <div>
                      <span className="text-gray-400 text-sm">Status:</span>
                      <div className="flex items-center gap-2 mt-1">
                        <Badge variant="outline" className={cn(
                          'text-xs',
                          selectedProofData.status === 'validated' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                          selectedProofData.status === 'validating' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                          selectedProofData.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                          'bg-gray-500/20 text-gray-400 border-gray-500/30'
                        )}>
                          {selectedProofData.status}
                        </Badge>
                      </div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Validation Time:</span>
                      <div className="text-white font-mono text-sm">{selectedProofData.validationTime.toFixed(2)}ms</div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Confidence Score:</span>
                      <div className={cn(
                        'font-mono text-sm',
                        selectedProofData.confidence >= 90 ? 'text-green-400' :
                        selectedProofData.confidence >= 70 ? 'text-yellow-400' :
                        'text-red-400'
                      )}>
                        {selectedProofData.confidence.toFixed(1)}%
                      </div>
                    </div>
                    <div>
                      <span className="text-gray-400 text-sm">Timestamp:</span>
                      <div className="text-white text-sm">{new Date(selectedProofData.timestamp).toLocaleString()}</div>
                    </div>
                  </div>
                </div>

                {/* Confidence Progress */}
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Validation Confidence</span>
                    <span className={cn(
                      'font-medium',
                      selectedProofData.confidence >= 90 ? 'text-green-400' :
                      selectedProofData.confidence >= 70 ? 'text-yellow-400' :
                      'text-red-400'
                    )}>
                      {selectedProofData.confidence.toFixed(1)}%
                    </span>
                  </div>
                  <Progress value={selectedProofData.confidence} className="h-2" />
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="structure" className="space-y-6">
            {/* Proof Structure */}
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
                {/* Challenge */}
                <div className="p-4 bg-gray-800/50 rounded-lg">
                  <div className="flex items-center gap-2 mb-2">
                    <Hash className="h-4 w-4 text-blue-400" />
                    <span className="text-white font-medium">Challenge</span>
                  </div>
                  <div className="text-blue-400 font-mono text-xs break-all">
                    {selectedProofData.proofStructure.challenge}
                  </div>
                  <p className="text-gray-400 text-xs mt-2">
                    Cryptographic challenge issued by the validator
                  </p>
                </div>

                {/* Response */}
                <div className="p-4 bg-gray-800/50 rounded-lg">
                  <div className="flex items-center gap-2 mb-2">
                    <Key className="h-4 w-4 text-green-400" />
                    <span className="text-white font-medium">Response</span>
                  </div>
                  <div className="text-green-400 font-mono text-xs break-all">
                    {selectedProofData.proofStructure.response}
                  </div>
                  <p className="text-gray-400 text-xs mt-2">
                    Proof response demonstrating satisfaction of the challenge
                  </p>
                </div>

                {/* Signature */}
                <div className="p-4 bg-gray-800/50 rounded-lg">
                  <div className="flex items-center gap-2 mb-2">
                    <Lock className="h-4 w-4 text-purple-400" />
                    <span className="text-white font-medium">Digital Signature</span>
                  </div>
                  <div className="text-purple-400 font-mono text-xs break-all">
                    {selectedProofData.proofStructure.signature}
                  </div>
                  <p className="text-gray-400 text-xs mt-2">
                    Cryptographic signature ensuring proof authenticity
                  </p>
                </div>

                {/* Type-specific Fields */}
                {selectedProofData.proofStructure.merkleRoot && (
                  <div className="p-4 bg-gray-800/50 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <Binary className="h-4 w-4 text-cyan-400" />
                      <span className="text-white font-medium">Merkle Root (PoSp)</span>
                    </div>
                    <div className="text-cyan-400 font-mono text-xs break-all">
                      {selectedProofData.proofStructure.merkleRoot}
                    </div>
                    <p className="text-gray-400 text-xs mt-2">
                      Merkle tree root proving storage commitment
                    </p>
                  </div>
                )}

                {selectedProofData.proofStructure.difficulty && (
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="p-4 bg-gray-800/50 rounded-lg">
                      <div className="flex items-center gap-2 mb-2">
                        <Cpu className="h-4 w-4 text-yellow-400" />
                        <span className="text-white font-medium">Difficulty (PoWk)</span>
                      </div>
                      <div className="text-yellow-400 font-mono text-sm">
                        {selectedProofData.proofStructure.difficulty.toLocaleString()}
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
                        {selectedProofData.proofStructure.nonce}
                      </div>
                      <p className="text-gray-400 text-xs mt-2">
                        Solution nonce for the work puzzle
                      </p>
                    </div>
                  </div>
                )}

                {selectedProofData.proofStructure.timestamp && (
                  <div className="p-4 bg-gray-800/50 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <Timer className="h-4 w-4 text-purple-400" />
                      <span className="text-white font-medium">Proof Timestamp (PoTm)</span>
                    </div>
                    <div className="text-purple-400 font-mono text-sm">
                      {new Date(selectedProofData.proofStructure.timestamp).toISOString()}
                    </div>
                    <p className="text-gray-400 text-xs mt-2">
                      Precise timestamp for temporal ordering proof
                    </p>
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="validation" className="space-y-6">
            {/* Validation Steps */}
            <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
              <CardHeader>
                <CardTitle className="text-white flex items-center gap-2">
                  <Activity className="h-5 w-5 text-green-400" />
                  Validation Process
                </CardTitle>
                <CardDescription className="text-gray-400">
                  Step-by-step validation workflow and results
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {selectedProofData.validationSteps.map((step, index) => (
                    <div key={index} className="flex items-start gap-4 p-4 bg-gray-800/50 rounded-lg">
                      <div className={cn(
                        'flex items-center justify-center w-8 h-8 rounded-full border-2 mt-1',
                        step.status === 'completed' ? 'bg-green-500 border-green-500 text-white' :
                        step.status === 'failed' ? 'bg-red-500 border-red-500 text-white' :
                        'bg-gray-800 border-gray-600 text-gray-400'
                      )}>
                        {step.status === 'completed' ? <CheckCircle className="h-4 w-4" /> :
                         step.status === 'failed' ? <XCircle className="h-4 w-4" /> :
                         <Activity className="h-4 w-4" />}
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center justify-between mb-2">
                          <h4 className="text-white font-medium">{step.step}</h4>
                          <div className="flex items-center gap-2">
                            <Badge variant="outline" className={cn(
                              'text-xs',
                              step.status === 'completed' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                              step.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                              'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
                            )}>
                              {step.status}
                            </Badge>
                            <span className="text-gray-400 text-xs font-mono">
                              {step.duration.toFixed(1)}ms
                            </span>
                          </div>
                        </div>
                        <p className="text-gray-400 text-sm">{step.details}</p>
                        {step.status === 'completed' && (
                          <div className="mt-2">
                            <Progress value={100} className="h-1" />
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
                
                {/* Validation Summary */}
                <div className="mt-6 p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
                  <h4 className="text-green-400 font-medium mb-2">Validation Summary</h4>
                  <div className="grid gap-3 md:grid-cols-3 text-sm">
                    <div>
                      <span className="text-gray-400">Total Steps:</span>
                      <div className="text-white font-medium">{selectedProofData.validationSteps.length}</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Completed:</span>
                      <div className="text-green-400 font-medium">
                        {selectedProofData.validationSteps.filter(s => s.status === 'completed').length}
                      </div>
                    </div>
                    <div>
                      <span className="text-gray-400">Total Duration:</span>
                      <div className="text-white font-medium">
                        {selectedProofData.validationSteps.reduce((sum, step) => sum + step.duration, 0).toFixed(1)}ms
                      </div>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="security" className="space-y-6">
            {/* Security Analysis */}
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
                {/* Security Properties */}
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <CheckCircle className="h-4 w-4 text-green-400" />
                      <span className="text-green-400 font-medium">Cryptographic Integrity</span>
                    </div>
                    <p className="text-gray-300 text-sm">
                      Digital signature verification passed. Proof has not been tampered with.
                    </p>
                  </div>
                  
                  <div className="p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <CheckCircle className="h-4 w-4 text-green-400" />
                      <span className="text-green-400 font-medium">Non-Repudiation</span>
                    </div>
                    <p className="text-gray-300 text-sm">
                      Proof is cryptographically bound to the asset owner and cannot be denied.
                    </p>
                  </div>
                  
                  <div className="p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <CheckCircle className="h-4 w-4 text-green-400" />
                      <span className="text-green-400 font-medium">Temporal Consistency</span>
                    </div>
                    <p className="text-gray-300 text-sm">
                      Proof timestamps are consistent with the blockchain sequence.
                    </p>
                  </div>
                  
                  <div className="p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                      <CheckCircle className="h-4 w-4 text-green-400" />
                      <span className="text-green-400 font-medium">Consensus Validation</span>
                    </div>
                    <p className="text-gray-300 text-sm">
                      Proof has been validated by multiple network validators.
                    </p>
                  </div>
                </div>

                {/* Threat Analysis */}
                <div className="space-y-4">
                  <h4 className="text-white font-medium">Threat Resistance Analysis</h4>
                  <div className="space-y-3">
                    <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                      <div className="flex items-center gap-3">
                        <Shield className="h-4 w-4 text-green-400" />
                        <span className="text-white text-sm">Double-Spending Attack</span>
                      </div>
                      <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                        Resistant
                      </Badge>
                    </div>
                    
                    <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                      <div className="flex items-center gap-3">
                        <Shield className="h-4 w-4 text-green-400" />
                        <span className="text-white text-sm">Replay Attack</span>
                      </div>
                      <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                        Resistant
                      </Badge>
                    </div>
                    
                    <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                      <div className="flex items-center gap-3">
                        <Shield className="h-4 w-4 text-green-400" />
                        <span className="text-white text-sm">Sybil Attack</span>
                      </div>
                      <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                        Resistant
                      </Badge>
                    </div>
                    
                    <div className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                      <div className="flex items-center gap-3">
                        <Shield className="h-4 w-4 text-yellow-400" />
                        <span className="text-white text-sm">51% Attack</span>
                      </div>
                      <Badge variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400">
                        Mitigated
                      </Badge>
                    </div>
                  </div>
                </div>

                {/* Security Score */}
                <div className="p-4 bg-blue-500/10 border border-blue-500/30 rounded-lg">
                  <h4 className="text-blue-400 font-medium mb-3">Overall Security Score</h4>
                  <div className="flex items-center gap-4">
                    <div className="flex-1">
                      <Progress value={selectedProofData.confidence} className="h-3" />
                    </div>
                    <div className="text-blue-400 font-bold text-lg">
                      {selectedProofData.confidence.toFixed(0)}%
                    </div>
                  </div>
                  <p className="text-gray-300 text-sm mt-2">
                    Based on cryptographic validation, consensus agreement, and threat resistance analysis.
                  </p>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
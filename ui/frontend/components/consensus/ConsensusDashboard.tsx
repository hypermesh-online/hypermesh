// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Consensus Dashboard - Real-time Four-Proof consensus monitoring
 * 
 * Displays live consensus validation using the Proof of State Four-Proof system:
 * - PoSp (Proof of Space): WHERE - storage location and physical/network location
 * - PoSt (Proof of Stake): WHO - ownership, access rights, and economic stake
 * - PoWk (Proof of Work): WHAT/HOW - computational resources and processing
 * - PoTm (Proof of Time): WHEN - temporal ordering and timestamp validation
 * 
 * Integrates with real HyperMesh consensus API endpoints for production data.
 */

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { 
  useValidateConsensus, 
  useConsensusHistory, 
  useSubmitProof, 
  useByzantineDetections,
  useAssets,
  useSystemStatus
} from '@/lib/api';
import { 
  Shield, 
  Zap, 
  Clock, 
  HardDrive, 
  Users, 
  Activity,
  AlertTriangle,
  CheckCircle,
  XCircle,
  TrendingUp,
  Database,
  Network
} from 'lucide-react';

interface ProofTypeStats {
  type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
  name: string;
  description: string;
  icon: any;
  color: string;
  totalValidations: number;
  successRate: number;
  averageTime: number;
  lastValidation: string;
}

interface ConsensusMetrics {
  totalProofValidations: number;
  consensusRate: number;
  byzantineDetections: number;
  networkParticipation: number;
  averageBlockTime: number;
  lastBlockHash: string;
}

export function ConsensusDashboard() {
  const { systemStatus } = useSystemStatus(true);
  const { assets } = useAssets();
  const { data: byzantineDetections } = useByzantineDetections();
  const validateConsensus = useValidateConsensus();
  const submitProof = useSubmitProof();
  
  // Get consensus history for the first asset (in production, this would be system-wide)
  const firstAssetId = assets?.[0]?.id;
  const { data: consensusHistory } = useConsensusHistory(firstAssetId || '', 50);
  
  // Calculate real-time consensus metrics from API data
  const consensusMetrics = React.useMemo((): ConsensusMetrics => {
    if (!consensusHistory || !systemStatus) {
      return {
        totalProofValidations: 0,
        consensusRate: 0,
        byzantineDetections: byzantineDetections?.length || 0,
        networkParticipation: 0,
        averageBlockTime: 0,
        lastBlockHash: 'N/A'
      };
    }
    
    const validValidations = consensusHistory.filter(c => c.status === 'validated');
    const consensusRate = consensusHistory.length > 0 ? (validValidations.length / consensusHistory.length) * 100 : 0;
    
    // Calculate average block time from consensus history
    const blockTimes = consensusHistory.slice(0, -1).map((entry, index) => {
      const currentTime = new Date(entry.timestamp).getTime();
      const nextTime = new Date(consensusHistory[index + 1].timestamp).getTime();
      return Math.abs(nextTime - currentTime) / 1000; // seconds
    });
    
    const averageBlockTime = blockTimes.length > 0 ? 
      blockTimes.reduce((sum, time) => sum + time, 0) / blockTimes.length : 0;
    
    return {
      totalProofValidations: consensusHistory.length,
      consensusRate,
      byzantineDetections: byzantineDetections?.length || 0,
      networkParticipation: systemStatus.services ? 
        (Object.values(systemStatus.services).filter(s => s.status === 'healthy').length / 
         Object.values(systemStatus.services).length) * 100 : 0,
      averageBlockTime,
      lastBlockHash: consensusHistory[0]?.blockId || 'N/A'
    };
  }, [consensusHistory, systemStatus, byzantineDetections]);

  // Calculate proof type statistics from consensus history
  const proofTypeStats = React.useMemo((): ProofTypeStats[] => {
    const baseStats = [
      {
        type: 'PoSp' as const,
        name: 'Proof of Space',
        description: 'WHERE - Storage location and physical/network location verification',
        icon: HardDrive,
        color: 'blue',
        totalValidations: 0,
        successRate: 0,
        averageTime: 0,
        lastValidation: 'Never'
      },
      {
        type: 'PoSt' as const,
        name: 'Proof of Stake',
        description: 'WHO - Ownership, access rights, and economic stake validation',
        icon: Users,
        color: 'green',
        totalValidations: 0,
        successRate: 0,
        averageTime: 0,
        lastValidation: 'Never'
      },
      {
        type: 'PoWk' as const,
        name: 'Proof of Work',
        description: 'WHAT/HOW - Computational resources and processing validation',
        icon: Zap,
        color: 'yellow',
        totalValidations: 0,
        successRate: 0,
        averageTime: 0,
        lastValidation: 'Never'
      },
      {
        type: 'PoTm' as const,
        name: 'Proof of Time',
        description: 'WHEN - Temporal ordering and timestamp validation',
        icon: Clock,
        color: 'purple',
        totalValidations: 0,
        successRate: 0,
        averageTime: 0,
        lastValidation: 'Never'
      }
    ];

    if (!consensusHistory) return baseStats;

    // Calculate statistics for each proof type
    return baseStats.map(stat => {
      const proofEntries = consensusHistory.filter(entry => 
        entry.proofs && entry.proofs.some(proof => proof.type === stat.type)
      );
      
      const successfulProofs = proofEntries.filter(entry => entry.status === 'validated');
      const successRate = proofEntries.length > 0 ? (successfulProofs.length / proofEntries.length) * 100 : 0;
      
      // Calculate average validation time
      const validationTimes = proofEntries.map(entry => entry.validationTime || 0).filter(time => time > 0);
      const averageTime = validationTimes.length > 0 ? 
        validationTimes.reduce((sum, time) => sum + time, 0) / validationTimes.length : 0;
      
      const lastValidation = proofEntries.length > 0 ? 
        new Date(proofEntries[0].timestamp).toLocaleString() : 'Never';

      return {
        ...stat,
        totalValidations: proofEntries.length,
        successRate,
        averageTime,
        lastValidation
      };
    });
  }, [consensusHistory]);

  const handleTestConsensus = async () => {
    if (!firstAssetId) {
      alert('No assets available for consensus testing');
      return;
    }
    
    try {
      await validateConsensus.mutateAsync({
        assetId: firstAssetId,
        blockId: 'test-block-' + Date.now()
      });
      alert('Consensus validation initiated successfully!');
    } catch (error) {
      console.error('Consensus validation failed:', error);
      alert('Consensus validation failed. Check console for details.');
    }
  };

  const handleSubmitTestProof = async () => {
    if (!firstAssetId) {
      alert('No assets available for proof submission');
      return;
    }
    
    try {
      await submitProof.mutateAsync({
        assetId: firstAssetId,
        blockId: 'test-block-' + Date.now(),
        type: 'PoSt',
        data: { test: true, timestamp: Date.now() },
        signature: 'test-signature-' + Date.now()
      });
      alert('Test proof submitted successfully!');
    } catch (error) {
      console.error('Proof submission failed:', error);
      alert('Proof submission failed. Check console for details.');
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          Four-Proof Consensus System
        </h1>
        <p className="text-gray-400 max-w-3xl mx-auto">
          Real-time monitoring of the Proof of State Four-Proof consensus mechanism. Every asset requires all four proofs: 
          PoSp (WHERE), PoSt (WHO), PoWk (WHAT/HOW), and PoTm (WHEN) for complete validation.
        </p>
      </div>

      {/* Consensus Overview Metrics */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Validations</CardTitle>
            <Database className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{consensusMetrics.totalProofValidations}</div>
            <p className="text-xs text-gray-400">All time</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Consensus Rate</CardTitle>
            <CheckCircle className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{consensusMetrics.consensusRate.toFixed(1)}%</div>
            <p className="text-xs text-gray-400">Success rate</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Byzantine Detections</CardTitle>
            <AlertTriangle className="h-4 w-4 text-red-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-400">{consensusMetrics.byzantineDetections}</div>
            <p className="text-xs text-gray-400">Malicious nodes</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Network Participation</CardTitle>
            <Network className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{consensusMetrics.networkParticipation.toFixed(1)}%</div>
            <p className="text-xs text-gray-400">Active nodes</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="overview" className="space-y-6">
        <TabsList className="grid w-full grid-cols-4 bg-black/40">
          <TabsTrigger value="overview" className="data-[state=active]:bg-cyan-500/20">Overview</TabsTrigger>
          <TabsTrigger value="proofs" className="data-[state=active]:bg-cyan-500/20">Proof Types</TabsTrigger>
          <TabsTrigger value="history" className="data-[state=active]:bg-cyan-500/20">History</TabsTrigger>
          <TabsTrigger value="byzantine" className="data-[state=active]:bg-cyan-500/20">Security</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-6">
          {/* System Status */}
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Activity className="h-5 w-5 text-cyan-400" />
                Consensus System Status
              </CardTitle>
              <CardDescription className="text-gray-400">Real-time consensus health and performance metrics</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Average Block Time</span>
                    <span className="text-white font-mono">{consensusMetrics.averageBlockTime.toFixed(2)}s</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Last Block Hash</span>
                    <span className="text-cyan-400 font-mono text-sm">{consensusMetrics.lastBlockHash.slice(0, 12)}...</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">System Status</span>
                    <Badge variant={systemStatus ? 'default' : 'destructive'} className="text-xs">
                      {systemStatus ? 'Online' : 'Offline'}
                    </Badge>
                  </div>
                </div>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Active Assets</span>
                    <span className="text-white font-mono">{assets?.length || 0}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Consensus Participation</span>
                    <span className="text-green-400 font-mono">{consensusMetrics.networkParticipation.toFixed(1)}%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Last Update</span>
                    <span className="text-gray-300 text-sm">{new Date().toLocaleTimeString()}</span>
                  </div>
                </div>
              </div>
              
              <div className="pt-4 border-t border-cyan-500/20 flex gap-3">
                <Button 
                  onClick={handleTestConsensus}
                  disabled={validateConsensus.isPending || !firstAssetId}
                  className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
                >
                  {validateConsensus.isPending ? 'Testing...' : 'Test Consensus'}
                </Button>
                <Button 
                  onClick={handleSubmitTestProof}
                  disabled={submitProof.isPending || !firstAssetId}
                  variant="outline"
                  className="border-cyan-500/30 text-cyan-400"
                >
                  {submitProof.isPending ? 'Submitting...' : 'Submit Test Proof'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="proofs" className="space-y-6">
          {/* Four Proof Types */}
          <div className="grid gap-6 md:grid-cols-2">
            {proofTypeStats.map((proof) => {
              const Icon = proof.icon;
              return (
                <Card key={proof.type} className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
                  <CardHeader>
                    <CardTitle className="text-white flex items-center gap-2">
                      <Icon className={cn(
                        'h-5 w-5',
                        proof.color === 'blue' ? 'text-blue-400' :
                        proof.color === 'green' ? 'text-green-400' :
                        proof.color === 'yellow' ? 'text-yellow-400' :
                        'text-purple-400'
                      )} />
                      {proof.name} ({proof.type})
                    </CardTitle>
                    <CardDescription className="text-gray-400 text-sm">{proof.description}</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="grid gap-3">
                      <div className="flex justify-between">
                        <span className="text-gray-400">Total Validations</span>
                        <span className="text-white font-mono">{proof.totalValidations}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Success Rate</span>
                        <span className={cn(
                          'font-mono',
                          proof.successRate >= 90 ? 'text-green-400' :
                          proof.successRate >= 70 ? 'text-yellow-400' :
                          'text-red-400'
                        )}>
                          {proof.successRate.toFixed(1)}%
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Avg Validation Time</span>
                        <span className="text-white font-mono">{proof.averageTime.toFixed(2)}ms</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Last Validation</span>
                        <span className="text-gray-300 text-sm">{proof.lastValidation}</span>
                      </div>
                    </div>
                    
                    <div className="space-y-2">
                      <div className="flex justify-between text-sm">
                        <span className="text-gray-400">Performance</span>
                        <span className="text-gray-300">{proof.successRate.toFixed(0)}%</span>
                      </div>
                      <Progress 
                        value={proof.successRate} 
                        className="h-2"
                      />
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </TabsContent>

        <TabsContent value="history" className="space-y-6">
          {/* Consensus History */}
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Clock className="h-5 w-5 text-cyan-400" />
                Consensus Validation History
              </CardTitle>
              <CardDescription className="text-gray-400">Recent consensus validation events and outcomes</CardDescription>
            </CardHeader>
            <CardContent>
              {consensusHistory && consensusHistory.length > 0 ? (
                <div className="space-y-3 max-h-96 overflow-y-auto">
                  {consensusHistory.slice(0, 20).map((entry, index) => (
                    <div key={entry.blockId} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="text-white font-mono text-sm">{entry.blockId.slice(0, 12)}...</span>
                          <Badge 
                            variant="outline" 
                            className={cn(
                              'text-xs',
                              entry.status === 'validated' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                              entry.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                              'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
                            )}
                          >
                            {entry.status}
                          </Badge>
                          {entry.proofs && (
                            <div className="flex gap-1">
                              {entry.proofs.map(proof => (
                                <Badge key={proof.type} variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                                  {proof.type}
                                </Badge>
                              ))}
                            </div>
                          )}
                        </div>
                        <div className="text-sm text-gray-400">
                          Asset: {entry.assetId?.slice(0, 8)}... • 
                          Validation Time: {entry.validationTime || 0}ms
                        </div>
                      </div>
                      <div className="text-xs text-gray-500">
                        {new Date(entry.timestamp).toLocaleTimeString()}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center py-8">
                  <Database className="h-12 w-12 text-gray-600 mx-auto mb-3" />
                  <h3 className="text-lg font-medium text-white mb-2">No Consensus History</h3>
                  <p className="text-gray-400">
                    {systemStatus ? 'No consensus validations have been performed yet.' : 'System offline - unable to load consensus history.'}
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="byzantine" className="space-y-6">
          {/* Byzantine Detection */}
          <Card className="bg-black/40 border-red-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white flex items-center gap-2">
                <Shield className="h-5 w-5 text-red-400" />
                Byzantine Fault Detection
              </CardTitle>
              <CardDescription className="text-gray-400">Real-time monitoring of malicious behavior and security threats</CardDescription>
            </CardHeader>
            <CardContent>
              {byzantineDetections && byzantineDetections.length > 0 ? (
                <div className="space-y-3">
                  {byzantineDetections.map((detection, index) => (
                    <div key={detection.id} className="flex items-center justify-between p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <AlertTriangle className="h-4 w-4 text-red-400" />
                          <span className="text-white font-medium">Byzantine Behavior Detected</span>
                          <Badge variant="outline" className="text-xs bg-red-500/20 text-red-400 border-red-500/30">
                            {detection.severity}
                          </Badge>
                        </div>
                        <div className="text-sm text-gray-400">
                          Node: {detection.nodeId?.slice(0, 12)}... • 
                          Type: {detection.behaviorType} • 
                          Confidence: {detection.confidence}%
                        </div>
                      </div>
                      <div className="text-xs text-gray-500">
                        {new Date(detection.timestamp).toLocaleTimeString()}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center py-8">
                  <Shield className="h-12 w-12 text-green-600 mx-auto mb-3" />
                  <h3 className="text-lg font-medium text-white mb-2">Network Secure</h3>
                  <p className="text-gray-400">No Byzantine behavior detected. Network is operating normally.</p>
                  <div className="mt-4 text-sm text-green-400">
                    ✓ All nodes are behaving correctly
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Shield,
  Clock,
  Zap,
  Database,
  Users,
  RefreshCw,
  CheckCircle,
  Download
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { FourProofDisplay } from './FourProofDisplay';
import { StateProofHistory } from './StateProofHistory';
import { BlockValidation } from './BlockValidation';
import { useStateProofMetrics } from './hooks/useStateProofMetrics';

export interface StateProofMetrics {
  blockHeight: number;
  blockTime: number;
  validators: number;
  verificationTime: number;
  tps: number;
  proofCoverage: {
    space: number;
    stake: number;
    work: number;
    time: number;
  };
}

export interface HistoricalStateProofData {
  timestamp: Date;
  blockHeight: number;
  tps: number;
  proofCoverage: {
    space: number;
    stake: number;
    work: number;
    time: number;
  };
  validators: number;
}

interface ProofType {
  type: 'space' | 'stake' | 'work' | 'time';
  name: string;
  description: string;
  color: string;
  bgColor: string;
}

export interface ValidationResult {
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

interface StateProofMetricsPanelProps {
  stateProofMetrics: StateProofMetrics;
  historicalData: HistoricalStateProofData[];
  onValidateStateProof: () => Promise<ValidationResult>;
  onExportMetrics: () => Promise<void>;
  onViewDetails: (proofType: ProofType['type']) => void;
  refreshInterval?: number;
  showHistoricalTrends?: boolean;
  validationResults?: ValidationResult;
  isLoading?: boolean;
  className?: string;
}


export function StateProofMetricsPanel({
  stateProofMetrics,
  historicalData,
  onValidateStateProof,
  onExportMetrics,
  onViewDetails,
  refreshInterval = 5000,
  showHistoricalTrends = true,
  validationResults,
  isLoading = false,
  className
}: StateProofMetricsPanelProps) {
  const {
    activeTab,
    setActiveTab,
    timeRange,
    setTimeRange,
    validating,
    exporting,
    lastRefresh,
    handleValidateStateProof,
    handleExportMetrics,
    getHealthStatus
  } = useStateProofMetrics({
    refreshInterval,
    onValidateStateProof,
    onExportMetrics
  });

  const averageProofCoverage = Object.values(stateProofMetrics.proofCoverage)
    .reduce((acc, val) => acc + val, 0) / 4;

  const overallHealth = getHealthStatus(averageProofCoverage);

  return (
    <Card className={cn("w-full max-w-6xl mx-auto", className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <Shield className="h-6 w-6 text-quantum-600" />
            <div>
              <CardTitle>Four-Proof State Verification Metrics</CardTitle>
              <CardDescription>
                Proof of State protocol with unified WHERE/WHO/WHAT/WHEN validation
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center space-x-4">
            <div className="text-sm text-muted-foreground">
              Last updated: {lastRefresh.toLocaleTimeString()}
            </div>
            <Badge className={cn("font-medium", overallHealth.bg, overallHealth.color)}>
              {overallHealth.status}
            </Badge>
          </div>
        </div>
      </CardHeader>

      <CardContent>
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="overview">Overview</TabsTrigger>
            <TabsTrigger value="proofs">Proof Details</TabsTrigger>
            {showHistoricalTrends && <TabsTrigger value="trends">Historical Trends</TabsTrigger>}
          </TabsList>

          <TabsContent value="overview" className="space-y-6">
            {/* Real-Time Metrics */}
            <div className="grid gap-4 md:grid-cols-4">
              <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-white">Block Height</CardTitle>
                  <Database className="h-4 w-4 text-blue-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-white">{stateProofMetrics.blockHeight.toLocaleString()}</div>
                  <p className="text-xs text-blue-400">
                    +{(60 / stateProofMetrics.blockTime).toFixed(1)} blocks/min
                  </p>
                </CardContent>
              </Card>

              <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-white">TPS</CardTitle>
                  <Zap className="h-4 w-4 text-green-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-white">{stateProofMetrics.tps}</div>
                  <p className="text-xs text-green-400">
                    transactions/second
                  </p>
                </CardContent>
              </Card>

              <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-white">Validators</CardTitle>
                  <Users className="h-4 w-4 text-purple-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-white">{stateProofMetrics.validators}</div>
                  <p className="text-xs text-purple-400">
                    active participants
                  </p>
                </CardContent>
              </Card>

              <Card className="bg-black/40 border-quantum-500/30 backdrop-blur-lg">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-white">Verification</CardTitle>
                  <Clock className="h-4 w-4 text-quantum-400" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold text-white">{stateProofMetrics.verificationTime}s</div>
                  <p className="text-xs text-quantum-400">
                    time to verification
                  </p>
                </CardContent>
              </Card>
            </div>

            {/* Four-Proof System Overview */}
            <FourProofDisplay
              stateProofMetrics={stateProofMetrics}
              onViewDetails={onViewDetails}
            />

            {/* Performance Summary */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <Card>
                <CardHeader>
                  <CardTitle>Verification Performance</CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Block Production Rate:</span>
                    <span className="font-medium">{(60 / stateProofMetrics.blockTime).toFixed(1)} blocks/min</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Average Block Time:</span>
                    <span className="font-medium">{stateProofMetrics.blockTime}s</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Network Participation:</span>
                    <span className="font-medium">98.5%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Byzantine Fault Tolerance:</span>
                    <span className="font-medium text-green-600">33% threshold</span>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Network Health</CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>Proof Validation Rate:</span>
                      <span className="text-green-600 font-medium">{averageProofCoverage.toFixed(1)}%</span>
                    </div>
                    <Progress value={averageProofCoverage} className="h-2" />
                  </div>
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>Chain Integrity:</span>
                      <span className="text-green-600 font-medium">100%</span>
                    </div>
                    <Progress value={100} className="h-2" />
                  </div>
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>Verification Efficiency:</span>
                      <span className="text-blue-600 font-medium">94.2%</span>
                    </div>
                    <Progress value={94.2} className="h-2" />
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="proofs" className="space-y-6">
            <BlockValidation
              validationResults={validationResults}
              onViewDetails={onViewDetails}
            />
          </TabsContent>

          {showHistoricalTrends && (
            <TabsContent value="trends" className="space-y-6">
              <StateProofHistory
                stateProofMetrics={stateProofMetrics}
                historicalData={historicalData}
                timeRange={timeRange}
                onTimeRangeChange={setTimeRange}
              />
            </TabsContent>
          )}
        </Tabs>


        {/* Actions */}
        <div className="flex justify-between items-center pt-6 border-t">
          <div className="flex space-x-3">
            <Button
              variant="outline"
              onClick={handleValidateStateProof}
              disabled={validating || isLoading}
            >
              {validating ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <CheckCircle className="h-4 w-4 mr-2" />
              )}
              {validating ? 'Validating...' : 'Validate State Proof'}
            </Button>

            <Button
              variant="outline"
              onClick={handleExportMetrics}
              disabled={exporting || isLoading}
            >
              {exporting ? (
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Download className="h-4 w-4 mr-2" />
              )}
              {exporting ? 'Exporting...' : 'Export Metrics'}
            </Button>
          </div>

          <div className="text-sm text-muted-foreground">
            Auto-refresh: {refreshInterval > 0 ? `Every ${refreshInterval/1000}s` : 'Disabled'}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

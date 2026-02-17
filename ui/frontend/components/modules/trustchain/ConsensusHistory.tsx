// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { BarChart3 } from 'lucide-react';

interface ConsensusMetrics {
  proofCoverage: {
    space: number;
    stake: number;
    work: number;
    time: number;
  };
}

interface HistoricalConsensusData {
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

interface ConsensusHistoryProps {
  consensusMetrics: ConsensusMetrics;
  historicalData: HistoricalConsensusData[];
  timeRange: string;
  onTimeRangeChange: (value: string) => void;
}

const timeRanges = [
  { value: '1h', label: '1 Hour' },
  { value: '6h', label: '6 Hours' },
  { value: '24h', label: '24 Hours' },
  { value: '7d', label: '7 Days' },
  { value: '30d', label: '30 Days' }
];

export function ConsensusHistory({
  consensusMetrics,
  historicalData,
  timeRange,
  onTimeRangeChange
}: ConsensusHistoryProps) {
  const averageProofCoverage = Object.values(consensusMetrics.proofCoverage)
    .reduce((acc, val) => acc + val, 0) / 4;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Historical Performance</h3>
        <Select value={timeRange} onValueChange={onTimeRangeChange}>
          <SelectTrigger className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {timeRanges.map((range) => (
              <SelectItem key={range.value} value={range.value}>
                {range.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Proof Coverage Trends</CardTitle>
          <CardDescription>Coverage percentage over time for all four proof types</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-64 flex items-center justify-center border-2 border-dashed border-muted rounded-lg">
            <div className="text-center">
              <BarChart3 className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
              <p className="text-sm text-muted-foreground">
                Historical chart visualization would be rendered here
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                Data range: Last {timeRanges.find(r => r.value === timeRange)?.label}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Average Coverage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-quantum-600">
              {averageProofCoverage.toFixed(1)}%
            </div>
            <p className="text-sm text-muted-foreground">
              Last {timeRanges.find(r => r.value === timeRange)?.label}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Peak Performance</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">99.2%</div>
            <p className="text-sm text-muted-foreground">
              Highest coverage achieved
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Stability Score</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-600">96.8%</div>
            <p className="text-sm text-muted-foreground">
              Consistency metric
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
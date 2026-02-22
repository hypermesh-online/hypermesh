// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { useState, useEffect, useRef } from 'react';

interface ConsensusMetrics {
  blockHeight: number;
  blockTime: number;
  validators: number;
  finalityTime: number;
  tps: number;
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
    consensusParticipation: number;
  };
  recommendations: string[];
}

interface UseConsensusMetricsProps {
  refreshInterval?: number;
  onValidateConsensus: () => Promise<ValidationResult>;
  onExportMetrics: () => Promise<void>;
}

export function useConsensusMetrics({
  refreshInterval = 5000,
  onValidateConsensus,
  onExportMetrics
}: UseConsensusMetricsProps) {
  const [activeTab, setActiveTab] = useState('overview');
  const [timeRange, setTimeRange] = useState('24h');
  const [validating, setValidating] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (refreshInterval > 0) {
      intervalRef.current = setInterval(() => {
        setLastRefresh(new Date());
      }, refreshInterval);
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [refreshInterval]);

  const handleValidateConsensus = async () => {
    setValidating(true);
    try {
      return await onValidateConsensus();
    } finally {
      setValidating(false);
    }
  };

  const handleExportMetrics = async () => {
    setExporting(true);
    try {
      await onExportMetrics();
    } finally {
      setExporting(false);
    }
  };

  const getHealthStatus = (coverage: number) => {
    if (coverage >= 95) return { status: 'Excellent', color: 'text-green-600', bg: 'bg-green-100' };
    if (coverage >= 85) return { status: 'Good', color: 'text-blue-600', bg: 'bg-blue-100' };
    if (coverage >= 70) return { status: 'Warning', color: 'text-yellow-600', bg: 'bg-yellow-100' };
    return { status: 'Critical', color: 'text-red-600', bg: 'bg-red-100' };
  };

  return {
    activeTab,
    setActiveTab,
    timeRange,
    setTimeRange,
    validating,
    exporting,
    lastRefresh,
    handleValidateConsensus,
    handleExportMetrics,
    getHealthStatus
  };
}
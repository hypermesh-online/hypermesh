// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export interface EcosystemMetrics {
  totalAssets: number;
  activeCertificates: number;
  networkThroughput: number;
  consensusBlocks: number;
  quantumConnections: number;
  economicRewards: number;
}

export interface SystemStatus {
  name: string;
  status: 'online' | 'warning' | 'offline' | 'maintenance';
  uptime: number;
  lastChecked: string;
  metrics: Record<string, string>;
  description?: string;
}

export interface MetricTrend {
  value: number;
  change: number;
  trend: 'up' | 'down' | 'stable';
  period: string;
}

export interface EcosystemMetricsDashboardProps {
  metrics?: EcosystemMetrics;
  systemStatuses?: SystemStatus[];
  onRefresh?: () => void;
  autoRefresh?: boolean;
  refreshInterval?: number;
  loading?: boolean;
  className?: string;
}

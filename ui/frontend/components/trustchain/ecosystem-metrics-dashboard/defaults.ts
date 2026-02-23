// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { EcosystemMetrics, SystemStatus, MetricTrend } from './types';

export const defaultMetrics: EcosystemMetrics = {
  totalAssets: 1247,
  activeCertificates: 892,
  networkThroughput: 2.95,
  consensusBlocks: 15234,
  quantumConnections: 445,
  economicRewards: 12847.32
};

export const defaultSystemStatuses: SystemStatus[] = [
  {
    name: 'TrustChain CA',
    status: 'online',
    uptime: 2592000000,
    lastChecked: new Date().toISOString(),
    metrics: { 'Certificates Issued': '892', 'Root CAs': '3', 'Revoked Certs': '12' },
    description: 'Certificate Authority and trust management system'
  },
  {
    name: 'STOQ Protocol',
    status: 'warning',
    uptime: 2505600000,
    lastChecked: new Date().toISOString(),
    metrics: { 'Current Throughput': '2.95 Gbps', 'Active Connections': '445', 'Quantum Safe': '100%' },
    description: 'High-performance transport protocol with quantum security'
  },
  {
    name: 'HyperMesh Network',
    status: 'online',
    uptime: 2419200000,
    lastChecked: new Date().toISOString(),
    metrics: { 'Total Assets': '1,247', 'Active Nodes': '156', 'Asset Utilization': '67%' },
    description: 'Distributed asset sharing and resource coordination'
  },
  {
    name: 'Caesar Economics',
    status: 'online',
    uptime: 2332800000,
    lastChecked: new Date().toISOString(),
    metrics: { 'Total Rewards': '12,847.32 CAESAR', 'Staking Rate': '34%', 'Network Value': '$2.4M' },
    description: 'Economic incentive and reward distribution system'
  },
  {
    name: 'Four-Proof Consensus',
    status: 'online',
    uptime: 2246400000,
    lastChecked: new Date().toISOString(),
    metrics: { 'Block Height': '15,234', 'Validators': '67', 'Finality Time': '2.3s' },
    description: 'Proof of State validation with PoSp+PoSt+PoWk+PoTm authentication'
  }
];

export const metricTrends: Record<string, MetricTrend> = {
  totalAssets: { value: 1247, change: 2.4, trend: 'up', period: 'from last week' },
  activeCertificates: { value: 892, change: 1.2, trend: 'up', period: 'from last week' },
  networkThroughput: { value: 2.95, change: -0.3, trend: 'down', period: 'from target' },
  economicRewards: { value: 12847.32, change: 12.8, trend: 'up', period: 'this month' }
};

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { useSystemStatus } from '@/lib/api';
import { EcosystemMetricsDashboard } from './EcosystemMetricsDashboard';
import { NetworkManagement } from './NetworkManagement';
import { SecuritySettings } from './SecuritySettings';
import { ConsensusMetricsPanel } from './ConsensusMetricsPanel';
import { SecurityMonitoringDashboard } from '../security/SecurityMonitoringDashboard';
import { TrustChainSettings } from './TrustChainSettings';
import { cn } from '@/lib/utils';

const subNavigation = [
  { name: 'Overview', href: '/trustchain' },
  { name: 'Networks', href: '/trustchain/networks' },
  { name: 'Consensus', href: '/trustchain/consensus' },
  { name: 'Security', href: '/trustchain/security' },
  { name: 'Settings', href: '/trustchain/settings' },
];

interface NetworkConnection {
  id: string;
  name: string;
  type: 'Public' | 'P2P' | 'Federated';
  status: 'Connected' | 'Connecting' | 'Disconnected' | 'Error';
  trustScore: number;
  peers: number;
  consensus: string;
  description: string;
}

const networkConnections: NetworkConnection[] = [
  {
    id: 'public-main',
    name: 'HyperMesh Public Network',
    type: 'Public',
    status: 'Connected',
    trustScore: 94.2,
    peers: 15420,
    consensus: 'Proof of Stake',
    description: 'Global public network with open access and democratic consensus'
  },
  {
    id: 'p2p-local',
    name: 'Local P2P Cluster',
    type: 'P2P',
    status: 'Connected',
    trustScore: 98.7,
    peers: 12,
    consensus: 'Byzantine Fault Tolerance',
    description: 'Direct peer-to-peer connections with trusted nodes'
  },
  {
    id: 'fed-enterprise',
    name: 'Enterprise Federation',
    type: 'Federated',
    status: 'Connecting',
    trustScore: 87.5,
    peers: 234,
    consensus: 'Federated Byzantine Agreement',
    description: 'Private federated network for enterprise resource sharing'
  }
];

function TrustChainOverview() {
  const { systemStatus } = useSystemStatus(true);
  
  const systemStatuses = [
    {
      name: 'TrustChain CA',
      status: 'online' as const,
      uptime: 2592000000,
      lastChecked: new Date().toISOString(),
      metrics: {
        'Certificates Issued': '892',
        'Root CAs': '3',
        'Revoked Certs': '12'
      },
      description: 'Certificate Authority and trust management system'
    },
    ...networkConnections.map(net => ({
      name: net.name,
      status: (net.status === 'Connected' ? 'online' : 
               net.status === 'Connecting' ? 'warning' : 'offline') as const,
      uptime: Math.random() * 2592000000,
      lastChecked: new Date().toISOString(),
      metrics: {
        'Trust Score': `${net.trustScore}%`,
        'Peers': net.peers.toString(),
        'Consensus': net.consensus
      },
      description: net.description
    }))
  ];

  return (
    <EcosystemMetricsDashboard 
      systemStatuses={systemStatuses}
      onRefresh={() => {
        console.log('Refreshing TrustChain ecosystem data...');
      }}
    />
  );
}

function ConsensusSettings() {
  return (
    <ConsensusMetricsPanel 
      onRefresh={() => {
        console.log('Refreshing consensus metrics...');
      }}
      onValidate={() => {
        console.log('Validating consensus...');
      }}
    />
  );
}

function SubNavigation() {
  const location = useLocation();

  return (
    <div className="border-b border-green-500/20 mb-6">
      <nav className="-mb-px flex space-x-8">
        {subNavigation.map((item) => {
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'py-2 px-1 border-b-2 font-medium text-sm transition-colors',
                isActive
                  ? 'border-green-400 text-green-400'
                  : 'border-transparent text-gray-400 hover:text-white hover:border-green-500/50'
              )}
            >
              {item.name}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}

export function TrustChainRouting() {
  return (
    <div className="space-y-6">
      <SubNavigation />
      
      <Routes>
        <Route path="/" element={<TrustChainOverview />} />
        <Route path="/networks" element={<NetworkManagement />} />
        <Route path="/consensus" element={<ConsensusSettings />} />
        <Route path="/security" element={<SecurityMonitoringDashboard />} />
        <Route path="/settings" element={<TrustChainSettings />} />
      </Routes>
    </div>
  );
}
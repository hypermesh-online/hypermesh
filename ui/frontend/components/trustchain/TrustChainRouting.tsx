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
    {
      name: 'STOQ Transport',
      status: (systemStatus ? 'online' : 'offline') as const,
      uptime: systemStatus?.performance?.uptime ? systemStatus.performance.uptime * 86400000 : 0,
      lastChecked: new Date().toISOString(),
      metrics: {
        'Validation': 'Verified',
        'Protocol': 'QUIC/IPv6',
        'Encryption': 'FALCON-1024'
      },
      description: 'STOQ protocol transport layer with PoS validation'
    },
    {
      name: 'eBPF Security',
      status: 'online' as const,
      uptime: 2592000000,
      lastChecked: new Date().toISOString(),
      metrics: {
        'Validation': 'Active',
        'Mode': 'XDP',
        'Policies': 'Enforced'
      },
      description: 'eBPF packet processing and security enforcement'
    }
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

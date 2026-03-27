// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { Network } from 'lucide-react';
import { HyperMeshOverview } from './hypermesh/HyperMeshOverview';
import { ResourceConfiguration } from './hypermesh/ResourceConfiguration';
import { SharingManagement } from './hypermesh/SharingManagement';
import { StateProofDashboard } from '../proof-of-state/StateProofDashboard';
import { AdvancedAssetManagement } from '../assets/AdvancedAssetManagement';
import { BlockchainExplorer } from './hypermesh/BlockchainExplorer';
import { DnsManagement } from './hypermesh/DnsManagement';
import { TopologyView } from './hypermesh/TopologyView';
import { DomainManagement } from './hypermesh/DomainManagement';
import { AssetManagement } from './hypermesh/AssetManagement';
import { StorePipeline } from './hypermesh/StorePipeline';
import { NodeSettings } from './hypermesh/NodeSettings';

const subNavigation = [
  { name: 'Overview', href: '/hypermesh' },
  { name: 'Blockchain', href: '/hypermesh/blockchain' },
  { name: 'Assets', href: '/hypermesh/assets' },
  { name: 'DNS', href: '/hypermesh/dns' },
  { name: 'Domains', href: '/hypermesh/domains' },
  { name: 'Topology', href: '/hypermesh/topology' },
  { name: 'Sharing', href: '/hypermesh/sharing' },
  { name: 'Pipeline', href: '/hypermesh/pipeline' },
  { name: 'Resources', href: '/hypermesh/resources' },
  { name: 'Proof of State', href: '/hypermesh/proof-of-state' },
  { name: 'Settings', href: '/hypermesh/settings' },
  { name: 'Advanced', href: '/hypermesh/advanced' },
];

function SubNavigation() {
  const location = useLocation();

  return (
    <div className="border-b border-cyan-500/20 mb-6 overflow-x-auto">
      <nav className="-mb-px flex space-x-6">
        {subNavigation.map((item) => {
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'py-2 px-1 border-b-2 font-medium text-sm transition-colors whitespace-nowrap',
                isActive
                  ? 'border-cyan-400 text-cyan-400'
                  : 'border-transparent text-gray-400 hover:text-white hover:border-cyan-500/50',
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

export function HyperMeshModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600">
            <Network className="h-8 w-8 text-black" />
          </div>
          HyperMesh
        </h1>
        <p className="text-gray-400 mt-2">
          Federated resource sharing with Private, P2P, and Public network scopes
        </p>
      </div>

      <SubNavigation />

      <Routes>
        <Route path="/" element={<HyperMeshOverview />} />
        <Route path="/blockchain" element={<BlockchainExplorer />} />
        <Route path="/assets" element={<AssetManagement />} />
        <Route path="/dns" element={<DnsManagement />} />
        <Route path="/domains" element={<DomainManagement />} />
        <Route path="/topology" element={<TopologyView />} />
        <Route path="/sharing" element={<SharingManagement />} />
        <Route path="/pipeline" element={<StorePipeline />} />
        <Route path="/resources" element={<ResourceConfiguration />} />
        <Route path="/proof-of-state" element={<StateProofDashboard />} />
        <Route path="/settings" element={<NodeSettings />} />
        <Route path="/advanced" element={<AdvancedAssetManagement />} />
      </Routes>
    </div>
  );
}

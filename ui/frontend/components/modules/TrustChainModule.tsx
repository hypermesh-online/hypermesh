// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { Shield } from 'lucide-react';
import { cn } from '@/lib/utils';
import { OverviewPanel } from '../trustchain/panels/OverviewPanel';
import { CertificatesPanel } from '../trustchain/panels/CertificatesPanel';
import { CertificateExtensionsPanel } from '../trustchain/panels/CertificateExtensionsPanel';
import { IdentityPanel } from '../trustchain/panels/IdentityPanel';
import { FederationPanel } from '../trustchain/panels/FederationPanel';
import { StateProofPanel } from '../trustchain/panels/StateProofPanel';
import { SettingsPanel } from '../trustchain/panels/SettingsPanel';
import { SecuritySettings } from '../trustchain/SecuritySettings';
import { NetworkManagement } from '../trustchain/NetworkManagement';

const subNavigation = [
  { name: 'Overview', href: '/trustchain' },
  { name: 'Certificates', href: '/trustchain/certificates' },
  { name: 'Extensions', href: '/trustchain/extensions' },
  { name: 'Identity', href: '/trustchain/identity' },
  { name: 'Federation', href: '/trustchain/federation' },
  { name: 'Network', href: '/trustchain/network' },
  { name: 'Security', href: '/trustchain/security' },
  { name: 'Metrics', href: '/trustchain/metrics' },
  { name: 'Settings', href: '/trustchain/settings' },
];

function SubNavigation() {
  const location = useLocation();

  return (
    <div className="border-b border-green-500/20 mb-6 overflow-x-auto">
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
                  ? 'border-green-400 text-green-400'
                  : 'border-transparent text-gray-400 hover:text-white hover:border-green-500/50',
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

export function TrustChainModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-green-400 to-emerald-600">
            <Shield className="h-8 w-8 text-black" />
          </div>
          TrustChain
        </h1>
        <p className="text-gray-400 mt-2">
          Certificate authority, post-quantum identity, and Proof of State verification
        </p>
      </div>

      <SubNavigation />

      <Routes>
        <Route path="/" element={<OverviewPanel />} />
        <Route path="/certificates" element={<CertificatesPanel />} />
        <Route path="/extensions" element={<CertificateExtensionsPanel />} />
        <Route path="/identity" element={<IdentityPanel />} />
        <Route path="/federation" element={<FederationPanel />} />
        <Route path="/network" element={<NetworkManagement />} />
        <Route path="/security" element={<SecuritySettings />} />
        <Route path="/metrics" element={<StateProofPanel />} />
        <Route path="/settings" element={<SettingsPanel />} />
      </Routes>
    </div>
  );
}

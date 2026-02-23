// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { CaesarOverview } from './caesar/CaesarOverview';

const subNavigation = [
  { name: 'Overview', href: '/caesar' },
  { name: 'Wallet', href: '/caesar/wallet' },
  { name: 'Rewards', href: '/caesar/rewards' },
  { name: 'NGauge', href: '/caesar/ngauge' },
];

function CaesarWallet() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Wallet Management</h2>
      <p className="text-gray-400">Full wallet functionality with real Caesar backend integration.</p>
      {/* Wallet implementation will use real Caesar API */}
    </div>
  );
}

function CaesarRewards() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>
      <p className="text-gray-400">Track and claim your rewards from the Caesar economic system.</p>
      {/* Rewards implementation will use real Caesar API */}
    </div>
  );
}

function CaesarNGauge() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">NGauge Integration</h2>
      <p className="text-gray-400">Advanced engagement and advertising integration.</p>
      {/* NGauge implementation will use real Caesar API */}
    </div>
  );
}

export default function CaesarModule() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      {/* Sub-navigation */}
      <nav className="flex space-x-4 border-b border-gray-800 pb-4">
        {subNavigation.map((item) => (
          <Link
            key={item.name}
            to={item.href}
            className={cn(
              'px-3 py-2 text-sm font-medium rounded-lg transition-colors',
              location.pathname === item.href
                ? 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30'
                : 'text-gray-400 hover:text-white hover:bg-gray-800/50'
            )}
          >
            {item.name}
          </Link>
        ))}
      </nav>

      {/* Routes */}
      <Routes>
        <Route path="/" element={<CaesarOverview />} />
        <Route path="/wallet" element={<CaesarWallet />} />
        <Route path="/rewards" element={<CaesarRewards />} />
        <Route path="/ngauge" element={<CaesarNGauge />} />
      </Routes>
    </div>
  );
}

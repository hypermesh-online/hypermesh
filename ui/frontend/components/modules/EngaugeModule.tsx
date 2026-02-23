// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { BarChart2 } from 'lucide-react';

const EngaugeOverview = React.lazy(() => import('./engauge/EngaugeOverview'));
const EngaugeAnalytics = React.lazy(() => import('./engauge/EngaugeAnalytics'));
const EngaugeMarketplace = React.lazy(() => import('./engauge/EngaugeMarketplace'));
const EngaugeRouting = React.lazy(() => import('./engauge/EngaugeRouting'));

const subNavigation = [
  { name: 'Overview', href: '/engauge' },
  { name: 'Analytics', href: '/engauge/analytics' },
  { name: 'Marketplace', href: '/engauge/marketplace' },
  { name: 'Routing', href: '/engauge/routing' },
];

export default function EngaugeModule() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-orange-400 to-red-600">
            <BarChart2 className="h-8 w-8 text-black" />
          </div>
          Engauge
        </h1>
        <p className="text-gray-400 mt-2">
          Analytics, capacity metrics & resource marketplace
        </p>
      </div>

      <nav className="flex space-x-4 border-b border-gray-800 pb-4">
        {subNavigation.map((item) => (
          <Link
            key={item.name}
            to={item.href}
            className={cn(
              'px-3 py-2 text-sm font-medium rounded-lg transition-colors',
              location.pathname === item.href
                ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30'
                : 'text-gray-400 hover:text-white hover:bg-gray-800/50'
            )}
          >
            {item.name}
          </Link>
        ))}
      </nav>

      <React.Suspense fallback={<div className="flex items-center justify-center h-32"><div className="animate-spin rounded-full h-6 w-6 border-b-2 border-orange-400"></div></div>}>
        <Routes>
          <Route path="/" element={<EngaugeOverview />} />
          <Route path="/analytics" element={<EngaugeAnalytics />} />
          <Route path="/marketplace" element={<EngaugeMarketplace />} />
          <Route path="/routing" element={<EngaugeRouting />} />
        </Routes>
      </React.Suspense>
    </div>
  );
}

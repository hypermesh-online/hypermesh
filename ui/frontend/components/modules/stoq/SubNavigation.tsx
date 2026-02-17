// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';

const subNavigation = [
  { name: 'Overview', href: '/stoq' },
  { name: 'Protocol', href: '/stoq/protocol' },
  { name: 'P2P Tunnels', href: '/stoq/tunnels' },
  { name: 'Performance', href: '/stoq/performance' },
];

export function SubNavigation() {
  const location = useLocation();

  return (
    <div className="border-b border-cyan-500/20 mb-6">
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
                  ? 'border-cyan-400 text-cyan-400'
                  : 'border-transparent text-gray-400 hover:text-white hover:border-cyan-500/50'
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
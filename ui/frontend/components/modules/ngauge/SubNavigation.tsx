// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';

const subNavigation = [
  { name: 'Overview', href: '/ngauge' },
  { name: 'Onboarding', href: '/ngauge/onboarding' },
  { name: 'Ad Network', href: '/ngauge/ads' },
  { name: 'Analytics', href: '/ngauge/analytics' },
];

export function SubNavigation() {
  const location = useLocation();

  return (
    <div className="border-b border-border mb-6">
      <nav className="-mb-px flex space-x-8">
        {subNavigation.map((item) => {
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'py-2 px-1 border-b-2 font-medium text-sm',
                isActive
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:border-muted-foreground'
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
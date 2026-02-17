// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';

interface TabNavigationProps {
  items: Array<{
    name: string;
    href: string;
  }>;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function TabNavigation({
  items,
  theme = 'cyan',
  className
}: TabNavigationProps) {
  const location = useLocation();

  const getThemeColors = () => {
    const themes = {
      cyan: {
        active: 'border-cyan-400 text-cyan-400',
        inactive: 'border-transparent text-gray-400 hover:text-white hover:border-cyan-500/50'
      },
      green: {
        active: 'border-green-400 text-green-400',
        inactive: 'border-transparent text-gray-400 hover:text-white hover:border-green-500/50'
      },
      purple: {
        active: 'border-purple-400 text-purple-400',
        inactive: 'border-transparent text-gray-400 hover:text-white hover:border-purple-500/50'
      },
      red: {
        active: 'border-red-400 text-red-400',
        inactive: 'border-transparent text-gray-400 hover:text-white hover:border-red-500/50'
      },
      yellow: {
        active: 'border-yellow-400 text-yellow-400',
        inactive: 'border-transparent text-gray-400 hover:text-white hover:border-yellow-500/50'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();

  return (
    <div className={cn(`border-b border-${theme}-500/20 mb-6`, className)}>
      <nav className="-mb-px flex space-x-8">
        {items.map((item) => {
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'py-2 px-1 border-b-2 font-medium text-sm transition-colors',
                isActive ? colors.active : colors.inactive
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

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { ChevronRight, Home } from 'lucide-react';
import { cn } from '@/lib/utils';

interface BreadcrumbItem {
  label: string;
  href: string;
  icon?: React.ComponentType<{ className?: string }>;
}

interface BreadcrumbsProps {
  items?: BreadcrumbItem[];
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function Breadcrumbs({
  items,
  theme = 'cyan',
  className
}: BreadcrumbsProps) {
  const location = useLocation();

  const getThemeColors = () => {
    const themes = {
      cyan: {
        active: 'text-cyan-400',
        inactive: 'text-gray-400 hover:text-cyan-300',
        separator: 'text-cyan-500/50'
      },
      green: {
        active: 'text-green-400',
        inactive: 'text-gray-400 hover:text-green-300',
        separator: 'text-green-500/50'
      },
      purple: {
        active: 'text-purple-400',
        inactive: 'text-gray-400 hover:text-purple-300',
        separator: 'text-purple-500/50'
      },
      red: {
        active: 'text-red-400',
        inactive: 'text-gray-400 hover:text-red-300',
        separator: 'text-red-500/50'
      },
      yellow: {
        active: 'text-yellow-400',
        inactive: 'text-gray-400 hover:text-yellow-300',
        separator: 'text-yellow-500/50'
      }
    };
    return themes[theme];
  };

  const colors = getThemeColors();

  // Auto-generate breadcrumbs from current path if not provided
  const generateBreadcrumbs = (): BreadcrumbItem[] => {
    const pathSegments = location.pathname.split('/').filter(Boolean);
    const breadcrumbs: BreadcrumbItem[] = [
      { label: 'Dashboard', href: '/', icon: Home }
    ];

    let currentPath = '';
    pathSegments.forEach((segment, index) => {
      currentPath += `/${segment}`;
      
      // Capitalize and format segment names
      const label = segment.charAt(0).toUpperCase() + segment.slice(1);
      
      // Map specific segments to better names
      const segmentLabels: Record<string, string> = {
        'trustchain': 'TrustChain',
        'hypermesh': 'HyperMesh',
        'caesar': 'Caesar',
        'catalog': 'Catalog',
        'ngauge': 'NGauge',
        'identity': 'Identity',
        'networks': 'Networks',
        'trust': 'Trust Web',
        'blockchain': 'Blockchain',
        'network': 'P2P Network',
        'insights': 'System Insights',
        'trading': 'Trading',
        'governance': 'Governance',
        'analytics': 'Analytics',
        'markets': 'Markets',
        'creation': 'Creation',
        'management': 'Management',
        'onboarding': 'Onboarding',
        'ads': 'Ad Network'
      };

      breadcrumbs.push({
        label: segmentLabels[segment] || label,
        href: currentPath
      });
    });

    return breadcrumbs;
  };

  const breadcrumbItems = items || generateBreadcrumbs();

  if (breadcrumbItems.length <= 1) {
    return null;
  }

  return (
    <nav className={cn('flex items-center space-x-1 text-sm', className)}>
      {breadcrumbItems.map((item, index) => {
        const isLast = index === breadcrumbItems.length - 1;
        const Icon = item.icon;

        return (
          <React.Fragment key={item.href}>
            {index > 0 && (
              <ChevronRight className={cn('h-4 w-4', colors.separator)} />
            )}
            <div className="flex items-center gap-1">
              {Icon && <Icon className="h-4 w-4" />}
              {isLast ? (
                <span className={cn('font-medium', colors.active)}>
                  {item.label}
                </span>
              ) : (
                <Link
                  to={item.href}
                  className={cn(
                    'transition-colors duration-200',
                    colors.inactive
                  )}
                >
                  {item.label}
                </Link>
              )}
            </div>
          </React.Fragment>
        );
      })}
    </nav>
  );
}

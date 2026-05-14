// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { Package } from 'lucide-react';
import { cn } from '@/lib/utils';
import { type PrivacyLevel } from '@/lib/types';

// Import modular components
import { CatalogBrowse } from '../catalog/CatalogBrowse';
import { CatalogInstalled } from '../catalog/CatalogInstalled';
import { CatalogCreate } from '../catalog/CatalogCreate';
import { CatalogDependencies } from '../catalog/CatalogDependencies';

const subNavigation = [
  { name: 'Browse', href: '/catalog' },
  { name: 'Installed', href: '/catalog/installed' },
  { name: 'Dependencies', href: '/catalog/dependencies' },
  { name: 'Create', href: '/catalog/create' },
];

function SubNavigation() {
  const location = useLocation();

  return (
    <nav className="flex space-x-1 bg-black/40 p-1 rounded-lg border border-purple-500/30 backdrop-blur-lg">
      {subNavigation.map((item) => (
        <Link
          key={item.name}
          to={item.href}
          className={cn(
            'flex-1 text-center px-4 py-2 text-sm font-medium rounded transition-all',
            location.pathname === item.href
              ? 'bg-purple-600 text-white shadow-lg'
              : 'text-gray-400 hover:text-white hover:bg-purple-500/20'
          )}
        >
          {item.name}
        </Link>
      ))}
    </nav>
  );
}

export function CatalogModule() {
  const [selectedPrivacyLevel, setSelectedPrivacyLevel] = React.useState<PrivacyLevel>('federated');

  return (
    <div className="space-y-6">
      {/* Module Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-purple-400 to-pink-600">
            <Package className="h-8 w-8 text-black" />
          </div>
          Catalog
        </h1>
        <p className="text-gray-400 mt-2">
          Asset package manager with automatic dependency resolution and multi-adapter support
        </p>
      </div>

      {/* Sub Navigation */}
      <SubNavigation />

      {/* Routes */}
      <Routes>
        <Route 
          path="/" 
          element={
            <CatalogBrowse />
          } 
        />
        <Route path="/installed" element={<CatalogInstalled />} />
        <Route path="/dependencies" element={<CatalogDependencies />} />
        <Route path="/create" element={<CatalogCreate />} />
      </Routes>
    </div>
  );
}
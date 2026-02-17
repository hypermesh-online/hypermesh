// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { 
  Home,
  Network,
  Package,
  Shield,
  Coins,
  Activity,
  Zap
} from 'lucide-react';

const navigation = [
  { 
    name: 'Dashboard', 
    href: '/', 
    icon: Home, 
    description: 'User experience, journey & system overview' 
  },
  { 
    name: 'Monitor', 
    href: '/monitor', 
    icon: Activity, 
    description: 'Real-time system monitoring & performance' 
  },
  { 
    name: 'STOQ Demo', 
    href: '/stoq-demo', 
    icon: Zap, 
    description: 'Internet 2.0 native protocol demonstration' 
  },
  { 
    name: 'HyperMesh', 
    href: '/hypermesh', 
    icon: Network, 
    description: 'System resources & federated sharing' 
  },
  { 
    name: 'Catalog', 
    href: '/catalog', 
    icon: Package, 
    description: 'Asset package manager & library' 
  },
  { 
    name: 'TrustChain', 
    href: '/trustchain', 
    icon: Shield, 
    description: 'Network connections & consensus' 
  },
  { 
    name: 'Caesar', 
    href: '/caesar', 
    icon: Coins, 
    description: 'Token integration & economics' 
  },
];

export function Sidebar() {
  const location = useLocation();

  return (
    <div className="w-72 bg-black/80 backdrop-blur-lg border-r border-cyan-500/20">
      <div className="p-6">
        <div className="flex items-center gap-3 mb-2">
          <div className="w-8 h-8 bg-gradient-to-r from-cyan-400 to-blue-500 rounded-lg flex items-center justify-center">
            <Network className="h-5 w-5 text-black" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-white">HyperMesh</h1>
            <p className="text-xs text-cyan-400">Federated Resource Platform</p>
          </div>
        </div>
        <div className="text-xs text-gray-400 font-mono">
          node-7f8a9b2c.hypermesh
        </div>
      </div>
      
      <nav className="px-4 space-y-2">
        {navigation.map((item) => {
          const isActive = location.pathname === item.href || 
            (item.href !== '/' && location.pathname.startsWith(item.href));
          
          return (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'flex flex-col gap-1 px-3 py-3 rounded-lg text-sm font-medium transition-all duration-200',
                isActive
                  ? 'bg-gradient-to-r from-cyan-500/20 to-blue-500/20 border border-cyan-500/50 text-cyan-300 shadow-lg shadow-cyan-500/20'
                  : 'text-gray-300 hover:text-white hover:bg-cyan-500/10 hover:border hover:border-cyan-500/30'
              )}
            >
              <div className="flex items-center gap-3">
                <item.icon className={cn("h-4 w-4", isActive ? "text-cyan-400" : "")} />
                <span>{item.name}</span>
                {isActive && <div className="ml-auto w-2 h-2 bg-cyan-400 rounded-full animate-pulse" />}
              </div>
              <p className={cn(
                "text-xs ml-7",
                isActive ? "text-cyan-300/80" : "text-gray-500"
              )}>
                {item.description}
              </p>
            </Link>
          );
        })}
      </nav>
      
      <div className="p-4 mt-6 border-t border-gray-700">
        <div className="text-xs text-gray-400 space-y-2">
          <div className="flex justify-between">
            <span>Network Status:</span>
            <span className="text-green-400">Connected</span>
          </div>
          <div className="flex justify-between">
            <span>Resources:</span>
            <div className="text-right">
              <div className="text-cyan-400">CPU: 4 cores</div>
              <div className="text-cyan-400">RAM: 16GB</div>
              <div className="text-cyan-400">Storage: 500GB</div>
            </div>
          </div>
          <div className="flex justify-between">
            <span>Sharing Mode:</span>
            <span className="text-purple-400">Federated</span>
          </div>
        </div>
      </div>
    </div>
  );
}

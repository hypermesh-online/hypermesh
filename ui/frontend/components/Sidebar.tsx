// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { useNodeStatus, useShareInbox, useMessageInbox } from '@/lib/hooks/useBlockMatrix';
import {
  Home,
  Network,
  Package,
  Shield,
  Coins,
  Activity,
  Inbox,
  MessageSquare,
  BarChart2,
  Zap,
} from 'lucide-react';

const navigation = [
  {
    name: 'Dashboard',
    href: '/',
    icon: Home,
    description: 'System overview and node status'
  },
  {
    name: 'Monitor',
    href: '/monitor',
    icon: Activity,
    description: 'Real-time system monitoring'
  },
  {
    name: 'HyperMesh',
    href: '/hypermesh',
    icon: Network,
    description: 'System resources and sharing'
  },
  {
    name: 'Catalog',
    href: '/catalog',
    icon: Package,
    description: 'Asset package manager'
  },
  {
    name: 'TrustChain',
    href: '/trustchain',
    icon: Shield,
    description: 'Network connections and Proof of State'
  },
  {
    name: 'Caesar',
    href: '/caesar',
    icon: Coins,
    description: 'Token integration and economics'
  },
  {
    name: 'Engauge',
    href: '/engauge',
    icon: BarChart2,
    description: 'Analytics and resource marketplace'
  },
  {
    name: 'STOQ',
    href: '/stoq',
    icon: Zap,
    description: 'QUIC transport protocol'
  },
  {
    name: 'Inbox',
    href: '/inbox',
    icon: Inbox,
    description: 'Received file sharing invites'
  },
  {
    name: 'Messages',
    href: '/messages',
    icon: MessageSquare,
    description: 'Private peer-to-peer messaging'
  },
];

export function Sidebar() {
  const location = useLocation();
  const { data: status } = useNodeStatus(10_000);
  const { data: inbox } = useShareInbox(15_000);
  const { data: msgInbox } = useMessageInbox(15_000);
  const inboxCount = inbox?.invites?.length ?? 0;
  const messageCount = msgInbox?.count ?? 0;

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
          {status ? status.node_id : 'not connected'}
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
                {item.name === 'Inbox' && inboxCount > 0 && (
                  <span className="ml-auto bg-cyan-500 text-black text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center">
                    {inboxCount > 9 ? '9+' : inboxCount}
                  </span>
                )}
                {item.name === 'Messages' && messageCount > 0 && (
                  <span className="ml-auto bg-cyan-500 text-black text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center">
                    {messageCount > 9 ? '9+' : messageCount}
                  </span>
                )}
                {isActive && item.name !== 'Inbox' && item.name !== 'Messages' && <div className="ml-auto w-2 h-2 bg-cyan-400 rounded-full animate-pulse" />}
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
            <span>Daemon:</span>
            <span className={status ? "text-green-400" : "text-red-400"}>
              {status ? 'Connected' : 'Offline'}
            </span>
          </div>
          {status && (
            <>
              <div className="flex justify-between">
                <span>Chain Height:</span>
                <span className="text-cyan-400">{status.chain_height}</span>
              </div>
              <div className="flex justify-between">
                <span>Peers:</span>
                <span className="text-cyan-400">{status.peers}</span>
              </div>
              <div className="flex justify-between">
                <span>Privacy:</span>
                <span className="text-purple-400">{status.privacy_mode}</span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

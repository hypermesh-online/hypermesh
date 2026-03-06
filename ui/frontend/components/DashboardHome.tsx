// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ModuleCard } from '@/components/ui/ModuleCard';
import { ActivityItem } from '@/components/ui/ActivityItem';
import { ModuleHeader } from '@/components/ui/ModuleHeader';
import { NavigationHints } from '@/components/ui/NavigationHints';
import { FlowIndicator } from '@/components/ui/FlowIndicator';
import { AccessibilityWrapper } from '@/components/ui/AccessibilityWrapper';
import { ScreenReaderOnly } from '@/components/ui/ScreenReaderOnly';
import { useNodeStatus, useBlockchainHeight, useNetworkPeers, useAssetList } from '@/lib/hooks/useBlockMatrix';
import { getCrateStatus } from '@/lib/data/crateStatus';
import {
  Network,
  Package,
  Shield,
  Coins,
  Activity,
  Settings,
  ArrowRight,
  Server,
  Zap,
  AlertTriangle
} from 'lucide-react';

/**
 * System overview using real BlockMatrix API data.
 * Calls /api/v1/status, /api/v1/blockchain/height, /api/v1/network/peers,
 * and /api/v1/asset/list from the running daemon on port 9293.
 */
function useSystemOverview() {
  const statusQuery = useNodeStatus(10_000);
  const heightQuery = useBlockchainHeight(10_000);
  const peersQuery = useNetworkPeers(15_000);
  const assetsQuery = useAssetList(15_000);

  const status = statusQuery.data;
  const isOnline = statusQuery.isSuccess && !!status;

  return {
    nodeId: status?.node_id ?? 'unknown',
    coordinate: status?.coordinate ?? { x: 0, y: 0, z: 0 },
    chainHeight: status?.chain_height ?? heightQuery.data?.height ?? 0,
    privacyMode: status?.privacy_mode ?? 'Unknown',
    peerCount: status?.peers ?? peersQuery.data?.length ?? 0,
    uptimeSecs: status?.uptime_secs ?? 0,
    assetCount: assetsQuery.data?.length ?? 0,
    isOnline,
    isLoading: statusQuery.isLoading,
    error: statusQuery.error,
  };
}

const quickActions = [
  {
    title: 'Configure Resources',
    description: 'Set up CPU, RAM, and storage for sharing',
    href: '/hypermesh/resources',
    icon: Settings,
    badge: 'Setup Required',
    priority: 'high' as const
  },
  {
    title: 'Browse Catalog',
    description: 'Install new assets and dependencies',
    href: '/catalog',
    icon: Package,
    badge: 'Recommended',
    priority: 'medium' as const
  },
  {
    title: 'Connect Networks',
    description: 'Join P2P or Federated networks',
    href: '/trustchain',
    icon: Shield,
    badge: 'Available',
    priority: 'medium' as const
  },
  {
    title: 'Enable Caesar',
    description: 'Activate token rewards and payments',
    href: '/caesar',
    icon: Coins,
    badge: 'Optional',
    priority: 'low' as const
  }
];

/** Generate activity items from real system state */
function useRecentActivity(isOnline: boolean, peerCount: number, chainHeight: number, assetCount: number) {
  const activity: Array<{ type: string; message: string; time: string }> = [];

  if (isOnline) {
    activity.push({ type: 'success', message: 'BlockMatrix daemon connected', time: 'Just now' });
  }

  if (chainHeight > 0) {
    activity.push({ type: 'info', message: `Blockchain at height ${chainHeight}`, time: 'Just now' });
  }

  if (peerCount > 0) {
    activity.push({ type: 'success', message: `${peerCount} peer(s) connected`, time: 'Recent' });
  }

  if (assetCount > 0) {
    activity.push({ type: 'info', message: `${assetCount} asset(s) registered`, time: 'Recent' });
  }

  if (activity.length === 0) {
    return [
      { type: 'warning', message: 'Backend not connected (localhost:9293)', time: 'Now' },
      { type: 'info', message: 'Start the daemon: hypermesh daemon start', time: '' },
    ];
  }

  return activity.slice(0, 4);
}

const userJourneySteps = [
  {
    id: 'resources',
    title: 'Setup Resources',
    description: 'Configure your system resources for sharing',
    href: '/hypermesh/resources',
    status: 'current' as const
  },
  {
    id: 'catalog',
    title: 'Install Assets',
    description: 'Browse and install required dependencies',
    href: '/catalog',
    status: 'upcoming' as const
  },
  {
    id: 'networks',
    title: 'Connect Networks',
    description: 'Join trusted networks and establish connections',
    href: '/trustchain',
    status: 'upcoming' as const
  },
  {
    id: 'economics',
    title: 'Enable Rewards',
    description: 'Activate Caesar token integration',
    href: '/caesar',
    status: 'upcoming' as const
  }
];

export function DashboardHome() {
  const overview = useSystemOverview();
  const recentActivity = useRecentActivity(
    overview.isOnline,
    overview.peerCount,
    overview.chainHeight,
    overview.assetCount,
  );

  /** Format uptime seconds to human-readable */
  const formatUptime = (secs: number): string => {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return `${h}h ${m}m`;
  };

  return (
    <AccessibilityWrapper
      role="main"
      ariaLabel="HyperMesh Dashboard Overview"
      className="space-y-8"
    >
      <ScreenReaderOnly>
        <h1>HyperMesh Dashboard</h1>
        <p>
          Welcome to HyperMesh, your federated resource sharing platform.
          Navigate through system overview, resources, assets, and network connections.
        </p>
      </ScreenReaderOnly>

      <ModuleHeader
        title="HyperMesh Dashboard"
        description="Federated resource sharing platform with decentralized asset management and network connectivity."
        gradient="from-cyan-400 via-blue-500 to-purple-600"
        centered
      />

      {/* Connection Banner */}
      {!overview.isOnline && !overview.isLoading && (
        <div className="bg-orange-900/30 border border-orange-500/40 rounded-lg p-4 flex items-center gap-3">
          <AlertTriangle className="h-5 w-5 text-orange-400 shrink-0" />
          <div>
            <p className="text-orange-300 font-medium">Backend not connected</p>
            <p className="text-orange-400/70 text-sm">
              The BlockMatrix daemon at localhost:9293 is not responding. Start it with: <code className="bg-black/30 px-1 rounded">hypermesh daemon start</code>
            </p>
          </div>
        </div>
      )}

      {/* User Journey Progress */}
      <section aria-labelledby="journey-heading">
        <ScreenReaderOnly>
          <h2 id="journey-heading">Your Setup Progress</h2>
        </ScreenReaderOnly>
        <FlowIndicator
          steps={userJourneySteps}
          title="Setup Progress"
          theme="cyan"
          orientation="horizontal"
        />
      </section>

      {/* System Overview -- real data from /api/v1/status */}
      <section aria-labelledby="overview-heading">
        <h2 id="overview-heading" className="text-2xl font-bold text-white flex items-center gap-2 mb-4">
          <Activity className="h-6 w-6 text-cyan-400" />
          System Overview
          {overview.isOnline && (
            <Badge variant="outline" className="text-xs text-green-400 border-green-400 ml-2">Live</Badge>
          )}
        </h2>
        <div className="grid gap-4 md:grid-cols-4">
          <ModuleCard
            title="Chain Height"
            value={overview.chainHeight}
            subtitle={overview.isOnline ? `Node: ${overview.nodeId.slice(0, 12)}...` : "Offline"}
            icon={Zap}
            iconColor={overview.isOnline ? "text-cyan-400" : "text-gray-400"}
          />

          <ModuleCard
            title="Peers"
            value={overview.peerCount}
            subtitle={overview.isOnline ? `Privacy: ${overview.privacyMode}` : "Offline"}
            icon={Server}
            iconColor={overview.peerCount > 0 ? "text-green-400" : "text-gray-400"}
          />

          <ModuleCard
            title="Assets"
            value={overview.assetCount}
            subtitle={overview.isOnline ? "Blockchain-registered" : "Offline"}
            icon={Package}
            iconColor={overview.isOnline ? "text-purple-400" : "text-gray-400"}
          />

          <ModuleCard
            title="Uptime"
            value={overview.isOnline ? formatUptime(overview.uptimeSecs) : '--'}
            subtitle={overview.isOnline
              ? `Position: (${overview.coordinate.x},${overview.coordinate.y},${overview.coordinate.z})`
              : "Offline"}
            icon={Network}
            iconColor={overview.isOnline ? "text-blue-400" : "text-gray-400"}
          />
        </div>
      </section>

      {/* Quick Actions */}
      <section aria-labelledby="actions-heading">
        <ScreenReaderOnly>
          <h2 id="actions-heading">Quick Actions</h2>
        </ScreenReaderOnly>
        <NavigationHints
          hints={quickActions}
          title="Quick Actions"
          theme="cyan"
        />
      </section>

      {/* Main Modules */}
      <section aria-labelledby="modules-heading">
        <h2 id="modules-heading" className="text-2xl font-bold text-white flex items-center gap-2 mb-4">
          <Network className="h-6 w-6 text-cyan-400" />
          Platform Modules
        </h2>
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
          <Card className="bg-black/20 border-cyan-500/30 backdrop-blur-lg hover:border-cyan-500/50 transition-all duration-300 group">
            <CardHeader>
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600 opacity-70 group-hover:opacity-100 transition-opacity">
                  <Network className="h-5 w-5 text-black" />
                </div>
                <div>
                  <CardTitle className="text-white">HyperMesh</CardTitle>
                  <CardDescription className="text-gray-400">Resource Management</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-gray-400">
                Configure and manage system resources. Set up Private, Federated, and Public sharing modes.
              </p>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Peers:</span>
                  <span className="text-cyan-400">{overview.peerCount}</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Mode:</span>
                  <span className="text-purple-400">{overview.privacyMode}</span>
                </div>
              </div>
              <Button 
                className="w-full bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black font-medium"
                onClick={() => window.location.href = '/hypermesh'}
              >
                Manage Resources
                <ArrowRight className="h-4 w-4 ml-2" />
              </Button>
            </CardContent>
          </Card>

          <Card className="bg-black/20 border-purple-500/30 backdrop-blur-lg hover:border-purple-500/50 transition-all duration-300 group">
            <CardHeader>
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-lg bg-gradient-to-r from-purple-400 to-indigo-600 opacity-70 group-hover:opacity-100 transition-opacity">
                  <Package className="h-5 w-5 text-black" />
                </div>
                <div>
                  <CardTitle className="text-white">Catalog</CardTitle>
                  <CardDescription className="text-gray-400">Asset Package Manager</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-gray-400">
                Browse, install, and manage assets. Handle dependencies and asset types automatically.
              </p>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Installed:</span>
                  <span className="text-purple-400">{overview.assetCount} assets</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Development:</span>
                  <span className="text-yellow-400">
                    {getCrateStatus('catalog')?.features.inDevelopment.length || 0} in progress
                  </span>
                </div>
              </div>
              <Button 
                variant="outline" 
                className="w-full border-purple-500/30 text-purple-400 hover:bg-purple-500/20"
                onClick={() => window.location.href = '/catalog'}
              >
                Browse Catalog
                <ArrowRight className="h-4 w-4 ml-2" />
              </Button>
            </CardContent>
          </Card>

          <Card className="bg-black/20 border-green-500/30 backdrop-blur-lg hover:border-green-500/50 transition-all duration-300 group">
            <CardHeader>
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-lg bg-gradient-to-r from-green-400 to-emerald-600 opacity-70 group-hover:opacity-100 transition-opacity">
                  <Shield className="h-5 w-5 text-black" />
                </div>
                <div>
                  <CardTitle className="text-white">TrustChain</CardTitle>
                  <CardDescription className="text-gray-400">Network Connections</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-gray-400">
                Manage connections to Public, P2P, and Federated networks. Configure Proof of State verification.
              </p>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Peers:</span>
                  <span className="text-green-400">{overview.peerCount} connected</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">PoS Validation:</span>
                  <span className="text-green-400">Active</span>
                </div>
              </div>
              <Button 
                variant="outline" 
                className="w-full border-green-500/30 text-green-400 hover:bg-green-500/20"
                onClick={() => window.location.href = '/trustchain'}
              >
                Manage Networks
                <ArrowRight className="h-4 w-4 ml-2" />
              </Button>
            </CardContent>
          </Card>

          <Card className="bg-black/20 border-yellow-500/30 backdrop-blur-lg hover:border-yellow-500/50 transition-all duration-300 group">
            <CardHeader>
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-lg bg-gradient-to-r from-yellow-400 to-orange-600 opacity-70 group-hover:opacity-100 transition-opacity">
                  <Coins className="h-5 w-5 text-black" />
                </div>
                <div>
                  <CardTitle className="text-white">Caesar</CardTitle>
                  <CardDescription className="text-gray-400">Token Integration</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-gray-400">
                Enable token rewards for resource sharing. Access economic features and payments.
              </p>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Status:</span>
                  <span className="text-yellow-400/60">Not connected</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Backend:</span>
                  <span className="text-yellow-400/60">Caesar service required</span>
                </div>
              </div>
              <Button 
                variant="outline" 
                className="w-full border-yellow-500/30 text-yellow-400 hover:bg-yellow-500/20"
                onClick={() => window.location.href = '/caesar'}
              >
                View Wallet
                <ArrowRight className="h-4 w-4 ml-2" />
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* Activity and Node Info */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Activity className="h-5 w-5 text-cyan-400" />
              Recent Activity
              {overview.isOnline && (
                <Badge variant="outline" className="text-xs text-green-400 border-green-400">
                  Live
                </Badge>
              )}
            </CardTitle>
            <CardDescription className="text-gray-400">
              {overview.isOnline ? 'Live data from BlockMatrix daemon' : 'Daemon not connected'}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4" role="log" aria-label="Recent system activity">
              {recentActivity.map((act, index) => (
                <ActivityItem
                  key={index}
                  type={act.type as any}
                  message={act.message}
                  time={act.time}
                  theme="cyan"
                />
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Node Details Card */}
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Server className="h-5 w-5 text-cyan-400" />
              Node Details
            </CardTitle>
            <CardDescription className="text-gray-400">
              {overview.isOnline ? 'Real-time node information' : 'Connect to see node details'}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {overview.isOnline ? (
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Node ID</span>
                  <span className="text-white font-mono text-xs">{overview.nodeId}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Matrix Position</span>
                  <span className="text-cyan-400">
                    ({overview.coordinate.x}, {overview.coordinate.y}, {overview.coordinate.z})
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Chain Height</span>
                  <span className="text-white">{overview.chainHeight} blocks</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Privacy Mode</span>
                  <span className="text-purple-400">{overview.privacyMode}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Connected Peers</span>
                  <span className="text-green-400">{overview.peerCount}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Uptime</span>
                  <span className="text-blue-400">{formatUptime(overview.uptimeSecs)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Registered Assets</span>
                  <span className="text-purple-400">{overview.assetCount}</span>
                </div>
              </div>
            ) : (
              <div className="text-gray-500 text-sm py-4 text-center">
                Start the BlockMatrix daemon to see node details
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </AccessibilityWrapper>
  );
}

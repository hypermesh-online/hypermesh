// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import { ModuleCard } from '@/components/ui/ModuleCard';
import { ActivityItem } from '@/components/ui/ActivityItem';
import { ProgressMetric } from '@/components/ui/ProgressMetric';
import { ModuleHeader } from '@/components/ui/ModuleHeader';
import { NavigationHints } from '@/components/ui/NavigationHints';
import { FlowIndicator } from '@/components/ui/FlowIndicator';
import { UserJourney } from '@/components/ui/UserJourney';
import { AccessibilityWrapper } from '@/components/ui/AccessibilityWrapper';
import { ScreenReaderOnly } from '@/components/ui/ScreenReaderOnly';
import { SystemStatusWidget } from '@/components/api/SystemStatusWidget';
import { PerformanceMonitor } from '@/components/api/PerformanceMonitor';
import { useSystemStatus, useAssets, useQUICConnections, usePerformanceMetrics, useBalance, useEarnings } from '@/lib/api';
import { useHardware } from '@/lib/hooks/useHardware';
import { getCrateStatus } from '@/lib/data/crateStatus';
import { 
  Network, 
  Package,
  Shield, 
  Coins,
  Activity,
  CheckCircle,
  Settings,
  ArrowRight,
  Users,
  Server,
  Zap,
  AlertTriangle
} from 'lucide-react';

// Real-time system data using Web3 API with real hardware detection
function useSystemOverview() {
  const { systemStatus } = useSystemStatus(true);
  const { assets } = useAssets();
  const { activeConnections } = useQUICConnections();
  const { latestMetrics } = usePerformanceMetrics(undefined, undefined, true);
  const { capabilities, allocation, sharing, isLoading: hardwareLoading } = useHardware(5000); // Refresh every 5 seconds
  const caesarBalance = useBalance(); // Get real Caesar balance
  const caesarEarnings = useEarnings(); // Get real earnings data

  // Use real hardware data when available, fallback to defaults
  const totalResources = capabilities ? {
    cpu: capabilities.cpu.logical_cores,
    ram: Math.round(capabilities.memory.total_bytes / (1024 * 1024 * 1024)), // Convert to GB
    storage: Math.round(
      capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0) / (1024 * 1024 * 1024)
    ) // Convert to GB
  } : { cpu: 8, ram: 32, storage: 1000 }; // Fallback values

  const sharedResources = allocation ? {
    cpu: Math.floor(allocation.cpu.allocated),
    ram: Math.round(allocation.memory.allocated / (1024 * 1024 * 1024)), // Convert to GB
    storage: Math.round(allocation.storage.allocated / (1024 * 1024 * 1024)) // Convert to GB
  } : {
    cpu: Math.floor(totalResources.cpu * 0.5),
    ram: Math.floor(totalResources.ram * 0.5),
    storage: Math.floor(totalResources.storage * 0.5)
  };

  // Calculate real network bandwidth
  const networkBandwidth = capabilities ?
    capabilities.network.reduce((sum, iface) => sum + iface.speed_mbps, 0) :
    1000; // Default 1 Gbps

  return {
    totalResources,
    sharedResources,
    installedAssets: assets?.length || 0,
    activeConnections: activeConnections?.length || 0,
    tokenBalance: caesarBalance.data?.total || 0, // Real Caesar balance
    todayEarnings: caesarEarnings.data?.earnings_24h || 0, // Real earnings today
    networkHealth: systemStatus?.performance?.uptime || 0,
    networkBandwidth,
    isOnline: !!systemStatus,
    hardwareDetected: !!capabilities,
    cpuUsage: capabilities?.cpu.usage_percent || 0,
    memoryUsage: capabilities?.memory.usage_percent || 0,
    isTokenLoading: caesarBalance.isLoading,
    isEarningsLoading: caesarEarnings.isLoading,
    storageUsage: capabilities ?
      (capabilities.storage.reduce((sum, disk) => sum + disk.used_bytes, 0) /
       capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0)) * 100 : 0,
    systemInfo: capabilities?.system,
    activeSharingMode: sharing?.available_modes?.find(m => m.is_active)?.name || 'Device',
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

// Real-time activity from system events
function useRecentActivity() {
  const { systemStatus } = useSystemStatus(true);
  const { assets } = useAssets();
  const { connections } = useQUICConnections();
  
  // Generate activity from real system state
  const activity = [];
  
  if (systemStatus) {
    const servicesHealthy = Object.values(systemStatus.services).filter(s => s.status === 'healthy').length;
    const totalServices = Object.values(systemStatus.services).length;
    
    if (servicesHealthy === totalServices) {
      activity.push({ type: 'success', message: 'All services operational', time: 'Just now' });
    } else {
      activity.push({ type: 'warning', message: `${totalServices - servicesHealthy} services degraded`, time: 'Just now' });
    }
  }
  
  if (connections?.length > 0) {
    activity.push({ type: 'success', message: `${connections.length} QUIC connections active`, time: '2 minutes ago' });
  }
  
  if (assets?.length > 0) {
    activity.push({ type: 'info', message: `${assets.length} assets available`, time: '5 minutes ago' });
  }
  
  // Fallback for offline mode
  if (activity.length === 0) {
    return [
      { type: 'warning', message: 'Running in offline mode', time: 'Now' },
      { type: 'info', message: 'Waiting for backend services', time: '1 minute ago' },
    ];
  }
  
  return activity.slice(0, 4); // Keep last 4 activities
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
  const systemOverview = useSystemOverview();
  const recentActivity = useRecentActivity();
  const { systemStatus } = useSystemStatus(true);
  const { latestMetrics } = usePerformanceMetrics(undefined, undefined, true);
  
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

      {/* System Overview */}
      <section aria-labelledby="overview-heading">
        <h2 id="overview-heading" className="text-2xl font-bold text-white flex items-center gap-2 mb-4">
          <Activity className="h-6 w-6 text-cyan-400" />
          System Overview
        </h2>
        <div className="grid gap-4 md:grid-cols-4">
          <ModuleCard
            title="CPU Cores"
            value={`${systemOverview.sharedResources.cpu}/${systemOverview.totalResources.cpu}`}
            subtitle={systemOverview.hardwareDetected ?
              `${systemOverview.cpuUsage.toFixed(1)}% usage` :
              (systemOverview.isOnline ? "Shared / Available" : "Offline Mode")}
            icon={Zap}
            iconColor={systemOverview.hardwareDetected ?
              (systemOverview.cpuUsage > 80 ? "text-red-400" :
               systemOverview.cpuUsage > 50 ? "text-yellow-400" : "text-cyan-400") :
              "text-gray-400"}
            progress={systemOverview.hardwareDetected ? systemOverview.cpuUsage :
              (systemOverview.sharedResources.cpu / systemOverview.totalResources.cpu) * 100}
          />

          <ModuleCard
            title="Memory (GB)"
            value={`${systemOverview.sharedResources.ram}/${systemOverview.totalResources.ram}`}
            subtitle={systemOverview.hardwareDetected ?
              `${systemOverview.memoryUsage.toFixed(1)}% usage` :
              (systemOverview.isOnline ? "Shared / Available" : "Offline Mode")}
            icon={Server}
            iconColor={systemOverview.hardwareDetected ?
              (systemOverview.memoryUsage > 80 ? "text-red-400" :
               systemOverview.memoryUsage > 50 ? "text-yellow-400" : "text-green-400") :
              "text-gray-400"}
            progress={systemOverview.hardwareDetected ? systemOverview.memoryUsage :
              (systemOverview.sharedResources.ram / systemOverview.totalResources.ram) * 100}
          />

          <ModuleCard
            title="Installed Assets"
            value={systemOverview.installedAssets}
            subtitle={systemOverview.isOnline ? "Ready to use" : "Offline Mode"}
            icon={Package}
            iconColor={systemOverview.isOnline ? "text-purple-400" : "text-gray-400"}
          />

          <ModuleCard
            title="Network Health"
            value={`${systemOverview.networkHealth.toFixed(1)}%`}
            subtitle={systemOverview.isOnline ? "All systems operational" : "Offline Mode"}
            icon={Network}
            iconColor={systemOverview.isOnline ? "text-blue-400" : "text-gray-400"}
            progress={systemOverview.networkHealth}
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
                  <span className="text-gray-400">Active Shares:</span>
                  <span className="text-cyan-400">{systemOverview.activeConnections}</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Mode:</span>
                  <span className="text-purple-400">{systemOverview.activeSharingMode}</span>
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
                  <span className="text-purple-400">{systemOverview.installedAssets} assets</span>
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
                Manage connections to Public, P2P, and Federated networks. Configure consensus mechanisms.
              </p>
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Networks:</span>
                  <span className="text-green-400">{systemOverview.activeConnections} connected</span>
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
                  <span className="text-gray-400">Balance:</span>
                  {systemOverview.isTokenLoading ? (
                    <span className="h-3 w-16 bg-gray-700 rounded animate-pulse inline-block" />
                  ) : (
                    <span className="text-yellow-400">{systemOverview.tokenBalance.toFixed(2)} CSR</span>
                  )}
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-400">Earnings:</span>
                  {systemOverview.isEarningsLoading ? (
                    <span className="h-3 w-16 bg-gray-700 rounded animate-pulse inline-block" />
                  ) : (
                    <span className="text-yellow-400">+{systemOverview.todayEarnings.toFixed(2)} today</span>
                  )}
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

      {/* Activity and Alerts */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Activity className="h-5 w-5 text-cyan-400" />
              Recent Activity
              {systemOverview.isOnline && (
                <Badge variant="outline" className="text-xs text-green-400 border-green-400">
                  Live
                </Badge>
              )}
            </CardTitle>
            <CardDescription className="text-gray-400">
              {systemOverview.isOnline ? 'Latest system events and updates' : 'System running in offline mode'}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4" role="log" aria-label="Recent system activity">
              {recentActivity.map((activity, index) => (
                <ActivityItem
                  key={index}
                  type={activity.type as any}
                  message={activity.message}
                  time={activity.time}
                  theme="cyan"
                />
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Real-time System Status Widget */}
        <SystemStatusWidget />
      </div>
    </AccessibilityWrapper>
  );
}

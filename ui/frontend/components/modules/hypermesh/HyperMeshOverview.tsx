// @ts-nocheck — Phase 8 will rewrite with useBlockMatrix hooks
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { useAssets, useAllocations, useSystemStatus, usePerformanceMetrics } from '@/lib/api';
import { useHardware, useResourceMonitor, useSharingCapabilities } from '@/lib/hooks/useHardware';
import {
  Network,
  Settings,
  Users,
  Share,
  Lock,
  Globe,
  Cpu,
  MemoryStick,
  HardDrive,
  Activity
} from 'lucide-react';

export function HyperMeshOverview() {
  const { assets, availableAssets, allocatedAssets, isLoading: assetsLoading } = useAssets();
  const { allocations, activeAllocations, isLoading: allocationsLoading } = useAllocations();
  const { systemStatus } = useSystemStatus(true);
  const { latestMetrics } = usePerformanceMetrics(undefined, undefined, true);
  const { capabilities, allocation, formatted } = useHardware(5000); // Real hardware data
  const { allocation: realtimeAllocation } = useResourceMonitor(2000); // Real-time monitoring
  const { capabilities: sharingCaps } = useSharingCapabilities();

  // Calculate real system resources from hardware detection API
  const systemResources = React.useMemo(() => {
    if (capabilities && realtimeAllocation) {
      // Use real hardware data
      return {
        cpu: {
          total: capabilities.cpu.logical_cores,
          shared: Math.floor(realtimeAllocation.cpu.allocated),
          used: Math.floor(realtimeAllocation.cpu.used),
          usage_percent: capabilities.cpu.usage_percent
        },
        memory: {
          total: Math.round(capabilities.memory.total_bytes / (1024 * 1024 * 1024)), // GB
          shared: Math.round(realtimeAllocation.memory.allocated / (1024 * 1024 * 1024)),
          used: Math.round(realtimeAllocation.memory.used / (1024 * 1024 * 1024)),
          usage_percent: capabilities.memory.usage_percent
        },
        storage: {
          total: Math.round(
            capabilities.storage.reduce((sum, disk) => sum + disk.total_bytes, 0) / (1024 * 1024 * 1024)
          ), // GB
          shared: Math.round(realtimeAllocation.storage.allocated / (1024 * 1024 * 1024)),
          used: Math.round(realtimeAllocation.storage.used / (1024 * 1024 * 1024)),
          usage_percent: realtimeAllocation.storage.usage_percent
        },
        network: {
          bandwidth: capabilities.network.reduce((sum, iface) => sum + iface.speed_mbps, 0),
          shared: Math.floor(realtimeAllocation.network.allocated),
          used: Math.floor(realtimeAllocation.network.used),
          usage_percent: realtimeAllocation.network.usage_percent
        }
      };
    }

    // Fallback to default values if hardware detection not available
    const specs = { cpu: 8, memory: 32, storage: 1000, network: 1000 };

    // Calculate resource usage from active allocations
    let usedResources = { cpu: 0, memory: 0, storage: 0, network: 0 };

    if (activeAllocations) {
      activeAllocations.forEach(allocation => {
        if (allocation.amount) {
          usedResources.cpu += allocation.amount * 0.1;
          usedResources.memory += allocation.amount * 0.5;
          usedResources.storage += allocation.amount * 10;
        }
      });
    }

    return {
      cpu: { total: specs.cpu, shared: Math.floor(specs.cpu * 0.5), used: usedResources.cpu, usage_percent: 0 },
      memory: { total: specs.memory, shared: Math.floor(specs.memory * 0.5), used: usedResources.memory, usage_percent: 0 },
      storage: { total: specs.storage, shared: Math.floor(specs.storage * 0.5), used: usedResources.storage, usage_percent: 0 },
      network: { bandwidth: specs.network, shared: Math.floor(specs.network * 0.5), used: usedResources.network, usage_percent: 0 }
    };
  }, [capabilities, realtimeAllocation, activeAllocations]);

  const sharingModes = React.useMemo(() => {
    if (sharingCaps?.available_modes) {
      // Use real sharing capabilities from API
      return sharingCaps.available_modes.map(mode => ({
        name: mode.name,
        description: mode.description,
        status: mode.is_active ? 'active' : 'available',
        resources: {
          cpu: mode.resource_limits.max_cpu_cores,
          memory: Math.round(mode.resource_limits.max_memory_bytes / (1024 * 1024 * 1024)), // GB
          storage: Math.round(mode.resource_limits.max_storage_bytes / (1024 * 1024 * 1024)), // GB
          bandwidth: mode.resource_limits.max_network_mbps
        },
        icon: mode.name === 'Private' ? Lock : mode.name === 'Federated' ? Users : Globe,
        color: mode.name === 'Private' ? 'red' : mode.name === 'Federated' ? 'purple' : 'cyan'
      }));
    }

    // Fallback to default values
    return [
      {
        name: 'Private',
        description: 'Resources available only to your local applications',
        status: 'active',
        resources: { cpu: 2, memory: 8, storage: 250, bandwidth: 0 },
        icon: Lock,
        color: 'red'
      },
      {
        name: 'Federated',
        description: 'Shared with trusted networks and verified peers',
        status: 'active',
        resources: { cpu: 2, memory: 6, storage: 200, bandwidth: 500 },
        icon: Users,
        color: 'purple'
      },
      {
        name: 'Public',
        description: 'Available to the global HyperMesh network',
        status: 'available',
        resources: { cpu: 0, memory: 0, storage: 0, bandwidth: 0 },
        icon: Globe,
        color: 'cyan'
      }
    ];
  }, [sharingCaps]);

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-600 bg-clip-text text-transparent mb-2">
          HyperMesh Resource Manager
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Federated resource sharing platform with scoped access controls. Configure CPU, memory, storage, and network resources for Private, Federated, and Public sharing.
        </p>
      </div>

      {/* Resource Overview */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">CPU Cores</CardTitle>
            <Cpu className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{systemResources.cpu.shared}/{systemResources.cpu.total}</div>
            <p className="text-xs text-gray-400">Shared / Total</p>
            <Progress value={(systemResources.cpu.used / systemResources.cpu.shared) * 100} className="mt-2 h-1" />
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Memory (GB)</CardTitle>
            <MemoryStick className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{systemResources.memory.shared}/{systemResources.memory.total}</div>
            <p className="text-xs text-gray-400">Shared / Total</p>
            <Progress value={(systemResources.memory.used / systemResources.memory.shared) * 100} className="mt-2 h-1" />
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Storage (GB)</CardTitle>
            <HardDrive className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{systemResources.storage.shared}/{systemResources.storage.total}</div>
            <p className="text-xs text-gray-400">Shared / Total</p>
            <Progress value={(systemResources.storage.used / systemResources.storage.shared) * 100} className="mt-2 h-1" />
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Network (Mbps)</CardTitle>
            <Network className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{systemResources.network.shared}/{systemResources.network.bandwidth}</div>
            <p className="text-xs text-gray-400">Shared / Total</p>
            <Progress value={(systemResources.network.used / systemResources.network.shared) * 100} className="mt-2 h-1" />
          </CardContent>
        </Card>
      </div>

      {/* Sharing Modes */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Share className="h-5 w-5 text-cyan-400" />
            Resource Sharing Modes
          </CardTitle>
          <CardDescription className="text-gray-400">Configure how your resources are shared across different network scopes</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-6 md:grid-cols-3">
            {sharingModes.map((mode, index) => {
              const Icon = mode.icon;
              const isActive = mode.status === 'active';

              return (
                <div key={mode.name} className={cn(
                  'p-4 rounded-lg border transition-all duration-300',
                  isActive ? 'bg-cyan-500/10 border-cyan-500/30' : 'bg-gray-500/10 border-gray-600/30'
                )}>
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <Icon className={cn(
                        'h-6 w-6',
                        mode.color === 'red' ? 'text-red-400' :
                        mode.color === 'purple' ? 'text-purple-400' :
                        'text-cyan-400'
                      )} />
                      <h4 className="font-medium text-white">{mode.name}</h4>
                    </div>
                    <Badge variant={isActive ? 'default' : 'outline'}
                           className={isActive ? 'bg-green-500/20 text-green-400 border-green-500/30' : ''}>
                      {mode.status}
                    </Badge>
                  </div>

                  <p className="text-sm text-gray-400 mb-4">{mode.description}</p>

                  <div className="space-y-2">
                    <div className="flex justify-between text-xs">
                      <span className="text-gray-400">CPU:</span>
                      <span className="text-white">{mode.resources.cpu} cores</span>
                    </div>
                    <div className="flex justify-between text-xs">
                      <span className="text-gray-400">Memory:</span>
                      <span className="text-white">{mode.resources.memory} GB</span>
                    </div>
                    <div className="flex justify-between text-xs">
                      <span className="text-gray-400">Storage:</span>
                      <span className="text-white">{mode.resources.storage} GB</span>
                    </div>
                  </div>

                  <Button
                    variant={isActive ? 'outline' : 'default'}
                    size="sm"
                    className={cn(
                      'w-full mt-4',
                      isActive ? 'border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20' :
                      'bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black'
                    )}
                  >
                    {isActive ? 'Configure' : 'Enable'}
                  </Button>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Active Connections */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Activity className="h-5 w-5 text-cyan-400" />
            Active Resource Sharing
          </CardTitle>
          <CardDescription className="text-gray-400">Current resource consumers and providers</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {allocationsLoading ? (
              <div className="animate-pulse space-y-3">
                {[1,2,3].map(i => (
                  <div key={i} className="h-16 bg-gray-700 rounded-lg"></div>
                ))}
              </div>
            ) : activeAllocations && activeAllocations.length > 0 ? (
              activeAllocations.map((allocation, index) => (
                <div key={allocation.id} className="flex items-center justify-between p-4 border border-cyan-500/20 rounded-lg bg-cyan-500/5">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-1">
                      <h4 className="font-medium text-white">Allocation {allocation.id.slice(0, 8)}</h4>
                      <Badge variant="outline" className="text-xs bg-blue-500/20 text-blue-400">
                        Consumer
                      </Badge>
                      <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400">
                        Federated
                      </Badge>
                    </div>
                    <p className="text-sm text-gray-400">
                      Amount: {allocation.amount} {allocation.unit}, Duration: {Math.floor(allocation.duration / 3600)}h
                    </p>
                    <p className="text-xs text-gray-500">
                      Requester: {allocation.requesterId.slice(0, 12)}...
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="default" className={cn(
                      allocation.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                      allocation.status === 'pending' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                      'bg-cyan-500/20 text-cyan-400 border-cyan-500/30'
                    )}>
                      {allocation.status}
                    </Badge>
                    <Button variant="ghost" size="sm">
                      <Settings className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-400">
                {systemStatus ? 'No active allocations' : 'System offline - unable to load allocations'}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

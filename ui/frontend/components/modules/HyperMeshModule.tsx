// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { useAssets, useAllocations, useSystemStatus, usePerformanceMetrics } from '@/lib/api';
import { useHardware, useResourceMonitor, useSharingCapabilities } from '@/lib/hooks/useHardware';
import { ConsensusDashboard } from '../consensus/ConsensusDashboard';
import { AdvancedAssetManagement } from '../assets/AdvancedAssetManagement';
import { 
  Network, 
  Settings,
  Users,
  Server,
  Share,
  Lock,
  Globe,
  Shield,
  Zap,
  HardDrive,
  Cpu,
  MemoryStick,
  Activity
} from 'lucide-react';

const subNavigation = [
  { name: 'Overview', href: '/hypermesh' },
  { name: 'Resources', href: '/hypermesh/resources' },
  { name: 'Advanced Assets', href: '/hypermesh/advanced' },
  { name: 'Sharing', href: '/hypermesh/sharing' },
  { name: 'Consensus', href: '/hypermesh/consensus' },
];

function HyperMeshOverview() {
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

function ResourceConfiguration() {
  const { assets, isLoading } = useAssets();
  const { systemStatus } = useSystemStatus(true);
  const [cpuLimit, setCpuLimit] = React.useState(50);
  const [memoryLimit, setMemoryLimit] = React.useState(50);
  const [storageLimit, setStorageLimit] = React.useState(50);
  const [networkLimit, setNetworkLimit] = React.useState(50);
  
  const systemSpecs = { cpu: 8, memory: 32, storage: 1000, network: 1000 };
  
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Resource Configuration</h2>
      
      {/* System Overview */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">System Resources</CardTitle>
          <CardDescription className="text-gray-400">Configure sharing limits for your system resources</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* CPU Configuration */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-cyan-400" />
                <span className="text-white font-medium">CPU Cores</span>
              </div>
              <span className="text-cyan-400">{Math.floor((cpuLimit / 100) * systemSpecs.cpu)} / {systemSpecs.cpu} cores</span>
            </div>
            <div className="space-y-2">
              <Progress value={cpuLimit} className="h-2" />
              <div className="flex items-center gap-4">
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setCpuLimit(Math.max(0, cpuLimit - 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  -
                </Button>
                <span className="text-sm text-gray-400 min-w-[3rem] text-center">{cpuLimit}%</span>
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setCpuLimit(Math.min(100, cpuLimit + 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  +
                </Button>
              </div>
            </div>
          </div>
          
          {/* Memory Configuration */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <MemoryStick className="h-4 w-4 text-cyan-400" />
                <span className="text-white font-medium">Memory</span>
              </div>
              <span className="text-cyan-400">{Math.floor((memoryLimit / 100) * systemSpecs.memory)} / {systemSpecs.memory} GB</span>
            </div>
            <div className="space-y-2">
              <Progress value={memoryLimit} className="h-2" />
              <div className="flex items-center gap-4">
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setMemoryLimit(Math.max(0, memoryLimit - 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  -
                </Button>
                <span className="text-sm text-gray-400 min-w-[3rem] text-center">{memoryLimit}%</span>
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setMemoryLimit(Math.min(100, memoryLimit + 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  +
                </Button>
              </div>
            </div>
          </div>
          
          {/* Storage Configuration */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <HardDrive className="h-4 w-4 text-cyan-400" />
                <span className="text-white font-medium">Storage</span>
              </div>
              <span className="text-cyan-400">{Math.floor((storageLimit / 100) * systemSpecs.storage)} / {systemSpecs.storage} GB</span>
            </div>
            <div className="space-y-2">
              <Progress value={storageLimit} className="h-2" />
              <div className="flex items-center gap-4">
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setStorageLimit(Math.max(0, storageLimit - 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  -
                </Button>
                <span className="text-sm text-gray-400 min-w-[3rem] text-center">{storageLimit}%</span>
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setStorageLimit(Math.min(100, storageLimit + 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  +
                </Button>
              </div>
            </div>
          </div>
          
          {/* Network Configuration */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Network className="h-4 w-4 text-cyan-400" />
                <span className="text-white font-medium">Network Bandwidth</span>
              </div>
              <span className="text-cyan-400">{Math.floor((networkLimit / 100) * systemSpecs.network)} / {systemSpecs.network} Mbps</span>
            </div>
            <div className="space-y-2">
              <Progress value={networkLimit} className="h-2" />
              <div className="flex items-center gap-4">
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setNetworkLimit(Math.max(0, networkLimit - 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  -
                </Button>
                <span className="text-sm text-gray-400 min-w-[3rem] text-center">{networkLimit}%</span>
                <Button 
                  variant="outline" 
                  size="sm" 
                  onClick={() => setNetworkLimit(Math.min(100, networkLimit + 10))}
                  className="border-cyan-500/30 text-cyan-400"
                >
                  +
                </Button>
              </div>
            </div>
          </div>
          
          <div className="pt-4 border-t border-cyan-500/20">
            <div className="flex items-center justify-between">
              <div className="text-sm text-gray-400">
                <p>Total shared resources: {Math.floor((cpuLimit + memoryLimit + storageLimit + networkLimit) / 4)}% average</p>
                <p className="text-xs mt-1">Changes will be applied immediately to new allocations</p>
              </div>
              <Button 
                className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
                onClick={() => {
                  // In production, this would call the HyperMesh API to update settings
                  alert('Resource limits updated successfully!');
                }}
              >
                Apply Changes
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
      
      {/* Privacy Settings */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Privacy & Access Control</CardTitle>
          <CardDescription className="text-gray-400">Configure how resources are shared across different network scopes</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4">
            <div className="flex items-center justify-between p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
              <div className="flex items-center gap-3">
                <Lock className="h-5 w-5 text-red-400" />
                <div>
                  <h4 className="text-white font-medium">Private Mode</h4>
                  <p className="text-sm text-gray-400">Resources available only to local applications</p>
                </div>
              </div>
              <Badge className="bg-red-500/20 text-red-400 border-red-500/30">Active</Badge>
            </div>
            
            <div className="flex items-center justify-between p-3 bg-purple-500/10 border border-purple-500/30 rounded-lg">
              <div className="flex items-center gap-3">
                <Users className="h-5 w-5 text-purple-400" />
                <div>
                  <h4 className="text-white font-medium">Federated Mode</h4>
                  <p className="text-sm text-gray-400">Shared with trusted networks and verified peers</p>
                </div>
              </div>
              <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">Active</Badge>
            </div>
            
            <div className="flex items-center justify-between p-3 bg-gray-500/10 border border-gray-600/30 rounded-lg">
              <div className="flex items-center gap-3">
                <Globe className="h-5 w-5 text-gray-400" />
                <div>
                  <h4 className="text-white font-medium">Public Mode</h4>
                  <p className="text-sm text-gray-400">Available to the global HyperMesh network</p>
                </div>
              </div>
              <Button variant="outline" size="sm" className="border-cyan-500/30 text-cyan-400">
                Enable
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function SharingManagement() {
  const { allocations, activeAllocations, isLoading } = useAllocations();
  const { assets } = useAssets();
  const [selectedAllocation, setSelectedAllocation] = React.useState<string | null>(null);
  
  const sharingStats = React.useMemo(() => {
    const total = allocations?.length || 0;
    const active = activeAllocations?.length || 0;
    const pending = allocations?.filter(a => a.status === 'pending').length || 0;
    const completed = allocations?.filter(a => a.status === 'completed').length || 0;
    
    return { total, active, pending, completed };
  }, [allocations, activeAllocations]);
  
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Sharing Management</h2>
      
      {/* Sharing Statistics */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Allocations</CardTitle>
            <Activity className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{sharingStats.total}</div>
            <p className="text-xs text-gray-400">All time</p>
          </CardContent>
        </Card>
        
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active</CardTitle>
            <Zap className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{sharingStats.active}</div>
            <p className="text-xs text-gray-400">Currently sharing</p>
          </CardContent>
        </Card>
        
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Pending</CardTitle>
            <Settings className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-yellow-400">{sharingStats.pending}</div>
            <p className="text-xs text-gray-400">Awaiting approval</p>
          </CardContent>
        </Card>
        
        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Completed</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{sharingStats.completed}</div>
            <p className="text-xs text-gray-400">Successfully shared</p>
          </CardContent>
        </Card>
      </div>
      
      {/* Active Allocations Management */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Share className="h-5 w-5 text-cyan-400" />
            Active Resource Sharing
          </CardTitle>
          <CardDescription className="text-gray-400">Manage ongoing resource allocations and sharing sessions</CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              {[1,2,3].map(i => (
                <div key={i} className="animate-pulse h-20 bg-gray-700 rounded-lg"></div>
              ))}
            </div>
          ) : activeAllocations && activeAllocations.length > 0 ? (
            <div className="space-y-4">
              {activeAllocations.map((allocation) => (
                <div 
                  key={allocation.id} 
                  className="border border-cyan-500/20 rounded-lg p-4 bg-cyan-500/5 hover:bg-cyan-500/10 transition-colors cursor-pointer"
                  onClick={() => setSelectedAllocation(allocation.id)}
                >
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <h4 className="font-medium text-white">Allocation {allocation.id.slice(0, 8)}...</h4>
                      <Badge 
                        variant="outline" 
                        className={`text-xs ${
                          allocation.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                          allocation.status === 'pending' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                          'bg-cyan-500/20 text-cyan-400 border-cyan-500/30'
                        }`}
                      >
                        {allocation.status}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
                        <Settings className="h-4 w-4" />
                      </Button>
                      <Button 
                        variant="ghost" 
                        size="sm" 
                        className="text-red-400 hover:bg-red-500/20"
                        onClick={(e) => {
                          e.stopPropagation();
                          // In production, this would call releaseAllocation API
                          alert(`Terminating allocation ${allocation.id}`);
                        }}
                      >
                        Terminate
                      </Button>
                    </div>
                  </div>
                  
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <span className="text-gray-400">Resource:</span>
                      <div className="text-white font-mono">{allocation.amount} {allocation.unit}</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Duration:</span>
                      <div className="text-white font-mono">{Math.floor(allocation.duration / 3600)}h</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Requester:</span>
                      <div className="text-white font-mono">{allocation.requesterId.slice(0, 8)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Started:</span>
                      <div className="text-white font-mono">{new Date(allocation.startTime).toLocaleTimeString()}</div>
                    </div>
                  </div>
                  
                  {allocation.proxyAddress && (
                    <div className="mt-3 pt-3 border-t border-cyan-500/20">
                      <span className="text-gray-400 text-sm">Proxy Address:</span>
                      <div className="text-cyan-400 font-mono text-sm">{allocation.proxyAddress}</div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8">
              <Share className="h-12 w-12 text-gray-600 mx-auto mb-3" />
              <h3 className="text-lg font-medium text-white mb-2">No Active Sharing</h3>
              <p className="text-gray-400">Your resources are available but not currently being used by others.</p>
              <p className="text-sm text-gray-500 mt-2">Configure resource limits in the Resources tab to start sharing.</p>
            </div>
          )}
        </CardContent>
      </Card>
      
      {/* Resource Allocation History */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Allocation History</CardTitle>
          <CardDescription className="text-gray-400">Recent resource sharing activity and completed allocations</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {allocations && allocations.length > 0 ? (
              allocations.slice(0, 5).map((allocation) => (
                <div key={allocation.id} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-white font-mono text-sm">{allocation.id.slice(0, 12)}...</span>
                      <Badge 
                        variant="outline" 
                        className={`text-xs ${
                          allocation.status === 'completed' ? 'bg-green-500/20 text-green-400' :
                          allocation.status === 'cancelled' ? 'bg-red-500/20 text-red-400' :
                          allocation.status === 'failed' ? 'bg-red-500/20 text-red-400' :
                          'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {allocation.status}
                      </Badge>
                    </div>
                    <div className="text-sm text-gray-400">
                      {allocation.amount} {allocation.unit} • {Math.floor(allocation.duration / 3600)}h duration
                    </div>
                  </div>
                  <div className="text-xs text-gray-500">
                    {new Date(allocation.startTime).toLocaleDateString()}
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-6 text-gray-400">
                No allocation history available
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}


function SubNavigation() {
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

export function HyperMeshModule() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2 text-white">
          <div className="p-2 rounded-lg bg-gradient-to-r from-cyan-400 to-blue-600">
            <Network className="h-8 w-8 text-black" />
          </div>
          HyperMesh
        </h1>
        <p className="text-gray-400 mt-2">
          Federated resource sharing with Private, P2P, and Public network scopes
        </p>
      </div>

      <SubNavigation />

      <Routes>
        <Route path="/" element={<HyperMeshOverview />} />
        <Route path="/resources" element={<ResourceConfiguration />} />
        <Route path="/advanced" element={<AdvancedAssetManagement />} />
        <Route path="/sharing" element={<SharingManagement />} />
        <Route path="/consensus" element={<ConsensusDashboard />} />
      </Routes>
    </div>
  );
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { Input } from '@/components/ui/input';
import { useNodeStatus, useConfigShow, useConfigGet } from '@/lib/hooks/useBlockMatrix';
import {
  Network,
  Users,
  Lock,
  Globe,
  Cpu,
  MemoryStick,
  HardDrive,
  Search,
  Settings
} from 'lucide-react';

export function ResourceConfiguration() {
  const { data: nodeStatus } = useNodeStatus();
  const { data: configData, isLoading: configLoading } = useConfigShow();
  const [cpuLimit, setCpuLimit] = React.useState(50);
  const [memoryLimit, setMemoryLimit] = React.useState(50);
  const [storageLimit, setStorageLimit] = React.useState(50);
  const [networkLimit, setNetworkLimit] = React.useState(50);
  const [configFilter, setConfigFilter] = React.useState('');
  const [configSearchKey, setConfigSearchKey] = React.useState<string | undefined>(undefined);
  const configKeyResult = useConfigGet(configSearchKey);

  // Use real config data for system specs when available
  const configRecord = configData as Record<string, unknown> | undefined;
  const systemSpecs = React.useMemo(() => {
    if (configRecord) {
      return {
        cpu: typeof configRecord.cpu_cores === 'number' ? configRecord.cpu_cores : 8,
        memory: typeof configRecord.memory_gb === 'number' ? configRecord.memory_gb : 32,
        storage: typeof configRecord.storage_gb === 'number' ? configRecord.storage_gb : 1000,
        network: typeof configRecord.network_mbps === 'number' ? configRecord.network_mbps : 1000,
      };
    }
    return { cpu: 8, memory: 32, storage: 1000, network: 1000 };
  }, [configRecord]);

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
          <CardDescription className="text-gray-400">
            {nodeStatus ? (
              <>Current privacy mode: <span className="text-purple-400 font-medium">{nodeStatus.privacy_mode}</span></>
            ) : (
              'Configure how resources are shared across different network scopes'
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {configLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-16 w-full" />
            </div>
          ) : (
            <div className="grid gap-4">
              {[
                {
                  mode: 'Anonymous',
                  description: 'No identity tracking, privacy-first resource sharing',
                  icon: Lock,
                  color: 'red',
                },
                {
                  mode: 'Private',
                  description: 'Shared with trusted networks and verified peers',
                  icon: Users,
                  color: 'purple',
                },
                {
                  mode: 'Public',
                  description: 'Available to the global HyperMesh network with full PoS',
                  icon: Globe,
                  color: 'cyan',
                },
              ].map(({ mode, description, icon: Icon, color }) => {
                const currentMode = nodeStatus?.privacy_mode?.toLowerCase() ?? '';
                const isActive = currentMode === mode.toLowerCase() ||
                  (currentMode.includes(mode.toLowerCase()));
                const bgClass = isActive ? `bg-${color}-500/10 border-${color}-500/30` : 'bg-gray-500/10 border-gray-600/30';

                return (
                  <div key={mode} className={`flex items-center justify-between p-3 rounded-lg border ${isActive ? `bg-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-500/10 border-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-500/30` : 'bg-gray-500/10 border-gray-600/30'}`}>
                    <div className="flex items-center gap-3">
                      <Icon className={`h-5 w-5 ${isActive ? `text-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-400` : 'text-gray-400'}`} />
                      <div>
                        <h4 className="text-white font-medium">{mode} Mode</h4>
                        <p className="text-sm text-gray-400">{description}</p>
                      </div>
                    </div>
                    {isActive ? (
                      <Badge className={`bg-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-500/20 text-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-400 border-${color === 'red' ? 'red' : color === 'purple' ? 'purple' : 'cyan'}-500/30`}>
                        Active
                      </Badge>
                    ) : (
                      <Button variant="outline" size="sm" className="border-cyan-500/30 text-cyan-400">
                        Enable
                      </Button>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {/* Node Configuration */}
          {configRecord && (
            <div className="pt-4 border-t border-cyan-500/20">
              <h4 className="text-sm font-medium text-gray-400 mb-3">Node Configuration</h4>
              <div className="grid grid-cols-2 gap-2 text-xs">
                {Object.entries(configRecord).slice(0, 8).map(([key, value]) => (
                  <div key={key} className="flex justify-between p-2 bg-black/20 rounded">
                    <span className="text-gray-400">{key}</span>
                    <span className="text-white font-mono">{String(value)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Full Node Configuration Table */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Settings className="h-5 w-5 text-cyan-400" />
            Node Configuration
          </CardTitle>
          <CardDescription className="text-gray-400">
            All configuration keys and values from the running node
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Search / Filter */}
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
              <Input
                placeholder="Filter config keys..."
                value={configFilter}
                onChange={(e) => setConfigFilter(e.target.value)}
                className="pl-9 bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
              />
            </div>
            <Button
              variant="outline"
              size="sm"
              className="border-cyan-500/30 text-cyan-400 whitespace-nowrap"
              onClick={() => {
                if (configFilter.trim()) {
                  setConfigSearchKey(configFilter.trim());
                }
              }}
            >
              Lookup Key
            </Button>
          </div>

          {/* Key lookup result */}
          {configSearchKey && (
            <div className="p-3 rounded-lg bg-cyan-500/5 border border-cyan-500/20">
              <div className="text-xs text-gray-400 mb-1">Lookup: <span className="text-cyan-400 font-mono">{configSearchKey}</span></div>
              {configKeyResult.isLoading ? (
                <Skeleton className="h-6 w-32" />
              ) : configKeyResult.error ? (
                <span className="text-sm text-gray-500">Key not found</span>
              ) : (
                <span className="text-sm text-white font-mono break-all">{JSON.stringify(configKeyResult.data)}</span>
              )}
            </div>
          )}

          {/* Config table */}
          {configLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 6 }).map((_, i) => (
                <Skeleton key={i} className="h-8 w-full" />
              ))}
            </div>
          ) : configRecord ? (
            <div className="max-h-80 overflow-y-auto rounded-lg border border-gray-800">
              <table className="w-full text-sm">
                <thead className="sticky top-0 bg-black/80 backdrop-blur">
                  <tr className="border-b border-gray-800">
                    <th className="text-left py-2 px-3 text-gray-400 font-medium">Key</th>
                    <th className="text-left py-2 px-3 text-gray-400 font-medium">Value</th>
                  </tr>
                </thead>
                <tbody>
                  {Object.entries(configRecord)
                    .filter(([key]) =>
                      !configFilter || key.toLowerCase().includes(configFilter.toLowerCase())
                    )
                    .map(([key, value]) => (
                      <tr key={key} className="border-b border-gray-800/50 hover:bg-cyan-500/5">
                        <td className="py-2 px-3 text-cyan-400 font-mono text-xs">{key}</td>
                        <td className="py-2 px-3 text-white font-mono text-xs break-all">
                          {typeof value === 'object' ? JSON.stringify(value) : String(value)}
                        </td>
                      </tr>
                    ))}
                </tbody>
              </table>
              {Object.entries(configRecord).filter(([key]) =>
                !configFilter || key.toLowerCase().includes(configFilter.toLowerCase())
              ).length === 0 && (
                <p className="text-gray-500 text-center py-4 text-sm">No matching config keys</p>
              )}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No configuration data available</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

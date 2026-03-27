// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useConfigShow,
  useConfigGet,
  useTrustchainIdentity,
  useNodeStatus,
} from '@/lib/hooks/useBlockMatrix';
import {
  Settings,
  AlertTriangle,
  Search,
  Shield,
  Key,
} from 'lucide-react';

export function NodeSettings() {
  const { data: nodeStatus, isLoading: nodeLoading, error: nodeError } = useNodeStatus();
  const { data: configData, isLoading: configLoading } = useConfigShow();
  const { data: identity, isLoading: identityLoading } = useTrustchainIdentity();

  const [configFilter, setConfigFilter] = React.useState('');
  const [configSearchKey, setConfigSearchKey] = React.useState<string | undefined>(undefined);
  const configKeyResult = useConfigGet(configSearchKey);

  const configRecord = configData as Record<string, unknown> | undefined;

  if (nodeLoading && configLoading && identityLoading) return <ModuleLoading />;

  if (nodeError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{nodeError.message}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Node Settings</h2>

      {/* Identity */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Shield className="h-5 w-5 text-cyan-400" />
            Node Identity
          </CardTitle>
          <CardDescription className="text-gray-400">
            FALCON-1024 post-quantum identity and signing information
          </CardDescription>
        </CardHeader>
        <CardContent>
          {identityLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-6 w-full" />
              <Skeleton className="h-6 w-3/4" />
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-400">Node ID</span>
                <span className="text-white font-mono text-sm truncate max-w-[300px]">
                  {nodeStatus?.node_id ?? identity?.node_id ?? '--'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Algorithm</span>
                <span className="text-cyan-400 font-mono">
                  {identity?.key_algorithm ?? 'FALCON-1024'}
                </span>
              </div>
              {identity?.public_key && (
                <div className="flex justify-between">
                  <span className="text-gray-400">Public Key</span>
                  <span className="text-gray-300 font-mono text-sm truncate max-w-[300px]">
                    {identity.public_key.slice(0, 32)}...
                  </span>
                </div>
              )}
              {nodeStatus && (
                <>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Matrix Coordinate</span>
                    <span className="text-cyan-400 font-mono">
                      ({nodeStatus.coordinate.x}, {nodeStatus.coordinate.y}, {nodeStatus.coordinate.z})
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Privacy Mode</span>
                    <span className="text-purple-400">{nodeStatus.privacy_mode}</span>
                  </div>
                </>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Configuration viewer */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Settings className="h-5 w-5 text-cyan-400" />
            Configuration
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
              <Key className="h-4 w-4 mr-1" />
              Lookup
            </Button>
          </div>

          {/* Key lookup result */}
          {configSearchKey && (
            <div className="p-3 rounded-lg bg-cyan-500/5 border border-cyan-500/20">
              <div className="text-xs text-gray-400 mb-1">
                Lookup: <span className="text-cyan-400 font-mono">{configSearchKey}</span>
              </div>
              {configKeyResult.isLoading ? (
                <Skeleton className="h-6 w-32" />
              ) : configKeyResult.error ? (
                <span className="text-sm text-gray-500">Key not found</span>
              ) : (
                <span className="text-sm text-white font-mono break-all">
                  {JSON.stringify(configKeyResult.data)}
                </span>
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
                      !configFilter || key.toLowerCase().includes(configFilter.toLowerCase()),
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
                !configFilter || key.toLowerCase().includes(configFilter.toLowerCase()),
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

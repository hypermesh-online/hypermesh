// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Monitor, CheckCircle, RefreshCw, XCircle,
  Activity, Link, Eye, Settings
} from 'lucide-react';
import type { ProxyMapping } from './types';

interface RemoteAccessTabProps {
  proxyMappings: ProxyMapping[];
}

export function RemoteAccessTab({ proxyMappings }: RemoteAccessTabProps) {
  const activeMappings = proxyMappings.filter(m => m.status === 'active');
  const connectingMappings = proxyMappings.filter(m => m.status === 'connecting');
  const errorMappings = proxyMappings.filter(m => m.status === 'error');

  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Monitor className="h-5 w-5 text-purple-400" />
          Remote Resource Access
        </CardTitle>
        <CardDescription className="text-gray-400">
          Access remote resources through NAT-like proxy addresses with security validation
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          <div className="grid gap-4 md:grid-cols-3">
            <Card className="bg-green-500/10 border-green-500/30">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <CheckCircle className="h-5 w-5 text-green-400" />
                  <div>
                    <div className="text-green-400 font-medium">{activeMappings.length} Active</div>
                    <div className="text-sm text-gray-400">Ready for access</div>
                  </div>
                </div>
              </CardContent>
            </Card>
            <Card className="bg-yellow-500/10 border-yellow-500/30">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <RefreshCw className="h-5 w-5 text-yellow-400" />
                  <div>
                    <div className="text-yellow-400 font-medium">{connectingMappings.length} Connecting</div>
                    <div className="text-sm text-gray-400">Establishing links</div>
                  </div>
                </div>
              </CardContent>
            </Card>
            <Card className="bg-red-500/10 border-red-500/30">
              <CardContent className="p-4">
                <div className="flex items-center gap-3">
                  <XCircle className="h-5 w-5 text-red-400" />
                  <div>
                    <div className="text-red-400 font-medium">{errorMappings.length} Errors</div>
                    <div className="text-sm text-gray-400">Need attention</div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>

          <div className="space-y-3">
            <h4 className="text-white font-medium flex items-center gap-2">
              <Activity className="h-4 w-4 text-purple-400" />
              Active Remote Connections
            </h4>

            {activeMappings.length > 0 ? (
              <div className="space-y-3">
                {activeMappings.map((mapping) => (
                  <div key={mapping.id} className="flex items-center justify-between p-3 bg-purple-500/5 border border-purple-500/20 rounded-lg">
                    <div className="flex-1">
                      <div className="flex items-center gap-3 mb-2">
                        <Link className="h-4 w-4 text-purple-400" />
                        <span className="text-white font-mono text-sm">{mapping.virtualAddress}</span>
                        <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400">
                          Connected
                        </Badge>
                      </div>
                      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
                        <div>
                          <span className="text-gray-400">Proxy:</span>
                          <div className="text-purple-400 font-mono">{mapping.proxyAddress}</div>
                        </div>
                        <div>
                          <span className="text-gray-400">Latency:</span>
                          <div className="text-white">{mapping.latency.toFixed(1)} ms</div>
                        </div>
                        <div>
                          <span className="text-gray-400">Bandwidth:</span>
                          <div className="text-white">{mapping.bandwidth.toFixed(0)} Mbps</div>
                        </div>
                        <div>
                          <span className="text-gray-400">Sessions:</span>
                          <div className="text-white">{mapping.connectionCount}</div>
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
                        <Eye className="h-4 w-4" />
                      </Button>
                      <Button variant="ghost" size="sm" className="text-purple-400 hover:bg-purple-500/20">
                        <Settings className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-6 text-gray-400">
                <Activity className="h-8 w-8 mx-auto mb-2 text-gray-600" />
                <p>No active remote connections</p>
                <p className="text-sm">Configure proxy mappings to enable remote access</p>
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

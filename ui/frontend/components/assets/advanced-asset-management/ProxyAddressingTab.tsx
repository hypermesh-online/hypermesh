// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Globe, MapPin, Activity } from 'lucide-react';
import type { ProxyAddress } from './types';

interface ProxyAddressingTabProps {
  proxyAddresses: ProxyAddress[];
}

export function ProxyAddressingTab({ proxyAddresses }: ProxyAddressingTabProps) {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Globe className="h-5 w-5 text-cyan-400" />
          NAT-like Proxy Addressing
        </CardTitle>
        <CardDescription className="text-gray-400">IPv6-like addressing system for remote asset access with trust-based routing</CardDescription>
      </CardHeader>
      <CardContent>
        {proxyAddresses.length > 0 ? (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {proxyAddresses.map((proxy) => (
              <div key={proxy.id} className="flex items-center justify-between p-4 bg-cyan-500/5 border border-cyan-500/20 rounded-lg">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <MapPin className="h-4 w-4 text-cyan-400" />
                    <span className="text-white font-mono text-sm">{proxy.virtualAddress}</span>
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      proxy.accessLevel === 'private' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                      proxy.accessLevel === 'federated' ? 'bg-blue-500/20 text-blue-400 border-blue-500/30' :
                      'bg-green-500/20 text-green-400 border-green-500/30'
                    )}>
                      {proxy.accessLevel}
                    </Badge>
                  </div>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
                    <div>
                      <span className="text-gray-400">Physical Address:</span>
                      <div className="text-cyan-400 font-mono">{proxy.physicalAddress}</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Bandwidth:</span>
                      <div className="text-white">{proxy.bandwidth.toFixed(0)} Mbps</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Latency:</span>
                      <div className="text-white">{proxy.latency.toFixed(1)} ms</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Validation:</span>
                      <Badge variant="outline" className={cn('text-xs',
                        proxy.validationStatus === 'verified' ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
                      )}>
                        {proxy.validationStatus === 'verified' ? 'Verified' : 'Rejected'}
                      </Badge>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Button variant="ghost" size="sm" className="text-cyan-400 hover:bg-cyan-500/20">
                    <Activity className="h-4 w-4" />
                  </Button>
                  <Button variant="ghost" size="sm" className="text-green-400 hover:bg-green-500/20">
                    Test
                  </Button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Globe className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Proxy Addresses</h3>
            <p className="text-gray-400">Create assets to automatically generate NAT-like proxy addresses for remote access.</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

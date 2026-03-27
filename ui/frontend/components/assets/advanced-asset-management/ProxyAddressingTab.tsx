// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Globe, Info } from 'lucide-react';

export function ProxyAddressingTab() {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Globe className="h-5 w-5 text-cyan-400" />
          NAT-like Proxy Addressing
        </CardTitle>
        <CardDescription className="text-gray-400">
          IPv6-like addressing system for remote asset access with trust-based routing
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="text-center py-8">
          <Info className="h-10 w-10 text-gray-600 mx-auto mb-3" />
          <h3 className="text-lg font-medium text-white mb-2">Coming Soon</h3>
          <p className="text-gray-400 text-sm max-w-md mx-auto">
            Proxy addressing will be available once remote asset access is exposed
            through the IPC daemon. Each asset gets an IPv6 ULA address
            (fd48:4d00 prefix) for NAT-like proxy routing.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

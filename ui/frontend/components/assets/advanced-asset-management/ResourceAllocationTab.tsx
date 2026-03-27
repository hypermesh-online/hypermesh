// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Share, Info } from 'lucide-react';

export function ResourceAllocationTab() {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Share className="h-5 w-5 text-green-400" />
          Resource Allocation
        </CardTitle>
        <CardDescription className="text-gray-400">
          Privacy-aware resource sharing with federated trust controls
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="text-center py-8">
          <Info className="h-10 w-10 text-gray-600 mx-auto mb-3" />
          <h3 className="text-lg font-medium text-white mb-2">Coming Soon</h3>
          <p className="text-gray-400 text-sm max-w-md mx-auto">
            Resource allocation management will be available once the IPC daemon
            exposes allocation tracking. Configure sharing limits in the Resources tab.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

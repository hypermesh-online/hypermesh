// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Share } from 'lucide-react';
import type { AssetAllocation } from '@/lib/api';

interface ResourceAllocationTabProps {
  activeAllocations: AssetAllocation[] | undefined;
}

export function ResourceAllocationTab({ activeAllocations }: ResourceAllocationTabProps) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Share className="h-5 w-5 text-green-400" />
          Resource Allocation Management
        </CardTitle>
        <CardDescription className="text-gray-400">Privacy-aware resource sharing with federated trust controls</CardDescription>
      </CardHeader>
      <CardContent>
        {activeAllocations && activeAllocations.length > 0 ? (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {activeAllocations.map((allocation) => (
              <div key={allocation.id} className="flex items-center justify-between p-4 bg-green-500/5 border border-green-500/20 rounded-lg">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <Share className="h-4 w-4 text-green-400" />
                    <span className="text-white font-medium">Allocation {allocation.id.slice(0, 8)}...</span>
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      allocation.status === 'active' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                      allocation.status === 'pending' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                      'bg-red-500/20 text-red-400 border-red-500/30'
                    )}>
                      {allocation.status}
                    </Badge>
                  </div>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                    <div>
                      <span className="text-gray-400">Resource:</span>
                      <div className="text-white">{allocation.amount} {allocation.unit}</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Duration:</span>
                      <div className="text-white">{Math.floor(allocation.duration / 3600)}h</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Requester:</span>
                      <div className="text-white font-mono">{allocation.requesterId.slice(0, 8)}...</div>
                    </div>
                    <div>
                      <span className="text-gray-400">Started:</span>
                      <div className="text-white">{new Date(allocation.startTime).toLocaleTimeString()}</div>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Button variant="ghost" size="sm" className="text-blue-400 hover:bg-blue-500/20">
                    Monitor
                  </Button>
                  <Button variant="ghost" size="sm" className="text-red-400 hover:bg-red-500/20">
                    Terminate
                  </Button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <Share className="h-12 w-12 text-gray-600 mx-auto mb-3" />
            <h3 className="text-lg font-medium text-white mb-2">No Active Allocations</h3>
            <p className="text-gray-400">Configure resource sharing to see active allocations appear here.</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

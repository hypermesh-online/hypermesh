// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { Activity, Play, Pause } from 'lucide-react';
import { getProofIcon } from './utils';
import type { ProofValidation } from './types';

interface ValidationTabProps {
  recentValidations: ProofValidation[];
  monitoringActive: boolean;
  setMonitoringActive: (active: boolean) => void;
}

export function ValidationTab({
  recentValidations,
  monitoringActive,
  setMonitoringActive
}: ValidationTabProps) {
  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-white flex items-center gap-2">
              <Activity className="h-5 w-5 text-purple-400" />
              Real-time Proof Validation
            </CardTitle>
            <CardDescription className="text-gray-400">
              Live stream of four-proof consensus validations across the network
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant={monitoringActive ? 'default' : 'outline'} className="text-xs">
              {monitoringActive ? 'LIVE' : 'PAUSED'}
            </Badge>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setMonitoringActive(!monitoringActive)}
              className="text-purple-400"
            >
              {monitoringActive ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-3 max-h-96 overflow-y-auto">
          {recentValidations.map((validation) => {
            const ProofIcon = getProofIcon(validation.type);

            return (
              <div
                key={validation.id}
                className={cn(
                  'flex items-center justify-between p-3 rounded-lg border transition-all',
                  validation.status === 'validated' ? 'bg-green-500/5 border-green-500/30' :
                  validation.status === 'validating' ? 'bg-yellow-500/5 border-yellow-500/30 animate-pulse' :
                  validation.status === 'failed' ? 'bg-red-500/5 border-red-500/30' :
                  'bg-gray-500/5 border-gray-600/30'
                )}
              >
                <div className="flex items-center gap-3">
                  <ProofIcon className={cn(
                    'h-5 w-5',
                    validation.type === 'PoSp' ? 'text-blue-400' :
                    validation.type === 'PoSt' ? 'text-green-400' :
                    validation.type === 'PoWk' ? 'text-yellow-400' :
                    'text-purple-400'
                  )} />
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-white font-medium">{validation.type} Validation</span>
                      <Badge variant="outline" className={cn(
                        'text-xs',
                        validation.status === 'validated' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                        validation.status === 'validating' ? 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30' :
                        validation.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                        'bg-gray-500/20 text-gray-400 border-gray-500/30'
                      )}>
                        {validation.status}
                      </Badge>
                    </div>
                    <div className="text-sm text-gray-400">
                      Asset: {validation.assetId.slice(0, 8)}... |
                      Block: {validation.blockId.slice(0, 8)}... |
                      Validator: {validation.validatorNode}
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-white font-mono text-sm">
                    {validation.validationTime.toFixed(0)}ms
                  </div>
                  <div className={cn(
                    'text-xs font-medium',
                    validation.confidence >= 90 ? 'text-green-400' :
                    validation.confidence >= 70 ? 'text-yellow-400' :
                    'text-red-400'
                  )}>
                    {validation.confidence.toFixed(1)}% confidence
                  </div>
                  <div className="text-xs text-gray-500">
                    {new Date(validation.timestamp).toLocaleTimeString()}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { Activity, CheckCircle, XCircle } from 'lucide-react';
import type { ProofData } from './types';

interface ProofValidationTabProps {
  proof: ProofData;
}

export function ProofValidationTab({ proof }: ProofValidationTabProps) {
  return (
    <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Activity className="h-5 w-5 text-green-400" />
          Validation Process
        </CardTitle>
        <CardDescription className="text-gray-400">
          Step-by-step validation workflow and results
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {proof.validationSteps.map((step, index) => (
            <div key={index} className="flex items-start gap-4 p-4 bg-gray-800/50 rounded-lg">
              <div className={cn(
                'flex items-center justify-center w-8 h-8 rounded-full border-2 mt-1',
                step.status === 'completed' ? 'bg-green-500 border-green-500 text-white' :
                step.status === 'failed' ? 'bg-red-500 border-red-500 text-white' :
                'bg-gray-800 border-gray-600 text-gray-400'
              )}>
                {step.status === 'completed' ? <CheckCircle className="h-4 w-4" /> :
                 step.status === 'failed' ? <XCircle className="h-4 w-4" /> :
                 <Activity className="h-4 w-4" />}
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between mb-2">
                  <h4 className="text-white font-medium">{step.step}</h4>
                  <div className="flex items-center gap-2">
                    <Badge variant="outline" className={cn(
                      'text-xs',
                      step.status === 'completed' ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                      step.status === 'failed' ? 'bg-red-500/20 text-red-400 border-red-500/30' :
                      'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
                    )}>
                      {step.status}
                    </Badge>
                    <span className="text-gray-400 text-xs font-mono">
                      {step.duration.toFixed(1)}ms
                    </span>
                  </div>
                </div>
                <p className="text-gray-400 text-sm">{step.details}</p>
                {step.status === 'completed' && (
                  <div className="mt-2">
                    <Progress value={100} className="h-1" />
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* Validation Summary */}
        <div className="mt-6 p-4 bg-green-500/10 border border-green-500/30 rounded-lg">
          <h4 className="text-green-400 font-medium mb-2">Validation Summary</h4>
          <div className="grid gap-3 md:grid-cols-3 text-sm">
            <div>
              <span className="text-gray-400">Total Steps:</span>
              <div className="text-white font-medium">{proof.validationSteps.length}</div>
            </div>
            <div>
              <span className="text-gray-400">Completed:</span>
              <div className="text-green-400 font-medium">
                {proof.validationSteps.filter(s => s.status === 'completed').length}
              </div>
            </div>
            <div>
              <span className="text-gray-400">Total Duration:</span>
              <div className="text-white font-medium">
                {proof.validationSteps.reduce((sum, step) => sum + step.duration, 0).toFixed(1)}ms
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

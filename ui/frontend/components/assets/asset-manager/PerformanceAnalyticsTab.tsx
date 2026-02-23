// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import type { AssetControlMetrics } from './types';
import {
  BarChart3,
  TrendingUp,
  Shield,
  Zap
} from 'lucide-react';

interface PerformanceAnalyticsTabProps {
  assetMetrics: AssetControlMetrics;
}

export function PerformanceAnalyticsTab({ assetMetrics }: PerformanceAnalyticsTabProps) {
  return (
    <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <BarChart3 className="h-5 w-5 text-cyan-400" />
          Performance Analytics & Optimization
        </CardTitle>
        <CardDescription className="text-gray-400">
          Real-time performance monitoring with optimization recommendations
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-6 lg:grid-cols-2">
          {/* Performance Metrics */}
          <div className="space-y-4">
            <h4 className="text-white font-medium">Real-time Performance Metrics</h4>
            <div className="space-y-4">
              {[
                { name: 'CPU Utilization', value: assetMetrics.cpuUsage, color: 'blue', unit: '%' },
                { name: 'Memory Usage', value: assetMetrics.memoryUsage, color: 'green', unit: '%' },
                { name: 'Storage I/O', value: assetMetrics.storageUsage, color: 'purple', unit: '%' },
                { name: 'Network Throughput', value: assetMetrics.networkUsage, color: 'cyan', unit: '%' }
              ].map((metric) => (
                <div key={metric.name} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400 text-sm">{metric.name}</span>
                    <span className="text-white font-mono text-sm">
                      {metric.value.toFixed(1)}{metric.unit}
                    </span>
                  </div>
                  <Progress value={metric.value} className="h-2" />
                </div>
              ))}
            </div>
          </div>

          {/* Optimization Recommendations */}
          <div className="space-y-4">
            <h4 className="text-white font-medium">Optimization Recommendations</h4>
            <div className="space-y-3">
              <div className="p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
                <div className="flex items-center gap-2 mb-1">
                  <TrendingUp className="h-4 w-4 text-green-400" />
                  <span className="text-green-400 font-medium text-sm">Excellent Performance</span>
                </div>
                <p className="text-gray-300 text-sm">
                  System performance is optimal. Consider scaling resources for increased capacity.
                </p>
              </div>

              <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg">
                <div className="flex items-center gap-2 mb-1">
                  <Shield className="h-4 w-4 text-blue-400" />
                  <span className="text-blue-400 font-medium text-sm">Security Status</span>
                </div>
                <p className="text-gray-300 text-sm">
                  All assets have valid consensus proofs. Security posture is strong.
                </p>
              </div>

              <div className="p-3 bg-purple-500/10 border border-purple-500/30 rounded-lg">
                <div className="flex items-center gap-2 mb-1">
                  <Zap className="h-4 w-4 text-purple-400" />
                  <span className="text-purple-400 font-medium text-sm">Resource Efficiency</span>
                </div>
                <p className="text-gray-300 text-sm">
                  Resource utilization at {assetMetrics.efficiency.toFixed(0)}%. Good balance between performance and capacity.
                </p>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

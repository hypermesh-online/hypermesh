// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { TrendingUp, Activity, Shield } from 'lucide-react';

export function AnalyticsTab() {
  return (
    <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <TrendingUp className="h-5 w-5 text-blue-400" />
          Performance Analytics
        </CardTitle>
        <CardDescription className="text-gray-400">Real-time asset performance monitoring and optimization recommendations</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-6 md:grid-cols-2">
          <PerformanceMetrics />
          <OptimizationRecommendations />
        </div>
      </CardContent>
    </Card>
  );
}

function PerformanceMetrics() {
  const metrics = [
    { label: 'CPU Utilization', value: 72.4 },
    { label: 'Memory Usage', value: 58.1 },
    { label: 'Storage I/O', value: 34.7 },
    { label: 'Network Throughput', value: 89.2 }
  ];

  return (
    <div className="space-y-4">
      <h4 className="text-white font-medium">Asset Performance Metrics</h4>
      <div className="space-y-3">
        {metrics.map((metric) => (
          <React.Fragment key={metric.label}>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">{metric.label}</span>
              <span className="text-white font-mono">{metric.value}%</span>
            </div>
            <Progress value={metric.value} className="h-2" />
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

function OptimizationRecommendations() {
  return (
    <div className="space-y-4">
      <h4 className="text-white font-medium">Optimization Recommendations</h4>
      <div className="space-y-3">
        <div className="p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
          <div className="flex items-center gap-2 mb-1">
            <TrendingUp className="h-4 w-4 text-green-400" />
            <span className="text-green-400 font-medium text-sm">High Efficiency</span>
          </div>
          <p className="text-gray-300 text-sm">Network assets are performing optimally. Consider increasing allocation limits.</p>
        </div>

        <div className="p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
          <div className="flex items-center gap-2 mb-1">
            <Activity className="h-4 w-4 text-yellow-400" />
            <span className="text-yellow-400 font-medium text-sm">Moderate Load</span>
          </div>
          <p className="text-gray-300 text-sm">CPU usage is moderate. Monitor for potential optimization opportunities.</p>
        </div>

        <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg">
          <div className="flex items-center gap-2 mb-1">
            <Shield className="h-4 w-4 text-blue-400" />
            <span className="text-blue-400 font-medium text-sm">Security Status</span>
          </div>
          <p className="text-gray-300 text-sm">All assets have valid consensus proofs. Security posture is good.</p>
        </div>
      </div>
    </div>
  );
}

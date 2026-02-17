// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { BarChart3 } from 'lucide-react';

export function PerformanceAnalytics() {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-white">Performance Analytics</h2>
        <Button variant="outline" className="border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20">
          <BarChart3 className="h-4 w-4 mr-2" />
          Export Report
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Peak Throughput</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">37.8 Gbps</div>
            <p className="text-xs text-gray-400">94.5% of target achieved</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Average Latency</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">12.4ms</div>
            <p className="text-xs text-gray-400">-2ms improvement</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Protocol Efficiency</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">2.3x</div>
            <p className="text-xs text-gray-400">vs traditional TCP</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Quality Score</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">94.2</div>
            <Progress value={94.2} className="mt-2 h-1" />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Protocol Comparison</CardTitle>
            <CardDescription className="text-gray-400">STOQ vs traditional protocols</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {[
                { metric: 'Connection Establishment', stoq: '1-RTT', traditional: '3-RTT', improvement: '66% faster' },
                { metric: 'Throughput', stoq: '32.4 Gbps', traditional: '14.2 Gbps', improvement: '2.3x faster' },
                { metric: 'Latency', stoq: '12ms', traditional: '28ms', improvement: '57% reduction' },
                { metric: 'Packet Loss Recovery', stoq: '1ms', traditional: '15ms', improvement: '93% faster' },
                { metric: 'CPU Overhead', stoq: '2.1%', traditional: '8.7%', improvement: '76% reduction' },
              ].map((comparison, i) => (
                <div key={i} className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                  <div className="flex justify-between items-center mb-2">
                    <h4 className="font-medium text-white">{comparison.metric}</h4>
                    <Badge variant="default" className="bg-green-500/20 text-green-400 border-green-500/30">
                      {comparison.improvement}
                    </Badge>
                  </div>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <p className="text-gray-400">STOQ</p>
                      <p className="text-cyan-400 font-medium">{comparison.stoq}</p>
                    </div>
                    <div>
                      <p className="text-gray-400">Traditional</p>
                      <p className="text-gray-500 font-medium">{comparison.traditional}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Network Bottlenecks</CardTitle>
            <CardDescription className="text-gray-400">Identified performance constraints</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {[
                {
                  location: 'Asia-Pacific Gateway',
                  severity: 'medium',
                  impact: '15% throughput reduction',
                  cause: 'IPv6 routing suboptimal',
                  recommendation: 'Deploy additional relay nodes'
                },
                {
                  location: 'Trans-Atlantic Link',
                  severity: 'low',
                  impact: '8% latency increase',
                  cause: 'Legacy infrastructure',
                  recommendation: 'Upgrade to STOQ v2.1'
                },
                {
                  location: 'EU Central Hub',
                  severity: 'low',
                  impact: '3% packet loss',
                  cause: 'Network congestion',
                  recommendation: 'Load balancing optimization'
                }
              ].map((bottleneck, i) => (
                <div key={i} className="p-4 border border-cyan-500/20 rounded-lg bg-cyan-500/5">
                  <div className="flex items-center justify-between mb-2">
                    <h4 className="font-medium text-white">{bottleneck.location}</h4>
                    <Badge variant={
                      bottleneck.severity === 'high' ? 'destructive' :
                      bottleneck.severity === 'medium' ? 'secondary' : 'default'
                    } className={
                      bottleneck.severity === 'low' ? 'bg-green-500/20 text-green-400 border-green-500/30' : ''
                    }>
                      {bottleneck.severity} priority
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-400 mb-1">
                    Impact: {bottleneck.impact}
                  </p>
                  <p className="text-sm text-gray-400 mb-2">
                    Cause: {bottleneck.cause}
                  </p>
                  <p className="text-sm font-medium text-cyan-400">
                    → {bottleneck.recommendation}
                  </p>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
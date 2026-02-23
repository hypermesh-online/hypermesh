// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Lock, Shield, Database } from 'lucide-react';
import type { EcosystemMetrics } from './types';

interface DetailedMetricsTabsProps {
  metrics: EcosystemMetrics;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function DetailedMetricsTabs({ metrics, activeTab, onTabChange }: DetailedMetricsTabsProps) {
  return (
    <Tabs value={activeTab} onValueChange={onTabChange} className="w-full">
      <TabsList className="grid w-full grid-cols-4 bg-black/20">
        <TabsTrigger value="overview" className="text-white">Performance</TabsTrigger>
        <TabsTrigger value="consensus" className="text-white">Consensus</TabsTrigger>
        <TabsTrigger value="security" className="text-white">Security</TabsTrigger>
        <TabsTrigger value="economics" className="text-white">Economics</TabsTrigger>
      </TabsList>

      <TabsContent value="overview" className="space-y-6 mt-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white">STOQ Protocol Performance</CardTitle>
              <CardDescription className="text-gray-400">
                Network throughput and connectivity metrics
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <div className="flex justify-between text-sm mb-2">
                  <span className="text-gray-400">Current Throughput</span>
                  <span className="text-white font-medium">{metrics.networkThroughput.toFixed(2)} Gbps</span>
                </div>
                <Progress value={(metrics.networkThroughput / 40) * 100} className="h-2" />
                <div className="flex justify-between text-xs text-gray-400 mt-1">
                  <span>Current</span>
                  <span>Target: 40 Gbps</span>
                </div>
              </div>
              <div className="text-sm text-gray-400">
                Performance bottleneck identified in QUIC implementation.
                Optimization in progress for Phase 1 deployment.
              </div>
            </CardContent>
          </Card>

          <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white">Asset Distribution</CardTitle>
              <CardDescription className="text-gray-400">
                HyperMesh asset allocation and utilization
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-400">Total Assets:</span>
                  <div className="text-lg font-bold text-blue-400">{metrics.totalAssets.toLocaleString()}</div>
                </div>
                <div>
                  <span className="text-gray-400">Active Nodes:</span>
                  <div className="text-lg font-bold text-green-400">156</div>
                </div>
                <div>
                  <span className="text-gray-400">Utilization:</span>
                  <div className="text-lg font-bold text-purple-400">67%</div>
                </div>
                <div>
                  <span className="text-gray-400">Avg. Sharing:</span>
                  <div className="text-lg font-bold text-yellow-400">8 assets/node</div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </TabsContent>

      <TabsContent value="consensus" className="space-y-6 mt-6">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Four-Proof Consensus</CardTitle>
            <CardDescription className="text-gray-400">
              Proof of State protocol validation status
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {[
                { name: 'PoSpace', color: 'text-blue-400', coverage: '98.5%' },
                { name: 'PoStake', color: 'text-green-400', coverage: '96.2%' },
                { name: 'PoWork', color: 'text-purple-400', coverage: '99.1%' },
                { name: 'PoTime', color: 'text-yellow-400', coverage: '97.8%' }
              ].map((proof) => (
                <div key={proof.name} className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">{proof.name}</span>
                    <span className={`${proof.color} font-medium`}>Active</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Coverage:</span>
                    <span className="text-white">{proof.coverage}</span>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="security" className="space-y-6 mt-6">
        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Quantum Security Status</CardTitle>
            <CardDescription className="text-gray-400">
              Post-quantum cryptography and security metrics
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="text-center p-4 border border-purple-500/20 rounded-lg bg-purple-500/5">
                <Lock className="h-8 w-8 text-purple-400 mx-auto mb-2" />
                <div className="text-lg font-bold text-purple-400">{metrics.quantumConnections}</div>
                <div className="text-sm text-gray-400">Quantum-Safe Connections</div>
              </div>
              <div className="text-center p-4 border border-green-500/20 rounded-lg bg-green-500/5">
                <Shield className="h-8 w-8 text-green-400 mx-auto mb-2" />
                <div className="text-lg font-bold text-green-400">{metrics.activeCertificates}</div>
                <div className="text-sm text-gray-400">FALCON-1024 Certificates</div>
              </div>
              <div className="text-center p-4 border border-blue-500/20 rounded-lg bg-blue-500/5">
                <Database className="h-8 w-8 text-blue-400 mx-auto mb-2" />
                <div className="text-lg font-bold text-blue-400">100%</div>
                <div className="text-sm text-gray-400">Quantum Encryption</div>
              </div>
            </div>
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="economics" className="space-y-6 mt-6">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Caesar Economic System</CardTitle>
            <CardDescription className="text-gray-400">
              Economic incentives and reward distribution
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="space-y-2">
                <div className="text-2xl font-bold text-green-400">{metrics.economicRewards.toFixed(2)}</div>
                <div className="text-sm text-gray-400">Total CAESAR Rewards</div>
                <div className="text-xs text-green-400">+12.8% this month</div>
              </div>
              <div className="space-y-2">
                <div className="text-2xl font-bold text-blue-400">34%</div>
                <div className="text-sm text-gray-400">Network Staking Rate</div>
                <div className="text-xs text-blue-400">+2.1% this week</div>
              </div>
              <div className="space-y-2">
                <div className="text-2xl font-bold text-purple-400">$2.4M</div>
                <div className="text-sm text-gray-400">Total Network Value</div>
                <div className="text-xs text-purple-400">+18.5% this quarter</div>
              </div>
            </div>
          </CardContent>
        </Card>
      </TabsContent>
    </Tabs>
  );
}

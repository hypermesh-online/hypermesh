// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import { Network } from 'lucide-react';

export function TunnelManagement() {
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-white">P2P Tunnel Management</h2>
        <Button className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black">
          <Network className="h-4 w-4 mr-2" />
          Create Tunnel
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Active Tunnels</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">78</div>
            <p className="text-xs text-gray-400">+5 new today</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Total Throughput</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">32.4 Gbps</div>
            <p className="text-xs text-gray-400">Across all tunnels</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Encryption Status</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">100%</div>
            <p className="text-xs text-gray-400">All tunnels secured</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-2">
            <CardTitle className="text-base text-white">Failover Ready</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">45</div>
            <p className="text-xs text-gray-400">Backup routes available</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="active" className="space-y-4">
        <TabsList className="bg-black/40 border-gray-700">
          <TabsTrigger value="active" className="data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-400">Active Tunnels</TabsTrigger>
          <TabsTrigger value="discovery" className="data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-400">Tunnel Discovery</TabsTrigger>
          <TabsTrigger value="security" className="data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-400">Security & Encryption</TabsTrigger>
        </TabsList>

        <TabsContent value="active">
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white">Active P2P Tunnels</CardTitle>
              <CardDescription className="text-gray-400">Currently established tunnel connections</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[...Array(8)].map((_, i) => (
                  <div key={i} className="flex items-center justify-between p-4 border border-cyan-500/20 rounded-lg bg-cyan-500/5">
                    <div>
                      <h4 className="font-medium text-white">TUN-{String(i + 1).padStart(3, '0')}</h4>
                      <p className="text-sm text-gray-400 font-mono">
                        {['us-west-1', 'eu-central-1', 'ap-southeast-1'][i % 3]} → 
                        {['eu-central-1', 'ap-southeast-1', 'us-east-1'][i % 3]}
                      </p>
                      <div className="flex gap-2 mt-1">
                        <Badge variant="outline" className="border-cyan-500/50 text-cyan-400">
                          {(Math.random() * 10 + 5).toFixed(1)} Gbps
                        </Badge>
                        <Badge variant="outline" className="border-cyan-500/50 text-cyan-400">
                          {Math.floor(Math.random() * 200 + 50)}ms
                        </Badge>
                        <Badge variant="default" className="bg-green-500/20 text-green-400 border-green-500/30">
                          STOQ/TLS 1.3
                        </Badge>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button variant="outline" size="sm" className="border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20">
                        Monitor
                      </Button>
                      <Button variant="outline" size="sm" className="border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20">
                        Configure
                      </Button>
                      <Badge variant={Math.random() > 0.8 ? 'destructive' : 'default'}
                             className={Math.random() > 0.8 ? '' : 'bg-green-500/20 text-green-400 border-green-500/30'}>
                        {Math.random() > 0.8 ? 'Degraded' : 'Healthy'}
                      </Badge>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="discovery">
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white">Tunnel Discovery</CardTitle>
              <CardDescription className="text-gray-400">Discover optimal tunnel connections using STOQ protocol</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[
                  { src: 'us-west-2', dst: 'ap-northeast-1', quality: 92, latency: '135ms', protocol: 'STOQ v2.1' },
                  { src: 'eu-west-1', dst: 'us-central-1', quality: 88, latency: '78ms', protocol: 'STOQ v2.1' },
                  { src: 'ap-south-1', dst: 'eu-central-1', quality: 75, latency: '185ms', protocol: 'STOQ v2.0' },
                  { src: 'sa-east-1', dst: 'us-east-1', quality: 91, latency: '95ms', protocol: 'STOQ v2.1' },
                ].map((discovery, i) => (
                  <div key={i} className="flex items-center justify-between p-4 border border-cyan-500/20 rounded-lg bg-cyan-500/5">
                    <div>
                      <p className="font-medium text-white font-mono">{discovery.src} → {discovery.dst}</p>
                      <p className="text-sm text-gray-400">
                        Latency: {discovery.latency} | Quality: {discovery.quality}% | {discovery.protocol}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant={
                        discovery.quality >= 90 ? 'default' :
                        discovery.quality >= 80 ? 'secondary' : 'outline'
                      } className={
                        discovery.quality >= 90 ? 'bg-green-500/20 text-green-400 border-green-500/30' :
                        discovery.quality >= 80 ? 'bg-cyan-500/20 text-cyan-400 border-cyan-500/30' : ''
                      }>
                        {discovery.quality >= 90 ? 'Excellent' :
                         discovery.quality >= 80 ? 'Good' : 'Fair'}
                      </Badge>
                      <Button size="sm" className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black">
                        Establish
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="security">
          <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
            <CardHeader>
              <CardTitle className="text-white">Security & Encryption</CardTitle>
              <CardDescription className="text-gray-400">STOQ tunnel security configuration and status</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid gap-6 lg:grid-cols-2">
                <div className="space-y-4">
                  <h4 className="font-medium text-white">Encryption Standards</h4>
                  {[
                    { protocol: 'STOQ + TLS 1.3', tunnels: 45, percentage: 58 },
                    { protocol: 'STOQ + ChaCha20', tunnels: 25, percentage: 32 },
                    { protocol: 'Legacy TLS 1.2', tunnels: 8, percentage: 10 },
                  ].map((protocol) => (
                    <div key={protocol.protocol} className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                      <div className="flex justify-between mb-1">
                        <span className="text-sm font-medium text-white">{protocol.protocol}</span>
                        <span className="text-sm text-cyan-400">{protocol.tunnels} tunnels</span>
                      </div>
                      <Progress value={protocol.percentage} className="h-1" />
                    </div>
                  ))}
                </div>

                <div className="space-y-4">
                  <h4 className="font-medium text-white">Security Events</h4>
                  {[
                    { type: 'info', message: 'STOQ token rotation completed', time: '5 min ago' },
                    { type: 'success', message: 'All tunnels validated successfully', time: '12 min ago' },
                    { type: 'info', message: 'Quantum-resistant upgrade available', time: '1 hour ago' },
                    { type: 'success', message: 'Security audit passed', time: '2 hours ago' },
                  ].map((event, i) => (
                    <div key={i} className="flex items-center gap-3 p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                      <div className={cn(
                        "w-2 h-2 rounded-full",
                        event.type === 'success' ? 'bg-green-400' :
                        event.type === 'warning' ? 'bg-yellow-400' :
                        event.type === 'info' ? 'bg-cyan-400' : 'bg-gray-400'
                      )} />
                      <div className="flex-1">
                        <p className="text-sm text-white">{event.message}</p>
                        <p className="text-xs text-gray-400">{event.time}</p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  Shield, 
  Network, 
  HardDrive, 
  Coins, 
  Activity, 
  RefreshCw,
  TrendingUp,
  TrendingDown,
  Minus,
  CheckCircle,
  AlertTriangle,
  XCircle,
  Clock,
  Zap,
  Lock,
  Globe,
  Database
} from 'lucide-react';
import { cn } from '@/lib/utils';

export interface EcosystemMetrics {
  totalAssets: number;
  activeCertificates: number;
  networkThroughput: number;
  consensusBlocks: number;
  quantumConnections: number;
  economicRewards: number;
}

export interface SystemStatus {
  name: string;
  status: 'online' | 'warning' | 'offline' | 'maintenance';
  uptime: number;
  lastChecked: string;
  metrics: Record<string, string>;
  description?: string;
}

export interface MetricTrend {
  value: number;
  change: number;
  trend: 'up' | 'down' | 'stable';
  period: string;
}

interface EcosystemMetricsDashboardProps {
  metrics?: EcosystemMetrics;
  systemStatuses?: SystemStatus[];
  onRefresh?: () => void;
  autoRefresh?: boolean;
  refreshInterval?: number;
  loading?: boolean;
  className?: string;
}

const defaultMetrics: EcosystemMetrics = {
  totalAssets: 1247,
  activeCertificates: 892,
  networkThroughput: 2.95,
  consensusBlocks: 15234,
  quantumConnections: 445,
  economicRewards: 12847.32
};

const defaultSystemStatuses: SystemStatus[] = [
  {
    name: 'TrustChain CA',
    status: 'online',
    uptime: 2592000000, // 30 days
    lastChecked: new Date().toISOString(),
    metrics: {
      'Certificates Issued': '892',
      'Root CAs': '3',
      'Revoked Certs': '12'
    },
    description: 'Certificate Authority and trust management system'
  },
  {
    name: 'STOQ Protocol',
    status: 'warning',
    uptime: 2505600000, // 29 days
    lastChecked: new Date().toISOString(),
    metrics: {
      'Current Throughput': '2.95 Gbps',
      'Active Connections': '445',
      'Quantum Safe': '100%'
    },
    description: 'High-performance transport protocol with quantum security'
  },
  {
    name: 'HyperMesh Network',
    status: 'online',
    uptime: 2419200000, // 28 days
    lastChecked: new Date().toISOString(),
    metrics: {
      'Total Assets': '1,247',
      'Active Nodes': '156',
      'Asset Utilization': '67%'
    },
    description: 'Distributed asset sharing and resource coordination'
  },
  {
    name: 'Caesar Economics',
    status: 'online',
    uptime: 2332800000, // 27 days
    lastChecked: new Date().toISOString(),
    metrics: {
      'Total Rewards': '12,847.32 CAESAR',
      'Staking Rate': '34%',
      'Network Value': '$2.4M'
    },
    description: 'Economic incentive and reward distribution system'
  },
  {
    name: 'Four-Proof Consensus',
    status: 'online',
    uptime: 2246400000, // 26 days
    lastChecked: new Date().toISOString(),
    metrics: {
      'Block Height': '15,234',
      'Validators': '67',
      'Finality Time': '2.3s'
    },
    description: 'Proof of State validation with PoSp+PoSt+PoWk+PoTm authentication'
  }
];

const metricTrends: Record<string, MetricTrend> = {
  totalAssets: { value: 1247, change: 2.4, trend: 'up', period: 'from last week' },
  activeCertificates: { value: 892, change: 1.2, trend: 'up', period: 'from last week' },
  networkThroughput: { value: 2.95, change: -0.3, trend: 'down', period: 'from target' },
  economicRewards: { value: 12847.32, change: 12.8, trend: 'up', period: 'this month' }
};

export function EcosystemMetricsDashboard({
  metrics = defaultMetrics,
  systemStatuses = defaultSystemStatuses,
  onRefresh,
  autoRefresh = true,
  refreshInterval = 30000,
  loading = false,
  className
}: EcosystemMetricsDashboardProps) {
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [activeTab, setActiveTab] = useState('overview');

  const handleRefresh = useCallback(() => {
    setLastRefresh(new Date());
    onRefresh?.();
  }, [onRefresh]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      handleRefresh();
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, handleRefresh]);

  const getStatusIcon = (status: SystemStatus['status']) => {
    switch (status) {
      case 'online':
        return <CheckCircle className="h-4 w-4 text-green-400" />;
      case 'warning':
        return <AlertTriangle className="h-4 w-4 text-yellow-400" />;
      case 'offline':
        return <XCircle className="h-4 w-4 text-red-400" />;
      case 'maintenance':
        return <Clock className="h-4 w-4 text-blue-400" />;
      default:
        return <CheckCircle className="h-4 w-4 text-gray-400" />;
    }
  };

  const getStatusColor = (status: SystemStatus['status']) => {
    switch (status) {
      case 'online':
        return 'text-green-400 bg-green-500/20 border-green-500/30';
      case 'warning':
        return 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30';
      case 'offline':
        return 'text-red-400 bg-red-500/20 border-red-500/30';
      case 'maintenance':
        return 'text-blue-400 bg-blue-500/20 border-blue-500/30';
      default:
        return 'text-gray-400 bg-gray-500/20 border-gray-500/30';
    }
  };

  const getTrendIcon = (trend: MetricTrend['trend']) => {
    switch (trend) {
      case 'up':
        return <TrendingUp className="h-3 w-3 text-green-400" />;
      case 'down':
        return <TrendingDown className="h-3 w-3 text-red-400" />;
      case 'stable':
        return <Minus className="h-3 w-3 text-gray-400" />;
    }
  };

  const formatUptime = (uptimeMs: number) => {
    const days = Math.floor(uptimeMs / (1000 * 60 * 60 * 24));
    const hours = Math.floor((uptimeMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    return `${days}d ${hours}h`;
  };

  const getUptimePercentage = (uptimeMs: number) => {
    const days = uptimeMs / (1000 * 60 * 60 * 24);
    const totalPossibleDays = 30; // Assuming 30 day measurement period
    return Math.min((days / totalPossibleDays) * 100, 100);
  };

  const overallHealthScore = () => {
    const onlineCount = systemStatuses.filter(s => s.status === 'online').length;
    const totalCount = systemStatuses.length;
    return (onlineCount / totalCount) * 100;
  };

  return (
    <div className={cn("space-y-6", className)}>
      {/* Dashboard Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Web3 Ecosystem Dashboard</h1>
          <p className="text-gray-400 mt-2">
            Quantum-secure, user-sovereign internet infrastructure
          </p>
        </div>
        <div className="flex items-center space-x-4">
          <div className="text-sm text-gray-400">
            Last updated: {lastRefresh.toLocaleTimeString()}
          </div>
          <Button 
            variant="outline" 
            size="sm" 
            onClick={handleRefresh} 
            disabled={loading}
            className="border-green-500/30 text-green-400 hover:bg-green-500/20"
          >
            <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Key Metrics Grid */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Assets</CardTitle>
            <HardDrive className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-400">{metrics.totalAssets.toLocaleString()}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.totalAssets.trend)}
              <span className={cn(
                metricTrends.totalAssets.trend === 'up' ? 'text-green-400' :
                metricTrends.totalAssets.trend === 'down' ? 'text-red-400' : 'text-gray-400'
              )}>
                {metricTrends.totalAssets.change > 0 ? '+' : ''}{metricTrends.totalAssets.change}%
              </span>
              <span>{metricTrends.totalAssets.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Active Certificates</CardTitle>
            <Shield className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-400">{metrics.activeCertificates.toLocaleString()}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.activeCertificates.trend)}
              <span className="text-green-400">
                +{metricTrends.activeCertificates.change}%
              </span>
              <span>{metricTrends.activeCertificates.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Network Throughput</CardTitle>
            <Network className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-yellow-400">{metrics.networkThroughput.toFixed(2)} Gbps</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.networkThroughput.trend)}
              <span className="text-red-400">
                {metricTrends.networkThroughput.change}%
              </span>
              <span>{metricTrends.networkThroughput.period}</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Economic Rewards</CardTitle>
            <Coins className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-400">{metrics.economicRewards.toFixed(2)}</div>
            <div className="flex items-center space-x-1 text-xs text-gray-400">
              {getTrendIcon(metricTrends.economicRewards.trend)}
              <span className="text-green-400">
                +{metricTrends.economicRewards.change}%
              </span>
              <span>{metricTrends.economicRewards.period}</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* System Health Overview */}
      <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-white flex items-center space-x-2">
                <Activity className="h-5 w-5 text-green-400" />
                <span>System Health Overview</span>
              </CardTitle>
              <CardDescription className="text-gray-400">
                Real-time status of all Web3 ecosystem components
              </CardDescription>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-green-400">{overallHealthScore().toFixed(1)}%</div>
              <div className="text-sm text-gray-400">Overall Health</div>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {systemStatuses.map((system) => (
              <div
                key={system.name}
                className={cn(
                  "border rounded-lg p-4 transition-all duration-300",
                  system.status === 'online' ? 'bg-green-500/5 border-green-500/30' :
                  system.status === 'warning' ? 'bg-yellow-500/5 border-yellow-500/30' :
                  system.status === 'offline' ? 'bg-red-500/5 border-red-500/30' :
                  'bg-blue-500/5 border-blue-500/30'
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <h4 className="font-medium text-white">{system.name}</h4>
                  <div className="flex items-center space-x-2">
                    {getStatusIcon(system.status)}
                    <Badge className={getStatusColor(system.status)}>
                      {system.status.toUpperCase()}
                    </Badge>
                  </div>
                </div>

                {system.description && (
                  <p className="text-xs text-gray-400 mb-3">{system.description}</p>
                )}

                <div className="space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Uptime:</span>
                    <span className="text-white">{formatUptime(system.uptime)}</span>
                  </div>
                  <Progress 
                    value={getUptimePercentage(system.uptime)} 
                    className="h-1" 
                    indicatorClassName="bg-green-400"
                  />
                  <div className="flex justify-between">
                    <span className="text-gray-400">Last Check:</span>
                    <span className="text-white">{new Date(system.lastChecked).toLocaleTimeString()}</span>
                  </div>
                </div>

                <div className="mt-3 pt-3 border-t border-current/20">
                  <div className="grid grid-cols-1 gap-1 text-xs">
                    {Object.entries(system.metrics).slice(0, 2).map(([key, value]) => (
                      <div key={key} className="flex justify-between">
                        <span className="text-gray-400">{key}:</span>
                        <span className="text-white font-mono">{value}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Detailed Metrics Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
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
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">PoSpace</span>
                    <span className="text-blue-400 font-medium">✓ Active</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Coverage:</span>
                    <span className="text-white">98.5%</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">PoStake</span>
                    <span className="text-green-400 font-medium">✓ Active</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Coverage:</span>
                    <span className="text-white">96.2%</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">PoWork</span>
                    <span className="text-purple-400 font-medium">✓ Active</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Coverage:</span>
                    <span className="text-white">99.1%</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">PoTime</span>
                    <span className="text-yellow-400 font-medium">✓ Active</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Coverage:</span>
                    <span className="text-white">97.8%</span>
                  </div>
                </div>
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
    </div>
  );
}
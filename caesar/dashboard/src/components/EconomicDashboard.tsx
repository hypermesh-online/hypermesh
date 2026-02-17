// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useCallback } from 'react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell
} from 'recharts';
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  Button,
  Alert,
  AlertDescription,
  Badge,
  Progress,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Switch
} from '../ui';
import { useEconomicEngine } from '../hooks/useEconomicEngine';
import { formatCurrency, formatPercentage, formatTimestamp } from '../utils/formatting';

interface EconomicMetrics {
  overallHealth: number;
  priceStability: number;
  liquidityHealth: number;
  participationRate: number;
  reserveRatio: number;
  demurrageEfficiency: number;
  antiSpeculationEffectiveness: number;
  systemStress: number;
  timestamp: number;
}

interface DemurrageData {
  totalCollected: string;
  averageRate: number;
  accountsProcessed: number;
  exemptAccounts: number;
}

interface AntiSpeculationData {
  totalPenalties: string;
  flaggedAccounts: number;
  circuitBreakerActivations: number;
  averageRiskScore: number;
}

interface StabilityPoolData {
  totalBalance: string;
  penaltyFunds: string;
  demurrageFunds: string;
  reserveFunds: string;
  emergencyFunds: string;
}

const COLORS = ['#0088FE', '#00C49F', '#FFBB28', '#FF8042', '#8884d8'];

export const EconomicDashboard: React.FC = () => {
  const {
    economicMetrics,
    demurrageData,
    antiSpeculationData,
    stabilityPoolData,
    economicParameters,
    isEmergencyMode,
    healthHistory,
    refreshData,
    updateParameters,
    activateEmergencyMode,
    deactivateEmergencyMode,
    applyDemurrage,
    executeIntervention,
    isLoading,
    error
  } = useEconomicEngine();

  const [selectedTimeRange, setSelectedTimeRange] = useState('24h');
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(30); // seconds
  const [showParameterDialog, setShowParameterDialog] = useState(false);
  const [showEmergencyDialog, setShowEmergencyDialog] = useState(false);
  const [parameterUpdates, setParameterUpdates] = useState({});

  // Auto-refresh functionality
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      refreshData();
    }, refreshInterval * 1000);

    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, refreshData]);

  const handleParameterUpdate = useCallback(async () => {
    try {
      await updateParameters(parameterUpdates);
      setShowParameterDialog(false);
      setParameterUpdates({});
    } catch (error) {
      console.error('Failed to update parameters:', error);
    }
  }, [parameterUpdates, updateParameters]);

  const handleEmergencyAction = useCallback(async (action: 'activate' | 'deactivate', reason?: string) => {
    try {
      if (action === 'activate' && reason) {
        await activateEmergencyMode(reason);
      } else if (action === 'deactivate') {
        await deactivateEmergencyMode();
      }
      setShowEmergencyDialog(false);
    } catch (error) {
      console.error('Failed to execute emergency action:', error);
    }
  }, [activateEmergencyMode, deactivateEmergencyMode]);

  const getHealthColor = (score: number): string => {
    if (score >= 800) return 'text-green-600';
    if (score >= 600) return 'text-yellow-600';
    if (score >= 400) return 'text-orange-600';
    return 'text-red-600';
  };

  const getHealthBadgeVariant = (score: number) => {
    if (score >= 800) return 'success';
    if (score >= 600) return 'warning';
    return 'destructive';
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-lg">Loading economic data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <Alert className="m-4">
        <AlertDescription>
          Error loading economic data: {error.message}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="p-6 space-y-6 bg-gray-50 min-h-screen">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold text-gray-900">Gateway Coin Economic Dashboard</h1>
        
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Switch
              checked={autoRefresh}
              onCheckedChange={setAutoRefresh}
            />
            <span className="text-sm">Auto-refresh</span>
          </div>
          
          <Select value={refreshInterval.toString()} onValueChange={(v) => setRefreshInterval(parseInt(v))}>
            <SelectTrigger className="w-20">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="10">10s</SelectItem>
              <SelectItem value="30">30s</SelectItem>
              <SelectItem value="60">1m</SelectItem>
              <SelectItem value="300">5m</SelectItem>
            </SelectContent>
          </Select>

          <Button onClick={refreshData} disabled={isLoading}>
            Refresh
          </Button>

          {isEmergencyMode && (
            <Badge variant="destructive" className="animate-pulse">
              EMERGENCY MODE ACTIVE
            </Badge>
          )}
        </div>
      </div>

      {/* Emergency Alert */}
      {isEmergencyMode && (
        <Alert className="border-red-500 bg-red-50">
          <AlertDescription className="flex justify-between items-center">
            <span>Emergency mode is currently active. All demurrage and penalties are paused.</span>
            <Dialog open={showEmergencyDialog} onOpenChange={setShowEmergencyDialog}>
              <DialogTrigger asChild>
                <Button variant="outline" size="sm">
                  Manage Emergency
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Emergency Mode Control</DialogTitle>
                </DialogHeader>
                <div className="space-y-4">
                  <p>Emergency mode is currently active. You can deactivate it to resume normal operations.</p>
                  <Button 
                    onClick={() => handleEmergencyAction('deactivate')}
                    className="w-full"
                    variant="destructive"
                  >
                    Deactivate Emergency Mode
                  </Button>
                </div>
              </DialogContent>
            </Dialog>
          </AlertDescription>
        </Alert>
      )}

      {/* Key Metrics Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardContent className="p-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-600">Overall Health</p>
                <p className={`text-2xl font-bold ${getHealthColor(economicMetrics.overallHealth / 10)}`}>
                  {formatPercentage(economicMetrics.overallHealth / 10)}
                </p>
              </div>
              <Badge variant={getHealthBadgeVariant(economicMetrics.overallHealth / 10)}>
                {economicMetrics.overallHealth >= 800 ? 'Excellent' :
                 economicMetrics.overallHealth >= 600 ? 'Good' :
                 economicMetrics.overallHealth >= 400 ? 'Fair' : 'Poor'}
              </Badge>
            </div>
            <Progress value={economicMetrics.overallHealth / 10} className="mt-2" />
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-6">
            <div>
              <p className="text-sm font-medium text-gray-600">Price Stability</p>
              <p className={`text-2xl font-bold ${getHealthColor(economicMetrics.priceStability / 10)}`}>
                {formatPercentage(economicMetrics.priceStability / 10)}
              </p>
              <Progress value={economicMetrics.priceStability / 10} className="mt-2" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-6">
            <div>
              <p className="text-sm font-medium text-gray-600">Reserve Ratio</p>
              <p className={`text-2xl font-bold ${getHealthColor(economicMetrics.reserveRatio / 10)}`}>
                {formatPercentage(economicMetrics.reserveRatio / 100)}
              </p>
              <Progress value={Math.min(economicMetrics.reserveRatio / 10, 100)} className="mt-2" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="p-6">
            <div>
              <p className="text-sm font-medium text-gray-600">System Stress</p>
              <p className={`text-2xl font-bold ${economicMetrics.systemStress <= 200 ? 'text-green-600' : 
                economicMetrics.systemStress <= 500 ? 'text-yellow-600' : 'text-red-600'}`}>
                {formatPercentage(economicMetrics.systemStress / 10)}
              </p>
              <Progress 
                value={economicMetrics.systemStress / 10} 
                className="mt-2"
                style={{ '--progress-background': economicMetrics.systemStress <= 200 ? '#10b981' : 
                  economicMetrics.systemStress <= 500 ? '#f59e0b' : '#ef4444' } as React.CSSProperties}
              />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main Dashboard Tabs */}
      <Tabs defaultValue="overview" className="w-full">
        <TabsList className="grid w-full grid-cols-6">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="demurrage">Demurrage</TabsTrigger>
          <TabsTrigger value="speculation">Anti-Speculation</TabsTrigger>
          <TabsTrigger value="stability">Stability Pool</TabsTrigger>
          <TabsTrigger value="parameters">Parameters</TabsTrigger>
          <TabsTrigger value="controls">Controls</TabsTrigger>
        </TabsList>

        {/* Overview Tab */}
        <TabsContent value="overview" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Health Metrics Over Time */}
            <Card className="col-span-2">
              <CardHeader>
                <CardTitle>System Health Over Time</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <LineChart data={healthHistory}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis 
                      dataKey="timestamp" 
                      tickFormatter={(value) => new Date(value * 1000).toLocaleTimeString()}
                    />
                    <YAxis domain={[0, 100]} />
                    <Tooltip 
                      labelFormatter={(value) => formatTimestamp(value)}
                      formatter={(value: any, name: string) => [formatPercentage(value), name]}
                    />
                    <Legend />
                    <Line type="monotone" dataKey="overallHealth" stroke="#8884d8" strokeWidth={2} />
                    <Line type="monotone" dataKey="priceStability" stroke="#82ca9d" strokeWidth={2} />
                    <Line type="monotone" dataKey="liquidityHealth" stroke="#ffc658" strokeWidth={2} />
                  </LineChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            {/* System Metrics Breakdown */}
            <Card>
              <CardHeader>
                <CardTitle>Current System Metrics</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={250}>
                  <PieChart>
                    <Pie
                      data={[
                        { name: 'Price Stability', value: economicMetrics.priceStability / 10 },
                        { name: 'Liquidity Health', value: economicMetrics.liquidityHealth / 10 },
                        { name: 'Participation Rate', value: economicMetrics.participationRate / 10 },
                        { name: 'Demurrage Efficiency', value: economicMetrics.demurrageEfficiency / 10 },
                      ]}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                      outerRadius={80}
                      fill="#8884d8"
                      dataKey="value"
                    >
                      {[0, 1, 2, 3].map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value: any) => formatPercentage(value)} />
                  </PieChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            {/* Recent Activity */}
            <Card>
              <CardHeader>
                <CardTitle>Recent System Activity</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-600">Last Parameter Update</span>
                    <span className="text-sm font-medium">
                      {formatTimestamp(economicParameters.lastUpdate)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-600">Demurrage Collections (24h)</span>
                    <span className="text-sm font-medium">
                      {formatCurrency(demurrageData.totalCollected)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-600">Speculation Penalties (24h)</span>
                    <span className="text-sm font-medium">
                      {formatCurrency(antiSpeculationData.totalPenalties)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-gray-600">Circuit Breaker Activations</span>
                    <Badge variant="outline">
                      {antiSpeculationData.circuitBreakerActivations}
                    </Badge>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Demurrage Tab */}
        <TabsContent value="demurrage" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <Card>
              <CardHeader>
                <CardTitle>Demurrage Overview</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label className="text-sm text-gray-600">Total Collected</Label>
                  <p className="text-2xl font-bold">{formatCurrency(demurrageData.totalCollected)}</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Average Rate</Label>
                  <p className="text-xl font-semibold">{formatPercentage(demurrageData.averageRate)}</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Accounts Processed</Label>
                  <p className="text-xl font-semibold">{demurrageData.accountsProcessed.toLocaleString()}</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Exempt Accounts</Label>
                  <p className="text-xl font-semibold">{demurrageData.exemptAccounts.toLocaleString()}</p>
                </div>
              </CardContent>
            </Card>

            <Card className="col-span-2">
              <CardHeader>
                <CardTitle>Demurrage Rate History</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <AreaChart data={healthHistory}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis 
                      dataKey="timestamp" 
                      tickFormatter={(value) => new Date(value * 1000).toLocaleDateString()}
                    />
                    <YAxis />
                    <Tooltip 
                      labelFormatter={(value) => formatTimestamp(value)}
                      formatter={(value: any) => [formatPercentage(value), 'Demurrage Rate']}
                    />
                    <Area 
                      type="monotone" 
                      dataKey="demurrageRate" 
                      stroke="#8884d8" 
                      fillOpacity={0.6} 
                      fill="#8884d8" 
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Anti-Speculation Tab */}
        <TabsContent value="speculation" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <Card>
              <CardHeader>
                <CardTitle>Anti-Speculation Metrics</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label className="text-sm text-gray-600">Total Penalties</Label>
                    <p className="text-xl font-bold">{formatCurrency(antiSpeculationData.totalPenalties)}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Flagged Accounts</Label>
                    <p className="text-xl font-bold text-red-600">{antiSpeculationData.flaggedAccounts}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Circuit Breaker</Label>
                    <p className="text-xl font-bold">{antiSpeculationData.circuitBreakerActivations}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Avg Risk Score</Label>
                    <p className={`text-xl font-bold ${antiSpeculationData.averageRiskScore <= 300 ? 'text-green-600' :
                      antiSpeculationData.averageRiskScore <= 600 ? 'text-yellow-600' : 'text-red-600'}`}>
                      {antiSpeculationData.averageRiskScore}/1000
                    </p>
                  </div>
                </div>
                <Progress 
                  value={antiSpeculationData.averageRiskScore / 10} 
                  className="w-full"
                />
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Risk Distribution</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={[
                    { range: '0-200', count: 45, fill: '#10b981' },
                    { range: '200-400', count: 28, fill: '#f59e0b' },
                    { range: '400-600', count: 15, fill: '#ef4444' },
                    { range: '600-800', count: 8, fill: '#dc2626' },
                    { range: '800-1000', count: 4, fill: '#991b1b' },
                  ]}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="range" />
                    <YAxis />
                    <Tooltip />
                    <Bar dataKey="count" />
                  </BarChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Stability Pool Tab */}
        <TabsContent value="stability" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <Card>
              <CardHeader>
                <CardTitle>Pool Composition</CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={250}>
                  <PieChart>
                    <Pie
                      data={[
                        { name: 'Penalty Funds', value: parseFloat(stabilityPoolData.penaltyFunds) },
                        { name: 'Demurrage Funds', value: parseFloat(stabilityPoolData.demurrageFunds) },
                        { name: 'Reserve Funds', value: parseFloat(stabilityPoolData.reserveFunds) },
                        { name: 'Emergency Funds', value: parseFloat(stabilityPoolData.emergencyFunds) },
                      ]}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      label={({ name, percent }) => `${name} ${(percent * 100).toFixed(1)}%`}
                      outerRadius={80}
                      fill="#8884d8"
                      dataKey="value"
                    >
                      {[0, 1, 2, 3].map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value: any) => formatCurrency(value.toString())} />
                  </PieChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Pool Details</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label className="text-sm text-gray-600">Total Pool Balance</Label>
                  <p className="text-2xl font-bold">{formatCurrency(stabilityPoolData.totalBalance)}</p>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label className="text-sm text-gray-600">Penalty Funds</Label>
                    <p className="text-lg font-semibold">{formatCurrency(stabilityPoolData.penaltyFunds)}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Demurrage Funds</Label>
                    <p className="text-lg font-semibold">{formatCurrency(stabilityPoolData.demurrageFunds)}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Reserve Funds</Label>
                    <p className="text-lg font-semibold">{formatCurrency(stabilityPoolData.reserveFunds)}</p>
                  </div>
                  <div>
                    <Label className="text-sm text-gray-600">Emergency Funds</Label>
                    <p className="text-lg font-semibold">{formatCurrency(stabilityPoolData.emergencyFunds)}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Parameters Tab */}
        <TabsContent value="parameters" className="space-y-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>Economic Parameters</CardTitle>
              <Dialog open={showParameterDialog} onOpenChange={setShowParameterDialog}>
                <DialogTrigger asChild>
                  <Button>Update Parameters</Button>
                </DialogTrigger>
                <DialogContent className="max-w-md">
                  <DialogHeader>
                    <DialogTitle>Update Economic Parameters</DialogTitle>
                  </DialogHeader>
                  <div className="space-y-4">
                    <div>
                      <Label htmlFor="baseDemurrageRate">Base Demurrage Rate (%/hour)</Label>
                      <Input
                        id="baseDemurrageRate"
                        type="number"
                        step="0.01"
                        value={parameterUpdates.baseDemurrageRate || economicParameters.baseDemurrageRate / 100}
                        onChange={(e) => setParameterUpdates({
                          ...parameterUpdates,
                          baseDemurrageRate: parseFloat(e.target.value) * 100
                        })}
                      />
                    </div>
                    <div>
                      <Label htmlFor="maxDemurrageRate">Max Demurrage Rate (%/hour)</Label>
                      <Input
                        id="maxDemurrageRate"
                        type="number"
                        step="0.01"
                        value={parameterUpdates.maxDemurrageRate || economicParameters.maxDemurrageRate / 100}
                        onChange={(e) => setParameterUpdates({
                          ...parameterUpdates,
                          maxDemurrageRate: parseFloat(e.target.value) * 100
                        })}
                      />
                    </div>
                    <div>
                      <Label htmlFor="fiatDiscountFactor">Fiat Discount Factor (%)</Label>
                      <Input
                        id="fiatDiscountFactor"
                        type="number"
                        step="1"
                        value={parameterUpdates.fiatDiscountFactor || economicParameters.fiatDiscountFactor / 100}
                        onChange={(e) => setParameterUpdates({
                          ...parameterUpdates,
                          fiatDiscountFactor: parseFloat(e.target.value) * 100
                        })}
                      />
                    </div>
                    <Button onClick={handleParameterUpdate} className="w-full">
                      Update Parameters
                    </Button>
                  </div>
                </DialogContent>
              </Dialog>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label className="text-sm text-gray-600">Base Demurrage Rate</Label>
                  <p className="text-lg font-semibold">{formatPercentage(economicParameters.baseDemurrageRate / 100)}/hour</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Max Demurrage Rate</Label>
                  <p className="text-lg font-semibold">{formatPercentage(economicParameters.maxDemurrageRate / 100)}/hour</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Stability Threshold</Label>
                  <p className="text-lg font-semibold">{formatPercentage(economicParameters.stabilityThreshold / 100)}</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Fiat Discount Factor</Label>
                  <p className="text-lg font-semibold">{formatPercentage(economicParameters.fiatDiscountFactor / 100)}</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Grace Period</Label>
                  <p className="text-lg font-semibold">{economicParameters.gracePeriodsHours} hours</p>
                </div>
                <div>
                  <Label className="text-sm text-gray-600">Emergency Threshold</Label>
                  <p className="text-lg font-semibold">{formatPercentage(economicParameters.emergencyThreshold / 100)}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Controls Tab */}
        <TabsContent value="controls" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <Card>
              <CardHeader>
                <CardTitle>Emergency Controls</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <Dialog open={showEmergencyDialog} onOpenChange={setShowEmergencyDialog}>
                  <DialogTrigger asChild>
                    <Button 
                      variant={isEmergencyMode ? "outline" : "destructive"}
                      className="w-full"
                    >
                      {isEmergencyMode ? 'Manage Emergency Mode' : 'Activate Emergency Mode'}
                    </Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Emergency Mode Control</DialogTitle>
                    </DialogHeader>
                    <div className="space-y-4">
                      {!isEmergencyMode ? (
                        <>
                          <Label htmlFor="emergencyReason">Emergency Reason</Label>
                          <Input
                            id="emergencyReason"
                            placeholder="Describe the emergency..."
                            onChange={(e) => setParameterUpdates({
                              ...parameterUpdates,
                              emergencyReason: e.target.value
                            })}
                          />
                          <Button 
                            onClick={() => handleEmergencyAction('activate', parameterUpdates.emergencyReason)}
                            className="w-full"
                            variant="destructive"
                          >
                            Activate Emergency Mode
                          </Button>
                        </>
                      ) : (
                        <Button 
                          onClick={() => handleEmergencyAction('deactivate')}
                          className="w-full"
                          variant="outline"
                        >
                          Deactivate Emergency Mode
                        </Button>
                      )}
                    </div>
                  </DialogContent>
                </Dialog>

                <Button 
                  onClick={() => refreshData()}
                  className="w-full"
                  variant="outline"
                >
                  Refresh All Data
                </Button>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>System Actions</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <Button 
                  onClick={() => executeIntervention('REBALANCE_RESERVES')}
                  className="w-full"
                  variant="outline"
                >
                  Trigger Reserve Rebalance
                </Button>
                
                <Button 
                  onClick={() => executeIntervention('UPDATE_STABILITY_METRICS')}
                  className="w-full"
                  variant="outline"
                >
                  Update Stability Metrics
                </Button>
                
                <Button 
                  onClick={() => executeIntervention('FORCE_PARAMETER_SYNC')}
                  className="w-full"
                  variant="outline"
                >
                  Force Cross-Chain Sync
                </Button>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
};
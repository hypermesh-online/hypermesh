// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { cn } from '@/lib/utils';
import {
  Coins,
  Wallet,
  ArrowUpDown,
  TrendingUp,
  TrendingDown,
  DollarSign,
  CreditCard,
  Send,
  Download,
  Settings,
  Shield,
  Zap,
  Activity,
  ArrowDownLeft,
  ArrowUpRight,
  Repeat,
  Award,
  AlertTriangle,
  Loader2,
  RefreshCw
} from 'lucide-react';
import {
  useCaesarOverview,
  useTokenValue,
  useSendTransaction,
  useClaimRewards,
  useStakeTokens
} from '@/lib/api';
import {
  TransactionType,
  TransactionStatus
} from '@/lib/api/services/CaesarAPI';
import type { TransactionsResponse, RewardsInfo } from '@/lib/api/services/CaesarAPI';

const subNavigation = [
  { name: 'Overview', href: '/caesar' },
  { name: 'Wallet', href: '/caesar/wallet' },
  { name: 'Rewards', href: '/caesar/rewards' },
  { name: 'NGauge', href: '/caesar/ngauge' },
];

function CaesarOverview() {
  // Fetch real data from Caesar backend
  const {
    balance,
    rewards,
    staking,
    transactions,
    earnings,
    exchangeRates,
    isLoading,
    error
  } = useCaesarOverview();

  const claimRewards = useClaimRewards();
  const [isRefreshing, setIsRefreshing] = React.useState(false);

  // Calculate token value in USD
  const tokenValue = useTokenValue(balance.data?.total || 0);

  // Format time ago for transactions
  const formatTimeAgo = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds} seconds ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes} minute${minutes !== 1 ? 's' : ''} ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} hour${hours !== 1 ? 's' : ''} ago`;
    const days = Math.floor(hours / 24);
    return `${days} day${days !== 1 ? 's' : ''} ago`;
  };

  // Cast transactions data to proper type
  const txData = transactions.data as TransactionsResponse | undefined;
  const rewardsData = rewards.data as RewardsInfo | undefined;

  // Format transaction for display
  const formatTransaction = (tx: any) => {
    const isIncoming = tx.type === TransactionType.Reward ||
                       (tx.type === TransactionType.Transfer && tx.to_wallet === 'DEFAULT_WALLET');
    return {
      type: tx.type,
      amount: `${isIncoming ? '+' : '-'}${tx.amount.toFixed(2)}`,
      description: tx.description || `${tx.type} transaction`,
      time: formatTimeAgo(tx.timestamp),
      status: tx.status
    };
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    // Refetch all data
    await Promise.all([
      balance.refetch(),
      rewards.refetch(),
      staking.refetch(),
      transactions.refetch(),
      earnings.refetch(),
      exchangeRates.refetch()
    ]);
    setIsRefreshing(false);
  };

  const handleClaimRewards = async () => {
    try {
      await claimRewards.mutateAsync({
        wallet_id: 'DEFAULT_WALLET'
      });
    } catch (err) {
      console.error('Failed to claim rewards:', err);
    }
  };

  if (error) {
    return (
      <div className="space-y-6">
        <Alert className="bg-red-500/10 border-red-500/30">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            Failed to connect to Caesar service. Please check your connection and try again.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center py-6">
        <div className="flex items-center justify-center space-x-3 mb-2">
          <h1 className="text-3xl font-bold bg-gradient-to-r from-yellow-400 to-orange-600 bg-clip-text text-transparent">
            Caesar Token Integration
          </h1>
          <Button
            onClick={handleRefresh}
            disabled={isRefreshing}
            size="sm"
            variant="ghost"
            className="text-yellow-400"
          >
            <RefreshCw className={cn("h-4 w-4", isRefreshing && "animate-spin")} />
          </Button>
        </div>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Real-time economic data from Caesar backend. Earn tokens by sharing resources and participating in the network.
        </p>
      </div>

      {/* Token Overview */}
      <div className="grid gap-4 md:grid-cols-5">
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Balance</CardTitle>
            <Wallet className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold text-yellow-400">
                  {balance.data?.total.toFixed(2) || '0.00'}
                </div>
                <p className="text-xs text-gray-400">
                  {tokenValue.formatted ? tokenValue.formatted : 'CSR tokens'}
                </p>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Today's Earnings</CardTitle>
            <TrendingUp className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-8 w-20" />
            ) : (
              <>
                <div className="text-2xl font-bold text-green-400">
                  +{earnings.data?.earnings_24h.toFixed(2) || '0.00'}
                </div>
                <p className="text-xs text-gray-400">CSR tokens</p>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Pending</CardTitle>
            <Activity className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-8 w-16" />
            ) : (
              <>
                <div className="text-2xl font-bold text-yellow-400">
                  {rewards.data?.pending_rewards.toFixed(2) || '0.00'}
                </div>
                <p className="text-xs text-gray-400">
                  {(rewards.data?.pending_rewards ?? 0) > 0 ? 'Ready to claim' : 'Processing'}
                </p>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Staked</CardTitle>
            <Shield className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-8 w-20" />
            ) : (
              <>
                <div className="text-2xl font-bold text-yellow-400">
                  {staking.data?.total_staked.toFixed(2) || '0.00'}
                </div>
                <p className="text-xs text-gray-400">
                  {staking.data?.apy ? `Earning ${staking.data.apy}% APY` : 'Locked'}
                </p>
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Value</CardTitle>
            <DollarSign className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-8 w-24" />
            ) : (
              <>
                <div className="text-2xl font-bold text-yellow-400">
                  {((balance.data?.total || 0) + (staking.data?.total_staked || 0)).toFixed(2)}
                </div>
                <p className="text-xs text-gray-400">
                  CSR tokens
                </p>
              </>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Transactions */}
        <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-white">Recent Transactions</CardTitle>
              <Link to="/caesar/wallet">
                <Button variant="ghost" size="sm" className="text-yellow-400 hover:text-yellow-300">
                  View All
                </Button>
              </Link>
            </div>
          </CardHeader>
          <CardContent>
            <Tabs defaultValue="all" className="w-full">
              <TabsList className="grid w-full grid-cols-3 bg-black/40">
                <TabsTrigger value="all">All</TabsTrigger>
                <TabsTrigger value="sent">Sent</TabsTrigger>
                <TabsTrigger value="received">Received</TabsTrigger>
              </TabsList>
              <TabsContent value="all" className="mt-4">
                {/* Recent Transactions */}
                <div className="space-y-2">
                  {isLoading ? (
                    <div className="flex items-center justify-center py-8">
                      <Loader2 className="h-6 w-6 animate-spin text-yellow-400" />
                    </div>
                  ) : txData?.transactions.length === 0 ? (
                    <div className="text-center py-8 text-gray-400">
                      No transactions yet
                    </div>
                  ) : (
                    txData?.transactions.slice(0, 4).map((tx) => {
                      const formatted = formatTransaction(tx);
                      const isIncoming = formatted.amount.startsWith('+');
                      return (
                        <div key={tx.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                          <div className="flex items-center space-x-3">
                            {isIncoming ? (
                              <ArrowDownLeft className="h-4 w-4 text-green-400" />
                            ) : (
                              <ArrowUpRight className="h-4 w-4 text-red-400" />
                            )}
                            <div>
                              <p className="text-sm font-medium text-white">{formatted.description}</p>
                              <p className="text-xs text-gray-400">{formatted.time}</p>
                            </div>
                          </div>
                          <div className="flex items-center space-x-2">
                            <div className={`text-sm font-bold ${isIncoming ? 'text-green-400' : 'text-red-400'}`}>
                              {formatted.amount} CSR
                            </div>
                            {formatted.status === TransactionStatus.Pending && (
                              <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
                                Pending
                              </Badge>
                            )}
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              </TabsContent>
              <TabsContent value="sent" className="mt-4">
                <div className="space-y-2">
                  {isLoading ? (
                    <Skeleton className="h-20 w-full" />
                  ) : (
                    txData?.transactions
                      .filter(tx => tx.from_wallet === 'DEFAULT_WALLET')
                      .slice(0, 4)
                      .map((tx) => {
                        const formatted = formatTransaction(tx);
                        return (
                          <div key={tx.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                            <div className="flex items-center space-x-3">
                              <ArrowUpRight className="h-4 w-4 text-red-400" />
                              <div>
                                <p className="text-sm font-medium text-white">{formatted.description}</p>
                                <p className="text-xs text-gray-400">{formatted.time}</p>
                              </div>
                            </div>
                            <div className="text-sm font-bold text-red-400">
                              {formatted.amount} CSR
                            </div>
                          </div>
                        );
                      })
                  )}
                </div>
              </TabsContent>
              <TabsContent value="received" className="mt-4">
                <div className="space-y-2">
                  {isLoading ? (
                    <Skeleton className="h-20 w-full" />
                  ) : (
                    txData?.transactions
                      .filter(tx => tx.to_wallet === 'DEFAULT_WALLET')
                      .slice(0, 4)
                      .map((tx) => {
                        const formatted = formatTransaction(tx);
                        return (
                          <div key={tx.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                            <div className="flex items-center space-x-3">
                              <ArrowDownLeft className="h-4 w-4 text-green-400" />
                              <div>
                                <p className="text-sm font-medium text-white">{formatted.description}</p>
                                <p className="text-xs text-gray-400">{formatted.time}</p>
                              </div>
                            </div>
                            <div className="text-sm font-bold text-green-400">
                              {formatted.amount} CSR
                            </div>
                          </div>
                        );
                      })
                  )}
                </div>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        {/* Quick Actions */}
        <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Quick Actions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <Button
                  className="bg-yellow-500/20 hover:bg-yellow-500/30 text-yellow-400 border-yellow-500/30"
                  disabled={!(rewardsData?.pending_rewards) || claimRewards.isPending}
                  onClick={handleClaimRewards}
                >
                  {claimRewards.isPending ? (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  ) : (
                    <Download className="mr-2 h-4 w-4" />
                  )}
                  Claim Rewards
                </Button>
                <Link to="/caesar/wallet">
                  <Button className="bg-blue-500/20 hover:bg-blue-500/30 text-blue-400 border-blue-500/30 w-full">
                    <Send className="mr-2 h-4 w-4" />
                    Send Tokens
                  </Button>
                </Link>
              </div>

              {/* Exchange Rate Info */}
              <div className="space-y-3 pt-4 border-t border-gray-800">
                <h4 className="text-sm font-medium text-gray-400">Current Exchange Rates</h4>
                {isLoading ? (
                  <Skeleton className="h-16 w-full" />
                ) : (
                  <div className="grid grid-cols-3 gap-2">
                    <div className="bg-black/20 rounded-lg p-2">
                      <p className="text-xs text-gray-400">CSR/USD</p>
                      <p className="text-sm font-bold text-white">
                        ${exchangeRates.data?.csr_to_usd.toFixed(4) || '0.00'}
                      </p>
                      {exchangeRates.data?.change_24h && (
                        <p className={`text-xs ${exchangeRates.data.change_24h >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                          {exchangeRates.data.change_24h >= 0 ? '+' : ''}{exchangeRates.data.change_24h.toFixed(2)}%
                        </p>
                      )}
                    </div>
                    <div className="bg-black/20 rounded-lg p-2">
                      <p className="text-xs text-gray-400">Volume 24h</p>
                      <p className="text-sm font-bold text-white">
                        {exchangeRates.data?.volume_24h ? `${(exchangeRates.data.volume_24h / 1000000).toFixed(2)}M` : '0'}
                      </p>
                    </div>
                    <div className="bg-black/20 rounded-lg p-2">
                      <p className="text-xs text-gray-400">APY</p>
                      <p className="text-sm font-bold text-green-400">
                        {staking.data?.apy || '0'}%
                      </p>
                    </div>
                  </div>
                )}
              </div>

              {/* Earning Breakdown */}
              <div className="space-y-3 pt-4 border-t border-gray-800">
                <h4 className="text-sm font-medium text-gray-400">Earnings Breakdown</h4>
                {isLoading ? (
                  <Skeleton className="h-20 w-full" />
                ) : earnings.data?.breakdown && earnings.data.breakdown.length > 0 ? (
                  <div className="space-y-2">
                    {earnings.data.breakdown.map((item, i) => (
                      <div key={i} className="flex items-center justify-between">
                        <span className="text-sm text-gray-400">{item.source}</span>
                        <div className="flex items-center space-x-2">
                          <span className="text-sm font-bold text-white">
                            {item.amount.toFixed(2)} CSR
                          </span>
                          <Badge
                            className={cn(
                              "text-xs",
                              item.trend === 'up'
                                ? "bg-green-500/20 text-green-400 border-green-500/30"
                                : item.trend === 'down'
                                ? "bg-red-500/20 text-red-400 border-red-500/30"
                                : "bg-gray-500/20 text-gray-400 border-gray-500/30"
                            )}
                          >
                            {item.percentage.toFixed(1)}%
                          </Badge>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-gray-500">No earnings data available</p>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function CaesarWallet() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Wallet Management</h2>
      <p className="text-gray-400">Full wallet functionality with real Caesar backend integration.</p>
      {/* Wallet implementation will use real Caesar API */}
    </div>
  );
}

function CaesarRewards() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>
      <p className="text-gray-400">Track and claim your rewards from the Caesar economic system.</p>
      {/* Rewards implementation will use real Caesar API */}
    </div>
  );
}

function CaesarNGauge() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">NGauge Integration</h2>
      <p className="text-gray-400">Advanced engagement and advertising integration.</p>
      {/* NGauge implementation will use real Caesar API */}
    </div>
  );
}

export default function CaesarModule() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      {/* Sub-navigation */}
      <nav className="flex space-x-4 border-b border-gray-800 pb-4">
        {subNavigation.map((item) => (
          <Link
            key={item.name}
            to={item.href}
            className={cn(
              'px-3 py-2 text-sm font-medium rounded-lg transition-colors',
              location.pathname === item.href
                ? 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30'
                : 'text-gray-400 hover:text-white hover:bg-gray-800/50'
            )}
          >
            {item.name}
          </Link>
        ))}
      </nav>

      {/* Routes */}
      <Routes>
        <Route path="/" element={<CaesarOverview />} />
        <Route path="/wallet" element={<CaesarWallet />} />
        <Route path="/rewards" element={<CaesarRewards />} />
        <Route path="/ngauge" element={<CaesarNGauge />} />
      </Routes>
    </div>
  );
}
// @ts-nocheck — Phase 8 will rewrite with useBlockMatrix hooks
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Wallet,
  TrendingUp,
  Download,
  Shield,
  Gift,
  AlertTriangle,
  Loader2,
  Clock,
  ArrowDownLeft,
  ArrowUpRight,
} from 'lucide-react';
import {
  useBalance,
  useTransactions,
  useRewards,
  useStakingInfo,
  useEarnings,
  useClaimRewards,
} from '@/lib/api';
import { TransactionType, TransactionStatus } from '@/lib/api';
import type { Transaction } from '@/lib/api';
import { CaesarOverview } from './caesar/CaesarOverview';

const subNavigation = [
  { name: 'Overview', href: '/caesar' },
  { name: 'Wallet', href: '/caesar/wallet' },
  { name: 'Rewards', href: '/caesar/rewards' },
];

function CaesarWallet() {
  const { data: balance, isLoading: balanceLoading, error: balanceError } = useBalance();
  const { data: txData, isLoading: txLoading } = useTransactions(undefined, 1, 20);
  const { data: staking, isLoading: stakingLoading } = useStakingInfo();

  if (balanceError) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Wallet Management</h2>
        <Alert className="bg-red-500/10 border-red-500/30">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            Caesar wallet service offline. Unable to load wallet data.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Wallet Management</h2>

      {/* Balance Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Available</CardTitle>
            <Wallet className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {balanceLoading ? <Skeleton className="h-8 w-24" /> : (
              <div className="text-2xl font-bold text-yellow-400">
                {balance?.available.toFixed(2) ?? '0.00'} <span className="text-sm font-normal">CSR</span>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Locked</CardTitle>
            <Shield className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {balanceLoading ? <Skeleton className="h-8 w-24" /> : (
              <div className="text-2xl font-bold text-orange-400">
                {balance?.locked.toFixed(2) ?? '0.00'} <span className="text-sm font-normal">CSR</span>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Staked</CardTitle>
            <TrendingUp className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {stakingLoading ? <Skeleton className="h-8 w-24" /> : (
              <>
                <div className="text-2xl font-bold text-green-400">
                  {staking?.total_staked.toFixed(2) ?? '0.00'} <span className="text-sm font-normal">CSR</span>
                </div>
                {staking?.apy ? (
                  <p className="text-xs text-gray-400">{staking.apy}% APY</p>
                ) : null}
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total</CardTitle>
            <Wallet className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {balanceLoading ? <Skeleton className="h-8 w-24" /> : (
              <div className="text-2xl font-bold text-yellow-400">
                {balance?.total.toFixed(2) ?? '0.00'} <span className="text-sm font-normal">CSR</span>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Transaction History */}
      <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Transaction History</CardTitle>
        </CardHeader>
        <CardContent>
          {txLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-yellow-400" />
            </div>
          ) : txData?.transactions && txData.transactions.length > 0 ? (
            <div className="space-y-2">
              {txData.transactions.map((tx: Transaction) => {
                const isIncoming = tx.type === TransactionType.Reward ||
                  (tx.type === TransactionType.Transfer && tx.to_wallet === 'DEFAULT_WALLET');
                return (
                  <div key={tx.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                    <div className="flex items-center space-x-3">
                      {isIncoming ? (
                        <ArrowDownLeft className="h-4 w-4 text-green-400" />
                      ) : (
                        <ArrowUpRight className="h-4 w-4 text-red-400" />
                      )}
                      <div>
                        <p className="text-sm font-medium text-white">{tx.description || `${tx.type} transaction`}</p>
                        <p className="text-xs text-gray-400">{new Date(tx.timestamp).toLocaleString()}</p>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <span className={cn("text-sm font-bold", isIncoming ? 'text-green-400' : 'text-red-400')}>
                        {isIncoming ? '+' : '-'}{tx.amount.toFixed(2)} CSR
                      </span>
                      {tx.status === TransactionStatus.Pending && (
                        <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30">Pending</Badge>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-400">No transactions yet</div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function CaesarRewards() {
  const { data: rewards, isLoading, error } = useRewards();
  const { data: earnings } = useEarnings();
  const claimRewards = useClaimRewards();

  if (error) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>
        <Alert className="bg-red-500/10 border-red-500/30">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            Caesar rewards service offline. Unable to load rewards data.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  const handleClaim = async () => {
    try {
      await claimRewards.mutateAsync({ wallet_id: 'DEFAULT_WALLET' });
    } catch (err) {
      console.error('Failed to claim rewards:', err);
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>

      {/* Rewards Summary */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Total Earned</CardTitle>
            <TrendingUp className="h-4 w-4 text-yellow-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? <Skeleton className="h-8 w-24" /> : (
              <div className="text-2xl font-bold text-yellow-400">{rewards?.total_earned.toFixed(2) ?? '0.00'}</div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-green-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Pending</CardTitle>
            <Gift className="h-4 w-4 text-green-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? <Skeleton className="h-8 w-24" /> : (
              <>
                <div className="text-2xl font-bold text-green-400">{rewards?.pending_rewards.toFixed(2) ?? '0.00'}</div>
                {(rewards?.pending_rewards ?? 0) > 0 && (
                  <Button
                    size="sm"
                    className="mt-2 bg-green-600 hover:bg-green-700 text-white"
                    disabled={claimRewards.isPending}
                    onClick={handleClaim}
                  >
                    {claimRewards.isPending ? <Loader2 className="h-3 w-3 animate-spin mr-1" /> : <Download className="h-3 w-3 mr-1" />}
                    Claim
                  </Button>
                )}
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-blue-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Claimed</CardTitle>
            <Download className="h-4 w-4 text-blue-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? <Skeleton className="h-8 w-24" /> : (
              <div className="text-2xl font-bold text-blue-400">{rewards?.claimed_rewards.toFixed(2) ?? '0.00'}</div>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Daily Rate</CardTitle>
            <Clock className="h-4 w-4 text-purple-400" />
          </CardHeader>
          <CardContent>
            {isLoading ? <Skeleton className="h-8 w-24" /> : (
              <>
                <div className="text-2xl font-bold text-purple-400">{rewards?.daily_rate.toFixed(4) ?? '0.00'}</div>
                {rewards?.multiplier && rewards.multiplier > 1 ? (
                  <Badge className="mt-1 bg-purple-500/20 text-purple-400 border-purple-500/30">
                    {rewards.multiplier}x multiplier
                  </Badge>
                ) : null}
              </>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Earnings Breakdown */}
      {earnings?.breakdown && earnings.breakdown.length > 0 && (
        <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Earnings Breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {earnings.breakdown.map((item, i) => (
                <div key={i} className="flex items-center justify-between p-3 bg-black/20 rounded-lg">
                  <span className="text-sm text-gray-400">{item.source}</span>
                  <div className="flex items-center space-x-3">
                    <span className="text-sm font-bold text-white">{item.amount.toFixed(2)} CSR</span>
                    <Badge className={cn(
                      "text-xs",
                      item.trend === 'up' ? "bg-green-500/20 text-green-400 border-green-500/30" :
                      item.trend === 'down' ? "bg-red-500/20 text-red-400 border-red-500/30" :
                      "bg-gray-500/20 text-gray-400 border-gray-500/30"
                    )}>
                      {item.percentage.toFixed(1)}%
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Reward Entries */}
      {rewards?.entries && rewards.entries.length > 0 && (
        <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Recent Reward Entries</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {rewards.entries.slice(0, 10).map((entry) => (
                <div key={entry.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
                  <div>
                    <p className="text-sm font-medium text-white">{entry.source}</p>
                    <p className="text-xs text-gray-400">{new Date(entry.timestamp).toLocaleString()}</p>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="text-sm font-bold text-green-400">+{entry.amount.toFixed(2)} CSR</span>
                    <Badge className={cn(
                      "text-xs",
                      entry.status === 'claimed' ? "bg-green-500/20 text-green-400 border-green-500/30" :
                      entry.status === 'pending' ? "bg-yellow-500/20 text-yellow-400 border-yellow-500/30" :
                      "bg-red-500/20 text-red-400 border-red-500/30"
                    )}>
                      {entry.status}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
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
      </Routes>
    </div>
  );
}

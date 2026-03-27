// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import { moduleColors } from '@/lib/tokens';
import {
  useCaesarOverview,
  useCaesarBalance,
  useCaesarRewards,
  useCaesarStaking,
  useCaesarTransactions,
} from '@/lib/hooks/useBlockMatrix';
import {
  Wallet,
  TrendingUp,
  Shield,
  Gift,
  Activity,
  ArrowDownLeft,
  ArrowUpRight,
  AlertTriangle,
} from 'lucide-react';

const colors = moduleColors.caesar;

export function CaesarOverview() {
  const overview = useCaesarOverview();
  const balance = useCaesarBalance();
  const rewards = useCaesarRewards();
  const staking = useCaesarStaking();
  const transactions = useCaesarTransactions(10);

  if (overview.isLoading && balance.isLoading) {
    return <ModuleLoading />;
  }

  if (overview.error && balance.error) {
    return (
      <div className="p-6">
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-10 w-10 text-red-400 mb-3" />
            <p className="text-red-400 font-medium">Caesar service offline</p>
            <p className="text-gray-500 text-sm mt-1">
              Unable to reach the Caesar backend. Check that the service is running.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const totalValue = (balance.data?.total ?? 0) + (staking.data?.total_staked ?? 0);

  return (
    <div className="p-6 space-y-6">
      <div className="text-center py-4">
        <h2 className={`text-2xl font-bold bg-gradient-to-r ${colors.gradient} bg-clip-text text-transparent`}>
          Caesar Overview
        </h2>
        <p className="text-gray-400 text-sm mt-1">
          Real-time economic data from Caesar EVP backend
        </p>
      </div>

      {/* Balance Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <BalanceCard
          title="Balance"
          value={balance.data?.available ?? overview.data?.balance ?? 0}
          icon={Wallet}
          loading={balance.isLoading}
          suffix="CSR"
        />
        <BalanceCard
          title="Locked"
          value={balance.data?.locked ?? overview.data?.locked ?? 0}
          icon={Shield}
          loading={balance.isLoading}
          suffix="CSR"
          valueClass="text-orange-400"
        />
        <BalanceCard
          title="Pending Rewards"
          value={rewards.data?.pending_rewards ?? overview.data?.pending_rewards ?? 0}
          icon={Gift}
          loading={rewards.isLoading}
          suffix="CSR"
          valueClass="text-green-400"
        />
        <BalanceCard
          title="Total Value"
          value={totalValue}
          icon={TrendingUp}
          loading={balance.isLoading && staking.isLoading}
          suffix="CSR"
        />
      </div>

      {/* Staking + Rewards Summary */}
      <div className="grid gap-6 md:grid-cols-2">
        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Shield className="h-5 w-5 text-yellow-400" />
              Staking
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {staking.isLoading ? (
              <div className="space-y-2">
                <Skeleton className="h-6 w-32" />
                <Skeleton className="h-6 w-24" />
              </div>
            ) : (
              <>
                <MetricRow label="Total Staked" value={`${(staking.data?.total_staked ?? 0).toFixed(2)} CSR`} />
                <MetricRow label="APY" value={`${staking.data?.apy ?? 0}%`} valueClass="text-green-400" />
                <MetricRow label="Rewards Earned" value={`${(staking.data?.total_rewards ?? 0).toFixed(2)} CSR`} />
                <MetricRow label="Available to Stake" value={`${(staking.data?.available_to_stake ?? 0).toFixed(2)} CSR`} />
              </>
            )}
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Gift className="h-5 w-5 text-yellow-400" />
              Rewards
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {rewards.isLoading ? (
              <div className="space-y-2">
                <Skeleton className="h-6 w-32" />
                <Skeleton className="h-6 w-24" />
              </div>
            ) : (
              <>
                <MetricRow label="Total Earned" value={`${(rewards.data?.total_earned ?? 0).toFixed(2)} CSR`} />
                <MetricRow label="Pending" value={`${(rewards.data?.pending_rewards ?? 0).toFixed(2)} CSR`} valueClass="text-green-400" />
                <MetricRow label="Claimed" value={`${(rewards.data?.claimed_rewards ?? 0).toFixed(2)} CSR`} />
                <MetricRow label="Daily Rate" value={`${(rewards.data?.daily_rate ?? 0).toFixed(4)} CSR/day`} />
                {(rewards.data?.multiplier ?? 0) > 1 && (
                  <div className="pt-1">
                    <Badge className="bg-purple-500/20 text-purple-400 border-purple-500/30">
                      {rewards.data?.multiplier}x multiplier
                    </Badge>
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Recent Transactions */}
      <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Activity className="h-5 w-5 text-yellow-400" />
            Recent Transactions
            <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30 text-xs ml-auto">
              {overview.data?.recent_transactions ?? transactions.data?.total ?? 0} total
            </Badge>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {transactions.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full" />
              ))}
            </div>
          ) : transactions.data?.transactions && transactions.data.transactions.length > 0 ? (
            <div className="space-y-2">
              {transactions.data.transactions.slice(0, 8).map((tx) => {
                const isIncoming = tx.type === 'reward' || tx.type === 'receive';
                return (
                  <div
                    key={tx.id}
                    className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800"
                  >
                    <div className="flex items-center space-x-3">
                      {isIncoming ? (
                        <ArrowDownLeft className="h-4 w-4 text-green-400" />
                      ) : (
                        <ArrowUpRight className="h-4 w-4 text-red-400" />
                      )}
                      <div>
                        <p className="text-sm font-medium text-white">
                          {tx.type.charAt(0).toUpperCase() + tx.type.slice(1)} transaction
                        </p>
                        <p className="text-xs text-gray-400">
                          {new Date(tx.timestamp).toLocaleString()}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <span
                        className={`text-sm font-bold ${isIncoming ? 'text-green-400' : 'text-red-400'}`}
                      >
                        {isIncoming ? '+' : '-'}{tx.amount.toFixed(2)} CSR
                      </span>
                      {tx.status === 'pending' && (
                        <Badge className="bg-yellow-500/20 text-yellow-400 border-yellow-500/30 text-xs">
                          Pending
                        </Badge>
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

function BalanceCard({
  title,
  value,
  icon: Icon,
  loading,
  suffix,
  valueClass,
}: {
  title: string;
  value: number;
  icon: React.ComponentType<{ className?: string }>;
  loading: boolean;
  suffix: string;
  valueClass?: string;
}) {
  return (
    <Card className="bg-black/40 border-yellow-500/30 backdrop-blur-lg">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{title}</CardTitle>
        <Icon className="h-4 w-4 text-yellow-400" />
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className="h-8 w-24" />
        ) : (
          <div className={`text-2xl font-bold ${valueClass ?? 'text-yellow-400'}`}>
            {value.toFixed(2)} <span className="text-sm font-normal">{suffix}</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function MetricRow({
  label,
  value,
  valueClass,
}: {
  label: string;
  value: string;
  valueClass?: string;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-gray-400">{label}</span>
      <span className={`text-sm font-bold ${valueClass ?? 'text-white'}`}>{value}</span>
    </div>
  );
}

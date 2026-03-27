// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useCaesarBalance,
  useCaesarTransactions,
  useCaesarRewards,
  useCaesarStaking,
} from '@/lib/hooks/useBlockMatrix';
import {
  Wallet,
  TrendingUp,
  Shield,
  Gift,
  Download,
  Clock,
  ArrowDownLeft,
  ArrowUpRight,
  AlertTriangle,
} from 'lucide-react';
import { CaesarOverview } from './caesar/CaesarOverview';

const subNavigation = [
  { name: 'Overview', href: '/caesar' },
  { name: 'Wallet', href: '/caesar/wallet' },
  { name: 'Rewards', href: '/caesar/rewards' },
];

function CaesarWallet() {
  const balance = useCaesarBalance();
  const transactions = useCaesarTransactions(20);
  const staking = useCaesarStaking();

  if (balance.error) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Wallet Management</h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex items-center gap-3 py-6">
            <AlertTriangle className="h-5 w-5 text-red-400" />
            <p className="text-red-400">Caesar wallet service offline. Unable to load wallet data.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Wallet Management</h2>

      <div className="grid gap-4 md:grid-cols-4">
        <WalletCard title="Available" icon={Wallet} loading={balance.isLoading}
          value={balance.data?.available ?? 0} />
        <WalletCard title="Locked" icon={Shield} loading={balance.isLoading}
          value={balance.data?.locked ?? 0} valueClass="text-orange-400" />
        <WalletCard title="Staked" icon={TrendingUp} loading={staking.isLoading}
          value={staking.data?.total_staked ?? 0} valueClass="text-green-400"
          subtitle={staking.data?.apy ? `${staking.data.apy}% APY` : undefined} />
        <WalletCard title="Total" icon={Wallet} loading={balance.isLoading}
          value={balance.data?.total ?? 0} />
      </div>

      <Card className="bg-black/40 border-gray-800 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Transaction History</CardTitle>
        </CardHeader>
        <CardContent>
          {transactions.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 4 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full" />
              ))}
            </div>
          ) : transactions.data?.transactions && transactions.data.transactions.length > 0 ? (
            <div className="space-y-2">
              {transactions.data.transactions.map((tx) => {
                const isIncoming = tx.type === 'reward' || tx.type === 'receive';
                return (
                  <div key={tx.id} className="flex items-center justify-between p-3 rounded-lg bg-black/20 border border-gray-800">
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
                      <span className={cn("text-sm font-bold", isIncoming ? 'text-green-400' : 'text-red-400')}>
                        {isIncoming ? '+' : '-'}{tx.amount.toFixed(2)} CSR
                      </span>
                      {tx.status === 'pending' && (
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
  const rewards = useCaesarRewards();

  if (rewards.error) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>
        <Card className="border-red-500/30 bg-red-500/5">
          <CardContent className="flex items-center gap-3 py-6">
            <AlertTriangle className="h-5 w-5 text-red-400" />
            <p className="text-red-400">Caesar rewards service offline. Unable to load rewards data.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Rewards Dashboard</h2>

      <div className="grid gap-4 md:grid-cols-4">
        <WalletCard title="Total Earned" icon={TrendingUp} loading={rewards.isLoading}
          value={rewards.data?.total_earned ?? 0} />
        <WalletCard title="Pending" icon={Gift} loading={rewards.isLoading}
          value={rewards.data?.pending_rewards ?? 0} valueClass="text-green-400"
          borderClass="border-green-500/30" />
        <WalletCard title="Claimed" icon={Download} loading={rewards.isLoading}
          value={rewards.data?.claimed_rewards ?? 0} valueClass="text-blue-400"
          borderClass="border-blue-500/30" />
        <WalletCard title="Daily Rate" icon={Clock} loading={rewards.isLoading}
          value={rewards.data?.daily_rate ?? 0} decimals={4} valueClass="text-purple-400"
          borderClass="border-purple-500/30"
          subtitle={rewards.data?.multiplier && rewards.data.multiplier > 1
            ? `${rewards.data.multiplier}x multiplier` : undefined} />
      </div>
    </div>
  );
}

function WalletCard({
  title,
  icon: Icon,
  loading,
  value,
  valueClass,
  borderClass,
  subtitle,
  decimals = 2,
}: {
  title: string;
  icon: React.ComponentType<{ className?: string }>;
  loading: boolean;
  value: number;
  valueClass?: string;
  borderClass?: string;
  subtitle?: string;
  decimals?: number;
}) {
  return (
    <Card className={`bg-black/40 ${borderClass ?? 'border-yellow-500/30'} backdrop-blur-lg`}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-white">{title}</CardTitle>
        <Icon className="h-4 w-4 text-yellow-400" />
      </CardHeader>
      <CardContent>
        {loading ? (
          <Skeleton className="h-8 w-24" />
        ) : (
          <>
            <div className={`text-2xl font-bold ${valueClass ?? 'text-yellow-400'}`}>
              {value.toFixed(decimals)} <span className="text-sm font-normal">CSR</span>
            </div>
            {subtitle && <p className="text-xs text-gray-400 mt-1">{subtitle}</p>}
          </>
        )}
      </CardContent>
    </Card>
  );
}

export default function CaesarModule() {
  const location = useLocation();

  return (
    <div className="space-y-6">
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

      <Routes>
        <Route path="/" element={<CaesarOverview />} />
        <Route path="/wallet" element={<CaesarWallet />} />
        <Route path="/rewards" element={<CaesarRewards />} />
      </Routes>
    </div>
  );
}

// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Caesar Economic System React Hooks
 *
 * Real-time integration with Caesar backend
 * All data is live from the Caesar service
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  caesarAPI,
  WalletResponse,
  BalanceResponse,
  TransactionsResponse,
  RewardsInfo,
  StakingInfo,
  ExchangeRates,
  AnalyticsData,
  EarningsDetails,
  SendTransactionRequest,
  ClaimRewardsRequest,
  StakeRequest,
  Transaction,
  StakePosition
} from '../services/CaesarAPI';

// Query keys for cache management
const caesarKeys = {
  all: ['caesar'] as const,
  wallet: (id?: string) => [...caesarKeys.all, 'wallet', id || 'DEFAULT_WALLET'] as const,
  balance: (id?: string) => [...caesarKeys.all, 'balance', id || 'DEFAULT_WALLET'] as const,
  transactions: (id?: string, page?: number) =>
    [...caesarKeys.all, 'transactions', id || 'DEFAULT_WALLET', page] as const,
  rewards: (id?: string) => [...caesarKeys.all, 'rewards', id || 'DEFAULT_WALLET'] as const,
  staking: (id?: string) => [...caesarKeys.all, 'staking', id || 'DEFAULT_WALLET'] as const,
  exchangeRates: () => [...caesarKeys.all, 'exchange-rates'] as const,
  analytics: (id?: string) => [...caesarKeys.all, 'analytics', id] as const,
  earnings: (id?: string) => [...caesarKeys.all, 'earnings', id || 'DEFAULT_WALLET'] as const,
};

/**
 * Get wallet information
 */
export function useWallet(walletId?: string, enabled = true) {
  return useQuery({
    queryKey: caesarKeys.wallet(walletId),
    queryFn: () => caesarAPI.getWallet(walletId),
    enabled,
    staleTime: 30000, // Consider data stale after 30 seconds
    refetchInterval: 60000, // Refetch every minute
  });
}

/**
 * Get real-time balance
 */
export function useBalance(walletId?: string, refetchInterval = 10000) {
  return useQuery({
    queryKey: caesarKeys.balance(walletId),
    queryFn: () => caesarAPI.getBalance(walletId),
    refetchInterval, // Default: refetch every 10 seconds for real-time updates
    staleTime: 5000,
  });
}

/**
 * Get transaction history with pagination
 */
export function useTransactions(walletId?: string, page = 1, limit = 50) {
  return useQuery({
    queryKey: caesarKeys.transactions(walletId, page),
    queryFn: () => caesarAPI.getTransactions(walletId, page, limit),
    staleTime: 10000,
    keepPreviousData: true, // Keep previous data while fetching new page
  });
}

/**
 * Get rewards information
 */
export function useRewards(walletId?: string, refetchInterval = 30000) {
  return useQuery({
    queryKey: caesarKeys.rewards(walletId),
    queryFn: () => caesarAPI.getRewards(walletId),
    refetchInterval, // Refetch every 30 seconds
    staleTime: 15000,
  });
}

/**
 * Get staking information
 */
export function useStakingInfo(walletId?: string) {
  return useQuery({
    queryKey: caesarKeys.staking(walletId),
    queryFn: () => caesarAPI.getStakingInfo(walletId),
    staleTime: 30000,
    refetchInterval: 60000,
  });
}

/**
 * Get live exchange rates
 */
export function useExchangeRates(refetchInterval = 5000) {
  return useQuery({
    queryKey: caesarKeys.exchangeRates(),
    queryFn: () => caesarAPI.getExchangeRates(),
    refetchInterval, // Refetch every 5 seconds for live rates
    staleTime: 2500,
  });
}

/**
 * Get analytics data
 */
export function useAnalytics(walletId?: string) {
  return useQuery({
    queryKey: caesarKeys.analytics(walletId),
    queryFn: () => caesarAPI.getAnalytics(walletId),
    staleTime: 60000,
    refetchInterval: 300000, // Refetch every 5 minutes
  });
}

/**
 * Get earnings details
 */
export function useEarnings(walletId?: string, refetchInterval = 60000) {
  return useQuery({
    queryKey: caesarKeys.earnings(walletId),
    queryFn: () => caesarAPI.getEarnings(walletId),
    refetchInterval, // Refetch every minute
    staleTime: 30000,
  });
}

/**
 * Send transaction mutation
 */
export function useSendTransaction() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: SendTransactionRequest) => caesarAPI.sendTransaction(request),
    onSuccess: (data, variables) => {
      // Invalidate balance and transactions for both sender and receiver
      queryClient.invalidateQueries(caesarKeys.balance(variables.from_wallet));
      queryClient.invalidateQueries(caesarKeys.transactions(variables.from_wallet));
      queryClient.invalidateQueries(caesarKeys.balance(variables.to_wallet));
      queryClient.invalidateQueries(caesarKeys.transactions(variables.to_wallet));
    },
  });
}

/**
 * Claim rewards mutation
 */
export function useClaimRewards() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: ClaimRewardsRequest) => caesarAPI.claimRewards(request),
    onSuccess: (data, variables) => {
      // Invalidate rewards and balance
      queryClient.invalidateQueries(caesarKeys.rewards(variables.wallet_id));
      queryClient.invalidateQueries(caesarKeys.balance(variables.wallet_id));
      queryClient.invalidateQueries(caesarKeys.transactions(variables.wallet_id));
    },
  });
}

/**
 * Stake tokens mutation
 */
export function useStakeTokens() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: StakeRequest) => caesarAPI.stakeTokens(request),
    onSuccess: (data, variables) => {
      // Invalidate staking info and balance
      queryClient.invalidateQueries(caesarKeys.staking(variables.wallet_id));
      queryClient.invalidateQueries(caesarKeys.balance(variables.wallet_id));
      queryClient.invalidateQueries(caesarKeys.transactions(variables.wallet_id));
    },
  });
}

/**
 * Unstake tokens mutation
 */
export function useUnstakeTokens() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (stakeId: string) => caesarAPI.unstakeTokens(stakeId),
    onSuccess: () => {
      // Invalidate all staking and balance data
      queryClient.invalidateQueries(caesarKeys.staking());
      queryClient.invalidateQueries(caesarKeys.balance());
      queryClient.invalidateQueries(caesarKeys.transactions());
    },
  });
}

/**
 * Combined hook for Caesar overview data
 */
export function useCaesarOverview(walletId?: string) {
  const balance = useBalance(walletId);
  const rewards = useRewards(walletId);
  const staking = useStakingInfo(walletId);
  const transactions = useTransactions(walletId, 1, 10); // Get last 10 transactions
  const exchangeRates = useExchangeRates();
  const earnings = useEarnings(walletId);

  return {
    balance,
    rewards,
    staking,
    transactions,
    exchangeRates,
    earnings,
    isLoading:
      balance.isLoading ||
      rewards.isLoading ||
      staking.isLoading ||
      transactions.isLoading ||
      exchangeRates.isLoading ||
      earnings.isLoading,
    error:
      balance.error ||
      rewards.error ||
      staking.error ||
      transactions.error ||
      exchangeRates.error ||
      earnings.error,
  };
}

/**
 * Real-time token value hook
 */
export function useTokenValue(amount: number) {
  const { data: rates } = useExchangeRates();

  if (!rates) {
    return {
      usd: 0,
      btc: 0,
      eth: 0,
      formatted: '$0.00',
    };
  }

  return {
    usd: amount * rates.csr_to_usd,
    btc: amount * rates.csr_to_btc,
    eth: amount * rates.csr_to_eth,
    formatted: new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
    }).format(amount * rates.csr_to_usd),
  };
}
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Caesar Economic System API Client
 *
 * Real integration with Caesar economic backend
 * NO MOCK DATA - All values from live Caesar service
 */

import { get, post } from '../../api';

// Caesar Data Types
export interface Wallet {
  id: string;
  address: string;
  balance: number;
  locked_balance: number;
  pending_rewards: number;
  created_at: number;
  updated_at: number;
}

export interface WalletResponse {
  success: boolean;
  wallet?: Wallet;
  error?: string;
}

export interface BalanceResponse {
  total: number;
  available: number;
  locked: number;
  pending: number;
  staked: number;
}

export interface Transaction {
  id: string;
  type: TransactionType;
  from_wallet: string;
  to_wallet: string;
  amount: number;
  fee: number;
  status: TransactionStatus;
  timestamp: number;
  description?: string;
  metadata?: Record<string, any>;
}

export enum TransactionType {
  Transfer = 'transfer',
  Reward = 'reward',
  Stake = 'stake',
  Unstake = 'unstake',
  Fee = 'fee',
  Exchange = 'exchange',
  Mint = 'mint',
}

export enum TransactionStatus {
  Pending = 'pending',
  Confirmed = 'confirmed',
  Failed = 'failed',
  Cancelled = 'cancelled',
}

export interface TransactionsResponse {
  transactions: Transaction[];
  total: number;
  page: number;
  limit: number;
}

export interface RewardEntry {
  id: string;
  type: string;
  amount: number;
  timestamp: number;
  status: 'pending' | 'claimed' | 'expired';
  source: string;
  metadata?: Record<string, any>;
}

export interface RewardsInfo {
  total_earned: number;
  pending_rewards: number;
  claimed_rewards: number;
  last_claim: number;
  entries: RewardEntry[];
  daily_rate: number;
  multiplier: number;
}

export interface StakingInfo {
  total_staked: number;
  active_stakes: StakePosition[];
  available_to_stake: number;
  total_rewards: number;
  apy: number;
  min_stake_amount: number;
  lock_periods: LockPeriod[];
}

export interface StakePosition {
  id: string;
  amount: number;
  lock_period_days: number;
  started_at: number;
  unlock_at: number;
  apy: number;
  rewards_earned: number;
  status: 'active' | 'unlocked' | 'withdrawn';
}

export interface LockPeriod {
  days: number;
  apy: number;
  min_amount: number;
  max_amount: number;
}

export interface ExchangeRates {
  csr_to_usd: number;
  csr_to_btc: number;
  csr_to_eth: number;
  last_updated: number;
  volume_24h: number;
  change_24h: number;
}

export interface AnalyticsData {
  total_supply: number;
  circulating_supply: number;
  market_cap: number;
  holders: number;
  transactions_24h: number;
  volume_24h: number;
  average_transaction: number;
  network_activity: NetworkActivity;
  staking_metrics: StakingMetrics;
}

export interface NetworkActivity {
  active_nodes: number;
  total_resources_shared: number;
  rewards_distributed_24h: number;
  new_users_24h: number;
}

export interface StakingMetrics {
  total_staked: number;
  staking_ratio: number;
  average_lock_period: number;
  total_stakers: number;
}

export interface EarningsDetails {
  total_earnings: number;
  earnings_24h: number;
  earnings_7d: number;
  earnings_30d: number;
  breakdown: EarningsBreakdown[];
  projection_daily: number;
  projection_monthly: number;
}

export interface EarningsBreakdown {
  source: string;
  amount: number;
  percentage: number;
  trend: 'up' | 'down' | 'stable';
}

export interface SendTransactionRequest {
  from_wallet: string;
  to_wallet: string;
  amount: number;
  description?: string;
  metadata?: Record<string, any>;
}

export interface ClaimRewardsRequest {
  wallet_id: string;
  reward_ids?: string[];
}

export interface StakeRequest {
  wallet_id: string;
  amount: number;
  lock_period_days: number;
}

// Caesar API Client
class CaesarAPI {
  private baseUrl = '/api/v1/caesar';

  /**
   * Get wallet information
   */
  async getWallet(walletId?: string): Promise<WalletResponse> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<WalletResponse>(`${this.baseUrl}/wallet?id=${wallet}`);
  }

  /**
   * Get wallet balance
   */
  async getBalance(walletId?: string): Promise<BalanceResponse> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<BalanceResponse>(`${this.baseUrl}/wallet/${wallet}/balance`);
  }

  /**
   * Get transaction history
   */
  async getTransactions(
    walletId?: string,
    page: number = 1,
    limit: number = 50
  ): Promise<TransactionsResponse> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<TransactionsResponse>(
      `${this.baseUrl}/transactions?wallet=${wallet}&page=${page}&limit=${limit}`
    );
  }

  /**
   * Get rewards information
   */
  async getRewards(walletId?: string): Promise<RewardsInfo> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<RewardsInfo>(`${this.baseUrl}/rewards?wallet=${wallet}`);
  }

  /**
   * Get staking information
   */
  async getStakingInfo(walletId?: string): Promise<StakingInfo> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<StakingInfo>(`${this.baseUrl}/staking?wallet=${wallet}`);
  }

  /**
   * Get current exchange rates
   */
  async getExchangeRates(): Promise<ExchangeRates> {
    return get<ExchangeRates>(`${this.baseUrl}/exchange/rates`);
  }

  /**
   * Get analytics data
   */
  async getAnalytics(walletId?: string): Promise<AnalyticsData> {
    const params = walletId ? `?wallet=${walletId}` : '';
    return get<AnalyticsData>(`${this.baseUrl}/analytics/overview${params}`);
  }

  /**
   * Get earnings details
   */
  async getEarnings(walletId?: string): Promise<EarningsDetails> {
    const wallet = walletId || 'DEFAULT_WALLET';
    return get<EarningsDetails>(`${this.baseUrl}/analytics/earnings?wallet=${wallet}`);
  }

  /**
   * Send transaction
   */
  async sendTransaction(request: SendTransactionRequest): Promise<Transaction> {
    return post<Transaction>(`${this.baseUrl}/transactions/send`, request);
  }

  /**
   * Claim pending rewards
   */
  async claimRewards(request: ClaimRewardsRequest): Promise<{ success: boolean; amount: number }> {
    return post<{ success: boolean; amount: number }>(
      `${this.baseUrl}/rewards/claim`,
      request
    );
  }

  /**
   * Stake tokens
   */
  async stakeTokens(request: StakeRequest): Promise<StakePosition> {
    return post<StakePosition>(`${this.baseUrl}/staking/stake`, request);
  }

  /**
   * Unstake tokens
   */
  async unstakeTokens(stakeId: string): Promise<{ success: boolean; amount: number }> {
    return post<{ success: boolean; amount: number }>(
      `${this.baseUrl}/staking/unstake`,
      { stake_id: stakeId }
    );
  }

  /**
   * Get real-time token price (WebSocket)
   */
  subscribeToPrice(callback: (price: number) => void): () => void {
    // In production, implement WebSocket subscription
    // For now, poll the exchange rates
    const interval = setInterval(async () => {
      try {
        const rates = await this.getExchangeRates();
        callback(rates.csr_to_usd);
      } catch (error) {
        console.error('Failed to fetch price:', error);
      }
    }, 5000); // Poll every 5 seconds

    return () => clearInterval(interval);
  }
}

// Export singleton instance
export const caesarAPI = new CaesarAPI();
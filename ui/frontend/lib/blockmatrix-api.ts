// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * BlockMatrix API Client
 *
 * Simple singleton client for the BlockMatrix API via Gateway at localhost:8443.
 * All endpoints return JSON directly (no RPC wrapper on success).
 *
 * Working endpoints:
 *   GET  /api/v1/status
 *   GET  /api/v1/ping
 *   GET  /api/v1/blockchain/height
 *   GET  /api/v1/blockchain/block/{index}
 *   GET  /api/v1/blockchain/validate
 *   GET  /api/v1/dns/list
 *   GET  /api/v1/dns/resolve/{name}
 *   POST /api/v1/dns/register
 *   GET  /api/v1/network/peers
 *   GET  /api/v1/topology/info
 *   GET  /api/v1/topology/neighbors
 *   GET  /api/v1/asset/list
 *   GET  /api/v1/dashboard/list
 *   GET  /api/v1/dashboard/info
 *   GET  /api/v1/config/show
 *   GET  /api/v1/config/get/{key}
 *   GET  /api/v1/domain/list
 *   POST /api/v1/domain/register
 *   POST /api/v1/domain/join
 */

import { getConfig } from './config';

// --- Response types matching the Rust IPC handlers ---

export interface NodeStatus {
  node_id: string;
  coordinate: { x: number; y: number; z: number };
  chain_height: number;
  privacy_mode: string;
  peers: number;
  uptime_secs: number;
}

export interface BlockData {
  index: number;
  hash: string;
  previous_hash: string;
  timestamp: number;
  assets: number;
  [key: string]: unknown;
}

export interface DnsRecord {
  name: string;
  address: string;
  [key: string]: unknown;
}

export interface TopologyInfo {
  coordinate: { x: number; y: number; z: number };
  node_id: string;
  [key: string]: unknown;
}

export interface TopologyNeighbor {
  coordinate: { x: number; y: number; z: number };
  distance: number;
  [key: string]: unknown;
}

export interface PeerInfo {
  node_id: string;
  address: string;
  coordinate?: { x: number; y: number; z: number };
  [key: string]: unknown;
}

export interface AssetRecord {
  id: string;
  category: string;
  content_hash: string;
  block_index: number;
  [key: string]: unknown;
}

export interface DomainRecord {
  name: string;
  owner: string;
  [key: string]: unknown;
}

export interface ShareInvite {
  invite_id: string;
  asset_id: string;
  sender_node_id: string;
  sender_name?: string;
  asset_name: string;
  asset_size: number;
  shard_count: number;
  created_at: number;
}

export interface ShareInboxResponse {
  invites: ShareInvite[];
  count: number;
}

export interface ShareActionResponse {
  invite_id: string;
  status: string;
}

export interface MessageItem {
  message_id: string;
  sender_node_id: string;
  sender_name?: string;
  recipient_node_id: string;
  body: string;
  content_type: string;
  reply_to?: string;
  created_at: number;
}

export interface MessageInboxResponse {
  messages: MessageItem[];
  count: number;
}

// --- Caesar response types ---

export interface CaesarOverview {
  balance: number;
  locked: number;
  pending_rewards: number;
  total_staked: number;
  recent_transactions: number;
  [key: string]: unknown;
}

export interface CaesarBalance {
  total: number;
  available: number;
  locked: number;
  pending: number;
  staked: number;
  [key: string]: unknown;
}

export interface TransactionItem {
  id: string;
  type: string;
  from_wallet: string;
  to_wallet: string;
  amount: number;
  fee: number;
  status: string;
  timestamp: number;
  [key: string]: unknown;
}

export interface TransactionList {
  transactions: TransactionItem[];
  total: number;
  [key: string]: unknown;
}

export interface RewardsInfo {
  total_earned: number;
  pending_rewards: number;
  claimed_rewards: number;
  daily_rate: number;
  multiplier: number;
  [key: string]: unknown;
}

export interface StakingInfo {
  total_staked: number;
  available_to_stake: number;
  total_rewards: number;
  apy: number;
  active_stakes: Array<{
    id: string;
    amount: number;
    apy: number;
    status: string;
    [key: string]: unknown;
  }>;
  [key: string]: unknown;
}

// --- Engauge response types ---

export interface CapacityMetrics {
  cpu_usage: number;
  memory_usage: number;
  storage_usage: number;
  network_usage: number;
  total_capacity: number;
  [key: string]: unknown;
}

export interface TrafficAnalysis {
  bytes_in: number;
  bytes_out: number;
  packets_in: number;
  packets_out: number;
  active_flows: number;
  [key: string]: unknown;
}

export interface ThrottleStatus {
  is_throttled: boolean;
  current_rate: number;
  max_rate: number;
  reason?: string;
  [key: string]: unknown;
}

export interface RoutingAdvisory {
  recommended_paths: Array<{
    destination: string;
    metric: number;
    [key: string]: unknown;
  }>;
  congestion_level: number;
  [key: string]: unknown;
}

// --- TrustChain response types ---

export interface TrustChainStatus {
  ca_status: string;
  total_certs: number;
  active_certs: number;
  revoked_certs: number;
  [key: string]: unknown;
}

export interface CertRecord {
  id: string;
  subject: string;
  issuer: string;
  valid_from: string;
  valid_to: string;
  status: string;
  [key: string]: unknown;
}

export interface CertList {
  certificates: CertRecord[];
  total: number;
  [key: string]: unknown;
}

export interface IdentityInfo {
  node_id: string;
  public_key: string;
  key_algorithm: string;
  created_at: number;
  [key: string]: unknown;
}

export interface FederationInfo {
  peers: Array<{
    node_id: string;
    trust_level: string;
    [key: string]: unknown;
  }>;
  total_peers: number;
  [key: string]: unknown;
}

// --- STOQ response types ---

export interface StoqStats {
  connections_active: number;
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
  [key: string]: unknown;
}

export interface ConnectionRecord {
  id: string;
  remote_addr: string;
  state: string;
  bytes_sent: number;
  bytes_received: number;
  [key: string]: unknown;
}

export interface ConnectionList {
  connections: ConnectionRecord[];
  total: number;
  [key: string]: unknown;
}

export interface PerformanceMetrics {
  throughput_mbps: number;
  latency_ms: number;
  packet_loss_pct: number;
  jitter_ms: number;
  [key: string]: unknown;
}

// --- Client ---

class BlockMatrixClient {
  private baseUrl: string;

  constructor() {
    this.baseUrl = getConfig().api.baseUrl;
  }

  private async fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);

    try {
      const url = `${this.baseUrl}${path}`;
      const res = await fetch(url, {
        ...init,
        signal: controller.signal,
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json',
          ...init?.headers,
        },
      });

      clearTimeout(timeout);

      if (!res.ok) {
        const text = await res.text().catch(() => res.statusText);
        throw new BlockMatrixError(res.status, text, path);
      }

      return (await res.json()) as T;
    } catch (err) {
      clearTimeout(timeout);
      if (err instanceof BlockMatrixError) throw err;
      if (err instanceof Error && err.name === 'AbortError') {
        throw new BlockMatrixError(408, 'Request timeout', path);
      }
      throw new BlockMatrixError(0, String(err), path);
    }
  }

  // --- Core ---

  async getStatus(): Promise<NodeStatus> {
    return this.fetchJson('/api/v1/status');
  }

  async ping(): Promise<string> {
    return this.fetchJson('/api/v1/ping');
  }

  // --- Blockchain ---

  async getBlockchainHeight(): Promise<{ height: number }> {
    return this.fetchJson('/api/v1/blockchain/height');
  }

  async getBlock(index: number): Promise<BlockData> {
    return this.fetchJson(`/api/v1/blockchain/block/${index}`);
  }

  async validateChain(): Promise<{ valid: boolean; height: number }> {
    return this.fetchJson('/api/v1/blockchain/validate');
  }

  // --- DNS ---

  async getDnsList(): Promise<DnsRecord[]> {
    return this.fetchJson('/api/v1/dns/list');
  }

  async resolveDns(name: string): Promise<DnsRecord> {
    return this.fetchJson(`/api/v1/dns/resolve/${encodeURIComponent(name)}`);
  }

  async registerDns(body: { name: string; address: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/dns/register', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  // --- Network ---

  async getNetworkPeers(): Promise<PeerInfo[]> {
    return this.fetchJson('/api/v1/network/peers');
  }

  // --- Topology ---

  async getTopologyInfo(): Promise<TopologyInfo> {
    return this.fetchJson('/api/v1/topology/info');
  }

  async getTopologyNeighbors(): Promise<TopologyNeighbor[]> {
    return this.fetchJson('/api/v1/topology/neighbors');
  }

  // --- Assets ---

  async getAssetList(): Promise<AssetRecord[]> {
    return this.fetchJson('/api/v1/asset/list');
  }

  // --- Dashboard ---

  async getDashboardList(): Promise<unknown[]> {
    return this.fetchJson('/api/v1/dashboard/list');
  }

  async getDashboardInfo(): Promise<unknown> {
    return this.fetchJson('/api/v1/dashboard/info');
  }

  // --- Config ---

  async getConfig(): Promise<unknown> {
    return this.fetchJson('/api/v1/config/show');
  }

  async getConfigKey(key: string): Promise<unknown> {
    return this.fetchJson(`/api/v1/config/get/${encodeURIComponent(key)}`);
  }

  // --- Domain ---

  async getDomainList(): Promise<DomainRecord[]> {
    return this.fetchJson('/api/v1/domain/list');
  }

  async registerDomain(body: { name: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/domain/register', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  async joinDomain(body: { domain: string; token: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/domain/join', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  // --- Sharing ---

  async shareSend(assetId: string, recipient: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/share/send', {
      method: 'POST',
      body: JSON.stringify({ asset_id: assetId, recipient }),
    });
  }

  async shareInbox(limit?: number): Promise<ShareInboxResponse> {
    const params = limit ? `?limit=${limit}` : '';
    return this.fetchJson(`/api/v1/share/inbox${params}`);
  }

  async shareAccept(inviteId: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/share/accept', {
      method: 'POST',
      body: JSON.stringify({ invite_id: inviteId }),
    });
  }

  async shareReject(inviteId: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/share/reject', {
      method: 'POST',
      body: JSON.stringify({ invite_id: inviteId }),
    });
  }

  // --- Messaging ---

  async messageSend(
    recipient: string,
    body: string,
    contentType?: string,
    replyTo?: string,
  ): Promise<{ message_id: string; status: string }> {
    return this.fetchJson('/api/v1/message/send', {
      method: 'POST',
      body: JSON.stringify({
        recipient,
        body,
        content_type: contentType ?? 'text/plain',
        ...(replyTo ? { reply_to: replyTo } : {}),
      }),
    });
  }

  async messageInbox(limit?: number): Promise<MessageInboxResponse> {
    const params = limit ? `?limit=${limit}` : '';
    return this.fetchJson(`/api/v1/message/inbox${params}`);
  }

  async messageHistory(peer: string, limit?: number): Promise<MessageInboxResponse> {
    const params = new URLSearchParams({ peer });
    if (limit) params.set('limit', String(limit));
    return this.fetchJson(`/api/v1/message/history?${params.toString()}`);
  }

  async messageRead(messageId: string): Promise<{ message: MessageItem }> {
    return this.fetchJson('/api/v1/message/read', {
      method: 'POST',
      body: JSON.stringify({ message_id: messageId }),
    });
  }

  // --- Caesar ---

  async caesarOverview(): Promise<CaesarOverview> {
    return this.fetchJson('/api/v1/caesar/overview');
  }

  async caesarBalance(): Promise<CaesarBalance> {
    return this.fetchJson('/api/v1/caesar/balance');
  }

  async caesarTransactions(limit?: number): Promise<TransactionList> {
    const params = limit ? `?limit=${limit}` : '';
    return this.fetchJson(`/api/v1/caesar/transactions${params}`);
  }

  async caesarRewards(): Promise<RewardsInfo> {
    return this.fetchJson('/api/v1/caesar/rewards');
  }

  async caesarStaking(): Promise<StakingInfo> {
    return this.fetchJson('/api/v1/caesar/staking');
  }

  // --- Engauge ---

  async engaugeCapacity(): Promise<CapacityMetrics> {
    return this.fetchJson('/api/v1/engauge/capacity');
  }

  async engaugeTraffic(): Promise<TrafficAnalysis> {
    return this.fetchJson('/api/v1/engauge/traffic');
  }

  async engaugeThrottle(): Promise<ThrottleStatus> {
    return this.fetchJson('/api/v1/engauge/throttle');
  }

  async engaugeRouting(): Promise<RoutingAdvisory> {
    return this.fetchJson('/api/v1/engauge/routing');
  }

  // --- TrustChain ---

  async trustchainStatus(): Promise<TrustChainStatus> {
    return this.fetchJson('/api/v1/trustchain/status');
  }

  async trustchainCerts(): Promise<CertList> {
    return this.fetchJson('/api/v1/trustchain/certs');
  }

  async trustchainIdentity(): Promise<IdentityInfo> {
    return this.fetchJson('/api/v1/trustchain/identity');
  }

  async trustchainFederation(): Promise<FederationInfo> {
    return this.fetchJson('/api/v1/trustchain/federation');
  }

  // --- STOQ ---

  async stoqStats(): Promise<StoqStats> {
    return this.fetchJson('/api/v1/stoq/stats');
  }

  async stoqConnections(): Promise<ConnectionList> {
    return this.fetchJson('/api/v1/stoq/connections');
  }

  async stoqPerformance(): Promise<PerformanceMetrics> {
    return this.fetchJson('/api/v1/stoq/performance');
  }
}

export class BlockMatrixError extends Error {
  constructor(
    public status: number,
    message: string,
    public path: string
  ) {
    super(message);
    this.name = 'BlockMatrixError';
  }
}

/** Singleton client instance */
export const blockMatrixClient = new BlockMatrixClient();

// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * BlockMatrix API Client
 *
 * Single Gateway entry point for browser + Tauri desktop clients.
 *
 * The Gateway HTTP/3 surface is a multi-service reverse proxy. Path prefix
 * encodes which daemon owns the request:
 *
 *   /api/v1/blockmatrix/*  → blockmatrix daemon ([::1]:9292)
 *   /api/v1/caesar/*       → caesar daemon ([::1]:9294)
 *   /api/v1/catalog/*      → catalog daemon ([::1]:9295)
 *   /api/v1/engauge/*      → engauge daemon ([::1]:9296)
 *   /api/v1/trustchain/*   → trustchain daemon ([::1]:8444)
 *
 * Scope-aware visibility is determined by the request's capability token +
 * privacy-mode header — NOT by client form factor.
 *
 * Working endpoints (blockmatrix daemon, served via /api/v1/blockmatrix/* prefix):
 *   GET  /api/v1/blockmatrix/status
 *   GET  /api/v1/blockmatrix/ping
 *   GET  /api/v1/blockmatrix/blockchain/height
 *   GET  /api/v1/blockmatrix/blockchain/block/{index}
 *   GET  /api/v1/blockmatrix/blockchain/validate
 *   GET  /api/v1/blockmatrix/dns/list
 *   GET  /api/v1/blockmatrix/dns/resolve/{name}
 *   POST /api/v1/blockmatrix/dns/register
 *   GET  /api/v1/blockmatrix/network/peers
 *   GET  /api/v1/blockmatrix/topology/info
 *   GET  /api/v1/blockmatrix/topology/neighbors
 *   GET  /api/v1/blockmatrix/asset/list
 *   POST /api/v1/blockmatrix/asset/register
 *   GET  /api/v1/blockmatrix/dashboard/list
 *   GET  /api/v1/blockmatrix/dashboard/info
 *   GET  /api/v1/blockmatrix/config/show
 *   GET  /api/v1/blockmatrix/config/get/{key}
 *   GET  /api/v1/blockmatrix/domain/list
 *   POST /api/v1/blockmatrix/domain/register
 *   POST /api/v1/blockmatrix/domain/join
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

/**
 * Input for `asset.register` IPC method.
 *
 * Required: `category` ('system' | 'application'), `content` (hex-encoded bytes).
 * Optional: `type_name`, `type_hash` (hex), `metadata` (JSON object).
 */
export interface AssetRegisterInput {
  category: 'system' | 'application';
  content: string;
  type_name?: string;
  type_hash?: string;
  metadata?: Record<string, unknown>;
}

export interface AssetRegisterResponse {
  asset_id: string;
  block_index: number;
  status: string;
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

export interface CertExtension {
  oid: string;
  critical: boolean;
  name: string | null;
}

export interface CertRecord {
  id: string;
  subject: string;
  issuer: string;
  valid_from: string;
  valid_to: string;
  status: 'active' | 'expired' | 'not_yet_valid';
  serial_number: string;
  signature_algorithm: string;
  signature_algorithm_oid: string;
  key_algorithm: string;
  key_algorithm_oid: string;
  fingerprint_sha256: string;
  fingerprint_blake3: string;
  key_usage: string[];
  extended_key_usage: string[];
  subject_alt_names: string[];
  extensions: CertExtension[];
  path: string;
}

export interface CertList {
  node_id: string;
  certificates: CertRecord[];
  total: number;
  status: string;
  error?: string;
}

export interface KeyInfo {
  present: boolean;
  bytes: number;
  fingerprint: string | null;
  key_algorithm: string;
}

export interface IdentityInfo {
  node_id: string;
  falcon: KeyInfo;
  kyber: KeyInfo;
  created_at: number | null;
  privacy_mode: string;
  status: string;
}

export interface FederationPeer {
  node_id: string;
  trust_level: 'full' | 'conditional' | 'untrusted';
  joined_at?: number;
  fingerprint?: string;
}

export interface FederationInfo {
  node_id: string;
  peers: FederationPeer[];
  total_peers: number;
  network_peers: number;
  trust_levels: { full: number; conditional: number; untrusted: number };
  status: string;
  note: string;
}

// --- STOQ response types ---

export interface StoqStats {
  node_id?: string;
  connections?: number;
  /** Legacy alias kept for compatibility with older daemons that emitted
   * `connections_active` instead of `connections`. */
  connections_active?: number;
  unique_endpoints?: number;
  transport_active?: boolean;
  shard_transport_active?: boolean;
  bytes_sent?: number;
  bytes_received?: number;
  packets_sent?: number;
  packets_received?: number;
  protocol?: string;
  privacy_mode?: string;
  uptime_secs?: number;
  [key: string]: unknown;
}

/**
 * STOQ connection record. Shape matches the daemon `stoq.connections` IPC
 * handler in `blockmatrix/src/ipc/handlers/stoq.rs::handle_connections`.
 * Per-connection byte counters are not yet exposed by the daemon — fields
 * are optional so consumers can render honest empty states.
 */
export interface ConnectionRecord {
  node_id: string;
  address: string;
  coordinate?: { x: number; y: number; z: number };
  protocol?: string;
  /** Reserved for future per-connection byte counters. */
  bytes_sent?: number;
  /** Reserved for future per-connection byte counters. */
  bytes_received?: number;
  /** Reserved for future per-connection state reporting. */
  state?: string;
  [key: string]: unknown;
}

export interface ConnectionList {
  count: number;
  connections: ConnectionRecord[];
  /** Legacy alias for `count`. */
  total?: number;
  note?: string;
  [key: string]: unknown;
}

/**
 * STOQ performance metrics. Daemon `stoq.performance` IPC handler currently
 * returns: `node_id, active_connections, avg_latency_ms, min_latency_ms,
 * max_latency_ms, throughput_bps, packet_loss_rate, congestion_window,
 * uptime_secs, status`. The forward-compatible fields below match the
 * stable contract; consumers should null-check before formatting.
 */
export interface PerformanceMetrics {
  node_id?: string;
  active_connections?: number;
  avg_latency_ms?: number;
  min_latency_ms?: number;
  max_latency_ms?: number;
  throughput_bps?: number;
  packet_loss_rate?: number;
  congestion_window?: number;
  uptime_secs?: number;
  status?: string;
  /** Legacy alias: throughput in megabits/sec. */
  throughput_mbps?: number;
  /** Legacy alias: latency in milliseconds. */
  latency_ms?: number;
  /** Legacy alias: packet loss as a percentage (0-100). */
  packet_loss_pct?: number;
  /** Legacy alias: jitter in milliseconds. */
  jitter_ms?: number;
  [key: string]: unknown;
}

// --- Dashboard response types ---

export interface DashboardEntry {
  name?: string;
  version?: string;
  domain?: string;
  description?: string;
  hash: string;
  block?: number;
  error?: string;
  [key: string]: unknown;
}

export interface DashboardList {
  count: number;
  dashboards: DashboardEntry[];
  [key: string]: unknown;
}

export interface DashboardAccess {
  public?: boolean;
  private?: boolean;
  [key: string]: unknown;
}

export interface DashboardInfo {
  name: string;
  version?: string;
  domain?: string;
  description?: string;
  found: boolean;
  hash: string;
  block?: number;
  files?: number;
  access?: DashboardAccess;
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
    return this.fetchJson('/api/v1/blockmatrix/status');
  }

  async ping(): Promise<string> {
    return this.fetchJson('/api/v1/blockmatrix/ping');
  }

  // --- Blockchain ---

  async getBlockchainHeight(): Promise<{ height: number }> {
    return this.fetchJson('/api/v1/blockmatrix/blockchain/height');
  }

  async getBlock(index: number): Promise<BlockData> {
    return this.fetchJson(`/api/v1/blockmatrix/blockchain/block/${index}`);
  }

  async validateChain(): Promise<{ valid: boolean; height: number }> {
    return this.fetchJson('/api/v1/blockmatrix/blockchain/validate');
  }

  // --- DNS ---

  async getDnsList(): Promise<DnsRecord[]> {
    return this.fetchJson('/api/v1/blockmatrix/dns/list');
  }

  async resolveDns(name: string): Promise<DnsRecord> {
    return this.fetchJson(`/api/v1/blockmatrix/dns/resolve/${encodeURIComponent(name)}`);
  }

  async registerDns(body: { name: string; address: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/blockmatrix/dns/register', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  // --- Network ---

  async getNetworkPeers(): Promise<PeerInfo[]> {
    return this.fetchJson('/api/v1/blockmatrix/network/peers');
  }

  // --- Topology ---

  async getTopologyInfo(): Promise<TopologyInfo> {
    return this.fetchJson('/api/v1/blockmatrix/topology/info');
  }

  async getTopologyNeighbors(): Promise<TopologyNeighbor[]> {
    return this.fetchJson('/api/v1/blockmatrix/topology/neighbors');
  }

  // --- Assets ---

  async getAssetList(): Promise<AssetRecord[]> {
    return this.fetchJson('/api/v1/blockmatrix/asset/list');
  }

  async registerAsset(body: AssetRegisterInput): Promise<AssetRegisterResponse> {
    return this.fetchJson('/api/v1/blockmatrix/asset/register', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  // --- Dashboard ---

  async getDashboardList(): Promise<DashboardList> {
    return this.fetchJson('/api/v1/blockmatrix/dashboard/list');
  }

  async getDashboardInfo(name: string): Promise<DashboardInfo> {
    return this.fetchJson('/api/v1/blockmatrix/dashboard/info', {
      method: 'POST',
      body: JSON.stringify({ name }),
    });
  }

  // --- Config ---

  async getConfig(): Promise<unknown> {
    return this.fetchJson('/api/v1/blockmatrix/config/show');
  }

  async getConfigKey(key: string): Promise<unknown> {
    return this.fetchJson(`/api/v1/blockmatrix/config/get/${encodeURIComponent(key)}`);
  }

  // --- Domain ---

  async getDomainList(): Promise<DomainRecord[]> {
    return this.fetchJson('/api/v1/blockmatrix/domain/list');
  }

  async registerDomain(body: { name: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/blockmatrix/domain/register', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  async joinDomain(body: { domain: string; token: string }): Promise<unknown> {
    return this.fetchJson('/api/v1/blockmatrix/domain/join', {
      method: 'POST',
      body: JSON.stringify(body),
    });
  }

  // --- Sharing ---

  async shareSend(assetId: string, recipient: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/blockmatrix/share/send', {
      method: 'POST',
      body: JSON.stringify({ asset_id: assetId, recipient }),
    });
  }

  async shareInbox(limit?: number): Promise<ShareInboxResponse> {
    const params = limit ? `?limit=${limit}` : '';
    return this.fetchJson(`/api/v1/blockmatrix/share/inbox${params}`);
  }

  async shareAccept(inviteId: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/blockmatrix/share/accept', {
      method: 'POST',
      body: JSON.stringify({ invite_id: inviteId }),
    });
  }

  async shareReject(inviteId: string): Promise<ShareActionResponse> {
    return this.fetchJson('/api/v1/blockmatrix/share/reject', {
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
    return this.fetchJson('/api/v1/blockmatrix/message/send', {
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
    return this.fetchJson(`/api/v1/blockmatrix/message/inbox${params}`);
  }

  async messageHistory(peer: string, limit?: number): Promise<MessageInboxResponse> {
    const params = new URLSearchParams({ peer });
    if (limit) params.set('limit', String(limit));
    return this.fetchJson(`/api/v1/blockmatrix/message/history?${params.toString()}`);
  }

  async messageRead(messageId: string): Promise<{ message: MessageItem }> {
    return this.fetchJson('/api/v1/blockmatrix/message/read', {
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

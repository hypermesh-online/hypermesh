// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * BlockMatrix API Client
 *
 * Simple singleton client for the real BlockMatrix HTTP API at localhost:9293.
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

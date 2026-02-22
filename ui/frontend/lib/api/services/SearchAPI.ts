// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Global Search API Service
 *
 * Real-time search across all Web3 ecosystem services
 * Integrates with HyperMesh assets, Caesar transactions, and TrustChain certificates
 */

import { get } from '../../api';
import { Asset } from './HyperMeshAPI';
import { Transaction } from './CaesarAPI';
import { Certificate } from './TrustChainAPI';

export interface SearchResult {
  id: string;
  type: 'asset' | 'transaction' | 'certificate' | 'node' | 'contract' | 'compute-job';
  title: string;
  description: string;
  metadata: Record<string, any>;
  relevance: number;
  path: string;
  timestamp?: string;
  tags?: string[];
  source: 'hypermesh' | 'caesar' | 'trustchain' | 'catalog';
}

export interface SearchFilter {
  type?: string[];
  dateRange?: { start?: string; end?: string };
  network?: string[];
  status?: string[];
  metadata?: Record<string, string>;
  limit?: number;
  offset?: number;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
  page: number;
  limit: number;
  query: string;
  filters: SearchFilter;
}

export interface SearchSuggestion {
  type: 'query' | 'filter' | 'recent' | 'popular';
  text: string;
  description?: string;
  count?: number;
}

class SearchAPI {
  private baseUrl = '/api/v1/search';

  /**
   * Perform global search across all services
   */
  async search(query: string, filters?: SearchFilter): Promise<SearchResponse> {
    const params = new URLSearchParams({
      q: query,
      ...(filters?.limit && { limit: filters.limit.toString() }),
      ...(filters?.offset && { offset: filters.offset.toString() }),
    });

    if (filters?.type?.length) {
      params.append('types', filters.type.join(','));
    }
    if (filters?.network?.length) {
      params.append('networks', filters.network.join(','));
    }
    if (filters?.status?.length) {
      params.append('statuses', filters.status.join(','));
    }

    return get<SearchResponse>(`${this.baseUrl}?${params.toString()}`);
  }

  /**
   * Get search suggestions based on partial query
   */
  async getSuggestions(query: string): Promise<SearchSuggestion[]> {
    if (!query || query.length < 2) return [];

    return get<SearchSuggestion[]>(`${this.baseUrl}/suggestions?q=${encodeURIComponent(query)}`);
  }

  /**
   * Search HyperMesh assets
   */
  async searchAssets(query: string, filters?: SearchFilter): Promise<SearchResult[]> {
    const response = await get<Asset[]>(`/api/v1/hypermesh/assets/search?q=${encodeURIComponent(query)}`);

    return response.map(asset => ({
      id: asset.id,
      type: 'asset' as const,
      title: asset.name,
      description: asset.description || `${asset.type} asset`,
      metadata: {
        type: asset.type,
        privacyLevel: asset.privacyLevel,
        status: asset.status,
      },
      relevance: 100, // Server should provide relevance score
      path: `/hypermesh/assets/${asset.id}`,
      timestamp: new Date(asset.createdAt).toISOString(),
      tags: [asset.type, asset.privacyLevel],
      source: 'hypermesh' as const,
    }));
  }

  /**
   * Search Caesar transactions
   */
  async searchTransactions(query: string, filters?: SearchFilter): Promise<SearchResult[]> {
    const response = await get<Transaction[]>(`/api/v1/caesar/transactions/search?q=${encodeURIComponent(query)}`);

    return response.map(tx => ({
      id: tx.id,
      type: 'transaction' as const,
      title: `${tx.type} - ${tx.amount} CSR`,
      description: tx.description || `Transaction from ${tx.from_wallet} to ${tx.to_wallet}`,
      metadata: {
        amount: tx.amount,
        fee: tx.fee,
        type: tx.type,
        status: tx.status,
        from: tx.from_wallet,
        to: tx.to_wallet,
      },
      relevance: 100,
      path: `/caesar/transactions/${tx.id}`,
      timestamp: new Date(tx.timestamp).toISOString(),
      tags: [tx.type, tx.status],
      source: 'caesar' as const,
    }));
  }

  /**
   * Search TrustChain certificates
   */
  async searchCertificates(query: string, filters?: SearchFilter): Promise<SearchResult[]> {
    const response = await get<Certificate[]>(`/api/v1/trustchain/certificates/search?q=${encodeURIComponent(query)}`);

    return response.map(cert => ({
      id: cert.serialNumber,
      type: 'certificate' as const,
      title: cert.commonName || cert.subject || 'TrustChain Certificate',
      description: `Certificate for ${cert.subject}`,
      metadata: {
        issuer: cert.issuer,
        validFrom: cert.validFrom,
        validUntil: cert.validTo,
        algorithm: cert.signatureAlgorithm,
        status: new Date(cert.validTo) > new Date() ? 'valid' : 'expired',
      },
      relevance: 100,
      path: `/trustchain/certificates/${cert.serialNumber}`,
      timestamp: cert.validFrom,
      tags: ['certificate', 'trustchain', cert.signatureAlgorithm || 'unknown'],
      source: 'trustchain' as const,
    }));
  }

  /**
   * Search nodes in the network
   */
  async searchNodes(query: string, filters?: SearchFilter): Promise<SearchResult[]> {
    const response = await get<any[]>(`/api/v1/hypermesh/nodes/search?q=${encodeURIComponent(query)}`);

    return response.map(node => ({
      id: node.id,
      type: 'node' as const,
      title: node.name || `Node ${node.id}`,
      description: `${node.type} node at ${node.address}`,
      metadata: {
        address: node.address,
        connections: node.connections,
        uptime: node.uptime,
        status: node.status,
        resources: node.resources,
      },
      relevance: 100,
      path: `/hypermesh/nodes/${node.id}`,
      timestamp: node.last_seen,
      tags: ['node', node.type, node.status],
      source: 'hypermesh' as const,
    }));
  }

  /**
   * Get recent searches for current user
   */
  async getRecentSearches(): Promise<string[]> {
    // In production, this would fetch from user preferences/history
    const stored = localStorage.getItem('recent_searches');
    return stored ? JSON.parse(stored).slice(0, 5) : [];
  }

  /**
   * Save a search to history
   */
  saveSearchToHistory(query: string) {
    const stored = localStorage.getItem('recent_searches');
    const searches = stored ? JSON.parse(stored) : [];

    // Remove duplicates and add to front
    const updated = [query, ...searches.filter((s: string) => s !== query)].slice(0, 10);
    localStorage.setItem('recent_searches', JSON.stringify(updated));
  }

  /**
   * Get trending searches
   */
  async getTrendingSearches(): Promise<SearchSuggestion[]> {
    try {
      return await get<SearchSuggestion[]>(`${this.baseUrl}/trending`);
    } catch {
      // Fallback to default trending topics
      return [
        { type: 'popular', text: 'consensus proofs', count: 145 },
        { type: 'popular', text: 'caesar staking', count: 89 },
        { type: 'popular', text: 'trustchain certificates', count: 67 },
        { type: 'popular', text: 'hypermesh nodes', count: 54 },
      ];
    }
  }
}

// Export singleton instance
export const searchAPI = new SearchAPI();
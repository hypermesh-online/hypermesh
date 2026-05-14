// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * Shared UI type definitions that don't belong in the BlockMatrix API
 * response surface but are referenced by multiple components.
 *
 * Runtime data must come from `@/lib/hooks/useBlockMatrix`. This module
 * holds only types/enums.
 */

// --- Privacy ---

export type PrivacyLevel =
  | 'private'
  | 'private_network'
  | 'p2p'
  | 'public_network'
  | 'full_public'
  | 'federated'
  | 'public'
  | 'anonymous'
  | 'verified';

// --- Catalog ---

export interface CatalogApplication {
  id: string;
  name: string;
  version: string;
  type: 'Application' | 'Library' | 'Runtime' | 'Service' | 'Data';
  adapter: 'Docker' | 'WASM' | 'Native' | 'Python' | 'Node.js' | 'Julia';
  status: 'Available' | 'Installed' | 'Installing' | 'Failed' | 'Updating';
  description: string;
  category?: string;
  requirements: { cpu?: number; memory?: number; storage?: number; network?: boolean };
  dependencies: string[];
  author: string;
  downloads: number;
  downloadCount?: number;
  rating: number;
  size: string;
  lastUpdated: string;
  tags?: string[];
  performance?: { latency: number; throughput: number };
  assetId?: string;
  privacyLevel?: PrivacyLevel;
}

// --- Search (reserved for M.4 mesh fan-out search) ---

export interface SearchResult {
  id: string;
  type: string;
  title: string;
  description: string;
  relevance?: number;
  tags?: string[];
  metadata?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface SearchFilter {
  type?: string[];
  network?: string[];
  [key: string]: string[] | string | undefined;
}

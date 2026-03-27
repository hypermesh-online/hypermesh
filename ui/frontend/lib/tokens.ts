// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/** Module color tokens — centralized visual identity for all HyperMesh modules */
export const moduleColors = {
  hypermesh: {
    gradient: 'from-cyan-400 to-blue-600',
    text: 'text-cyan-400',
    bg: 'bg-cyan-500/20',
    border: 'border-cyan-500/50',
  },
  trustchain: {
    gradient: 'from-green-400 to-emerald-600',
    text: 'text-green-400',
    bg: 'bg-green-500/20',
    border: 'border-green-500/50',
  },
  caesar: {
    gradient: 'from-yellow-400 to-amber-600',
    text: 'text-yellow-400',
    bg: 'bg-yellow-500/20',
    border: 'border-yellow-500/50',
  },
  engauge: {
    gradient: 'from-orange-400 to-red-600',
    text: 'text-orange-400',
    bg: 'bg-orange-500/20',
    border: 'border-orange-500/30',
  },
  stoq: {
    gradient: 'from-purple-400 to-indigo-600',
    text: 'text-purple-400',
    bg: 'bg-purple-500/20',
    border: 'border-purple-500/50',
  },
  catalog: {
    gradient: 'from-pink-400 to-rose-600',
    text: 'text-pink-400',
    bg: 'bg-pink-500/20',
    border: 'border-pink-500/50',
  },
} as const;

export type ModuleName = keyof typeof moduleColors;

/** Status indicator colors */
export const statusColors = {
  online: 'bg-green-500',
  offline: 'bg-red-500',
  warning: 'bg-yellow-500',
  syncing: 'bg-blue-500',
} as const;

export type StatusName = keyof typeof statusColors;

/** Polling intervals in milliseconds for React Query refetch cycles */
export const polling = {
  fast: 5_000,    // STOQ stats, engauge throttle
  normal: 10_000, // node status, blockchain height
  slow: 15_000,   // peers, assets, domains
  rare: 30_000,   // caesar overview, trustchain status
  static: 0,      // no polling (identity, config)
} as const;

export type PollingSpeed = keyof typeof polling;

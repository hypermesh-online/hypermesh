// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type { ThemeColors, TopologyNode, TopologyLink } from './types';

const themes: Record<string, ThemeColors> = {
  cyan: {
    core: '#22d3ee', edge: '#06b6d4', endpoint: '#0891b2',
    gateway: '#0e7490', service: '#155e75',
    linkActive: 'rgba(34, 211, 238, 0.8)', linkInactive: 'rgba(34, 211, 238, 0.3)',
    linkCongested: '#ef4444', background: 'rgba(34, 211, 238, 0.05)', grid: 'rgba(34, 211, 238, 0.1)'
  },
  green: {
    core: '#4ade80', edge: '#22c55e', endpoint: '#16a34a',
    gateway: '#15803d', service: '#166534',
    linkActive: 'rgba(74, 222, 128, 0.8)', linkInactive: 'rgba(74, 222, 128, 0.3)',
    linkCongested: '#ef4444', background: 'rgba(74, 222, 128, 0.05)', grid: 'rgba(74, 222, 128, 0.1)'
  },
  purple: {
    core: '#a855f7', edge: '#9333ea', endpoint: '#7c3aed',
    gateway: '#6d28d9', service: '#5b21b6',
    linkActive: 'rgba(168, 85, 247, 0.8)', linkInactive: 'rgba(168, 85, 247, 0.3)',
    linkCongested: '#ef4444', background: 'rgba(168, 85, 247, 0.05)', grid: 'rgba(168, 85, 247, 0.1)'
  },
  red: {
    core: '#f87171', edge: '#ef4444', endpoint: '#dc2626',
    gateway: '#b91c1c', service: '#991b1b',
    linkActive: 'rgba(248, 113, 113, 0.8)', linkInactive: 'rgba(248, 113, 113, 0.3)',
    linkCongested: '#fbbf24', background: 'rgba(248, 113, 113, 0.05)', grid: 'rgba(248, 113, 113, 0.1)'
  },
  yellow: {
    core: '#fbbf24', edge: '#f59e0b', endpoint: '#d97706',
    gateway: '#b45309', service: '#92400e',
    linkActive: 'rgba(251, 191, 36, 0.8)', linkInactive: 'rgba(251, 191, 36, 0.3)',
    linkCongested: '#ef4444', background: 'rgba(251, 191, 36, 0.05)', grid: 'rgba(251, 191, 36, 0.1)'
  }
};

export function getThemeColors(theme: string): ThemeColors {
  return themes[theme] || themes.cyan;
}

export function getNodeColor(node: TopologyNode, colors: ThemeColors): string {
  switch (node.status) {
    case 'error': return '#ef4444';
    case 'warning': return '#fbbf24';
    case 'maintenance': return '#6b7280';
    case 'inactive': return '#4b5563';
  }

  switch (node.type) {
    case 'core': return colors.core;
    case 'edge': return colors.edge;
    case 'endpoint': return colors.endpoint;
    case 'gateway': return colors.gateway;
    case 'service': return colors.service;
    default: return colors.edge;
  }
}

export function getNodeRadius(node: TopologyNode): number {
  const baseRadius: Record<string, number> = {
    core: 16, gateway: 12, edge: 10, service: 8, endpoint: 6
  };

  let radius = baseRadius[node.type] || 8;

  if (node.metrics?.connections) {
    radius += Math.min(node.metrics.connections / 5, 8);
  }

  return radius;
}

export function getLinkColor(link: TopologyLink, colors: ThemeColors): string {
  switch (link.status) {
    case 'congested': return colors.linkCongested;
    case 'degraded': return '#fbbf24';
    case 'inactive': return colors.linkInactive;
    default: return colors.linkActive;
  }
}

export function getLinkWidth(link: TopologyLink): number {
  let width = 2;

  if (link.bandwidth) {
    width = Math.max(1, Math.min(link.bandwidth / 1000, 8));
  }

  if (link.utilization && link.utilization > 80) {
    width += 2;
  }

  return width;
}

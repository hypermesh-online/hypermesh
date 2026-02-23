// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import type React from 'react';

export interface TopologyNode {
  id: string;
  label: string;
  type: 'core' | 'edge' | 'endpoint' | 'gateway' | 'service';
  status: 'active' | 'inactive' | 'warning' | 'error' | 'maintenance';
  position?: { x: number; y: number };
  metrics?: {
    load?: number;
    connections?: number;
    latency?: number;
    throughput?: number;
  };
  groups?: string[];
}

export interface TopologyLink {
  source: string;
  target: string;
  type: 'physical' | 'logical' | 'data' | 'control' | 'tunnel';
  status: 'active' | 'inactive' | 'congested' | 'degraded';
  bandwidth?: number;
  latency?: number;
  utilization?: number;
  weight?: number;
}

export interface TopologyChartProps {
  nodes: TopologyNode[];
  links: TopologyLink[];
  width?: number;
  height?: number;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  layout?: 'force' | 'hierarchical' | 'circular' | 'grid';
  interactive?: boolean;
  showMetrics?: boolean;
  showLabels?: boolean;
  onNodeClick?: (node: TopologyNode) => void;
  onLinkClick?: (link: TopologyLink) => void;
  className?: string;
}

export interface ThemeColors {
  core: string;
  edge: string;
  endpoint: string;
  gateway: string;
  service: string;
  linkActive: string;
  linkInactive: string;
  linkCongested: string;
  background: string;
  grid: string;
}

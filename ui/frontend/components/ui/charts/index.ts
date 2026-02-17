// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export { AreaChart } from './AreaChart';
export { BarChart } from './BarChart';
export { LineChart } from './LineChart';
export { PieChart } from './PieChart';
export { NetworkGraph } from './NetworkGraph';
export { SparklineChart } from './SparklineChart';
export { GaugeChart } from './GaugeChart';
export { MetricDisplay } from './MetricDisplay';
export { ChartContainer } from './ChartContainer';
export { TopologyChart } from './TopologyChart';
export { PerformanceChart } from './PerformanceChart';
export { SystemMetrics } from './SystemMetrics';

// Re-export types for convenience
export type { 
  TopologyNode, 
  TopologyLink 
} from './TopologyChart';

export type { 
  PerformanceDataPoint, 
  PerformanceMetric 
} from './PerformanceChart';

export type { 
  SystemMetric, 
  MetricValue 
} from './SystemMetrics';

// Chart themes
export const CHART_THEMES = ['cyan', 'green', 'purple', 'red', 'yellow'] as const;
export type ChartTheme = typeof CHART_THEMES[number];

// Common chart configurations
export const CHART_CONFIGS = {
  colors: {
    cyan: ['#22d3ee', '#06b6d4', '#0891b2', '#0e7490', '#155e75'],
    green: ['#4ade80', '#22c55e', '#16a34a', '#15803d', '#166534'],
    purple: ['#a855f7', '#9333ea', '#7c3aed', '#6d28d9', '#5b21b6'],
    red: ['#f87171', '#ef4444', '#dc2626', '#b91c1c', '#991b1b'],
    yellow: ['#fbbf24', '#f59e0b', '#d97706', '#b45309', '#92400e']
  },
  status: {
    excellent: '#4ade80',
    good: '#22d3ee',
    warning: '#fbbf24',
    critical: '#ef4444',
    inactive: '#6b7280'
  },
  opacity: {
    fill: 0.1,
    fillActive: 0.2,
    stroke: 0.8,
    strokeActive: 1.0,
    grid: 0.1
  }
} as const;

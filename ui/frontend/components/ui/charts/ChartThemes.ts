// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Centralized theme configuration for all charts
export const CHART_THEMES = {
  cyan: {
    name: 'Cyan',
    primary: '#22d3ee',
    secondary: '#06b6d4',
    accent: '#0891b2',
    colors: ['#22d3ee', '#06b6d4', '#0891b2', '#0e7490', '#155e75'],
    gradients: {
      light: 'from-cyan-400 to-cyan-600',
      medium: 'from-cyan-500 to-blue-600',
      dark: 'from-cyan-600 to-blue-800'
    },
    opacity: {
      fill: 'rgba(34, 211, 238, 0.1)',
      fillActive: 'rgba(34, 211, 238, 0.2)',
      stroke: 'rgba(34, 211, 238, 0.8)',
      grid: 'rgba(34, 211, 238, 0.1)'
    },
    status: {
      excellent: '#4ade80',
      good: '#22d3ee',
      warning: '#fbbf24',
      critical: '#ef4444',
      inactive: '#6b7280'
    }
  },
  
  green: {
    name: 'Green',
    primary: '#4ade80',
    secondary: '#22c55e',
    accent: '#16a34a',
    colors: ['#4ade80', '#22c55e', '#16a34a', '#15803d', '#166534'],
    gradients: {
      light: 'from-green-400 to-green-600',
      medium: 'from-green-500 to-emerald-600',
      dark: 'from-green-600 to-emerald-800'
    },
    opacity: {
      fill: 'rgba(74, 222, 128, 0.1)',
      fillActive: 'rgba(74, 222, 128, 0.2)',
      stroke: 'rgba(74, 222, 128, 0.8)',
      grid: 'rgba(74, 222, 128, 0.1)'
    },
    status: {
      excellent: '#4ade80',
      good: '#22c55e',
      warning: '#fbbf24',
      critical: '#ef4444',
      inactive: '#6b7280'
    }
  },
  
  purple: {
    name: 'Purple',
    primary: '#a855f7',
    secondary: '#9333ea',
    accent: '#7c3aed',
    colors: ['#a855f7', '#9333ea', '#7c3aed', '#6d28d9', '#5b21b6'],
    gradients: {
      light: 'from-purple-400 to-purple-600',
      medium: 'from-purple-500 to-indigo-600',
      dark: 'from-purple-600 to-indigo-800'
    },
    opacity: {
      fill: 'rgba(168, 85, 247, 0.1)',
      fillActive: 'rgba(168, 85, 247, 0.2)',
      stroke: 'rgba(168, 85, 247, 0.8)',
      grid: 'rgba(168, 85, 247, 0.1)'
    },
    status: {
      excellent: '#4ade80',
      good: '#a855f7',
      warning: '#fbbf24',
      critical: '#ef4444',
      inactive: '#6b7280'
    }
  },
  
  red: {
    name: 'Red',
    primary: '#f87171',
    secondary: '#ef4444',
    accent: '#dc2626',
    colors: ['#f87171', '#ef4444', '#dc2626', '#b91c1c', '#991b1b'],
    gradients: {
      light: 'from-red-400 to-red-600',
      medium: 'from-red-500 to-pink-600',
      dark: 'from-red-600 to-pink-800'
    },
    opacity: {
      fill: 'rgba(248, 113, 113, 0.1)',
      fillActive: 'rgba(248, 113, 113, 0.2)',
      stroke: 'rgba(248, 113, 113, 0.8)',
      grid: 'rgba(248, 113, 113, 0.1)'
    },
    status: {
      excellent: '#4ade80',
      good: '#22d3ee',
      warning: '#fbbf24',
      critical: '#f87171',
      inactive: '#6b7280'
    }
  },
  
  yellow: {
    name: 'Yellow',
    primary: '#fbbf24',
    secondary: '#f59e0b',
    accent: '#d97706',
    colors: ['#fbbf24', '#f59e0b', '#d97706', '#b45309', '#92400e'],
    gradients: {
      light: 'from-yellow-400 to-yellow-600',
      medium: 'from-yellow-500 to-orange-600',
      dark: 'from-yellow-600 to-orange-800'
    },
    opacity: {
      fill: 'rgba(251, 191, 36, 0.1)',
      fillActive: 'rgba(251, 191, 36, 0.2)',
      stroke: 'rgba(251, 191, 36, 0.8)',
      grid: 'rgba(251, 191, 36, 0.1)'
    },
    status: {
      excellent: '#4ade80',
      good: '#22d3ee',
      warning: '#fbbf24',
      critical: '#ef4444',
      inactive: '#6b7280'
    }
  }
} as const;

export type ChartTheme = keyof typeof CHART_THEMES;

// Helper functions for theme usage
export const getTheme = (theme: ChartTheme) => CHART_THEMES[theme];

export const getThemeColor = (theme: ChartTheme, variant: 'primary' | 'secondary' | 'accent' = 'primary') => {
  return CHART_THEMES[theme][variant];
};

export const getThemeColors = (theme: ChartTheme) => CHART_THEMES[theme].colors;

export const getThemeGradient = (theme: ChartTheme, variant: 'light' | 'medium' | 'dark' = 'medium') => {
  return CHART_THEMES[theme].gradients[variant];
};

export const getStatusColor = (status: 'excellent' | 'good' | 'warning' | 'critical' | 'inactive', theme: ChartTheme = 'cyan') => {
  return CHART_THEMES[theme].status[status];
};

// Animation configurations
export const CHART_ANIMATIONS = {
  duration: {
    fast: 300,
    normal: 500,
    slow: 800,
    verySlow: 1200
  },
  easing: {
    linear: 'linear',
    easeIn: 'ease-in',
    easeOut: 'ease-out', 
    easeInOut: 'ease-in-out',
    spring: 'cubic-bezier(0.68, -0.55, 0.265, 1.55)'
  },
  delays: {
    stagger: 100,
    sequence: 200
  }
} as const;

// Chart dimension presets
export const CHART_SIZES = {
  small: { width: 300, height: 200 },
  medium: { width: 600, height: 400 },
  large: { width: 800, height: 600 },
  wide: { width: 1000, height: 400 },
  tall: { width: 400, height: 800 }
} as const;

// Common chart configurations
export const CHART_DEFAULTS = {
  padding: { top: 20, right: 30, bottom: 40, left: 60 },
  strokeWidth: 2,
  pointRadius: 4,
  gridOpacity: 0.1,
  animationDuration: CHART_ANIMATIONS.duration.normal,
  showGrid: true,
  showLabels: true,
  showTooltips: true
} as const;

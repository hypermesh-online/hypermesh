// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Professional Design System - Apple/Google Standards
 * Clean, minimal design tokens for modern applications
 */

// Clean Color System - Minimal & Professional
export const colors = {
  // Neutral grays (primary palette)
  gray: {
    50: '#fafafa',
    100: '#f4f4f5',
    200: '#e4e4e7',
    300: '#d4d4d8',
    400: '#a1a1aa',
    500: '#71717a',
    600: '#52525b',
    700: '#3f3f46',
    800: '#27272a',
    900: '#18181b',
  },
  
  // Single accent color (iOS blue)
  primary: {
    50: '#eff6ff',
    100: '#dbeafe',
    500: '#3b82f6',
    600: '#2563eb',
    700: '#1d4ed8',
  },
  
  // Semantic colors only
  success: '#10b981',
  warning: '#f59e0b',
  error: '#ef4444',
  
  // Clean basics
  white: '#ffffff',
  black: '#000000',
} as const;

// Clean Typography System - SF Pro/Roboto Standards
export const typography = {
  fontFamily: {
    sans: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    mono: 'ui-monospace, "SF Mono", Monaco, monospace',
  },
  
  fontSize: {
    xs: '12px',    // Caption
    sm: '14px',    // Small text
    base: '16px',  // Body
    lg: '20px',    // Subheading
    xl: '24px',    // Heading
    '2xl': '32px', // Large heading
  },
  
  fontWeight: {
    normal: '400',
    medium: '500',
    semibold: '600',
  },
  
  lineHeight: {
    tight: '1.2',
    normal: '1.5',
  }
} as const;

// Consistent Spacing (8px grid)
export const spacing = {
  xs: '4px',
  sm: '8px',
  md: '16px',
  lg: '24px',
  xl: '32px',
  '2xl': '40px',
} as const;

// Border Radius
export const borderRadius = {
  sm: '4px',
  md: '8px',
  lg: '12px',
  xl: '16px',
  full: '9999px',
} as const;

// Clean Shadows
export const shadows = {
  sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
  md: '0 4px 6px -1px rgb(0 0 0 / 0.1)',
  lg: '0 10px 15px -3px rgb(0 0 0 / 0.1)',
} as const;

// Animation
export const animation = {
  fast: '150ms',
  normal: '250ms',
  slow: '350ms',
} as const;

// Export clean design tokens
export const designTokens = {
  colors,
  typography,
  spacing,
  borderRadius,
  shadows,
  animation,
} as const;

export type DesignTokens = typeof designTokens;
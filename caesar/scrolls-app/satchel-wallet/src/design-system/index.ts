// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Caesar Design System - Main Export Index
 * Complete professional design system for Caesar Token ecosystem
 */

// Design Tokens
export { designTokens, colors, typography, spacing, borderRadius, shadows, animation } from './tokens';
export type { DesignTokens } from './tokens';

// Core Components
export {
  Text,
  Card,
  Button,
  Input,
  Badge,
  Progress,
  Separator,
} from './components';

// Layout & Grid System
export {
  Grid,
  GridItem,
  Flex,
  Stack,
  Container,
  AspectRatio,
} from './grid';

// Typography System
export {
  Typography,
  Spacer,
  TextBlock,
  CodeBlock,
  InlineCode,
  Quote,
  List,
  typographyScale,
} from './typography';

// Animation & Interaction System
export {
  Animated,
  Staggered,
  HoverEffect,
  Loading,
  Parallax,
  Reveal,
  Transition,
  Morph,
} from './animations';

// Utility Functions
export {
  cn,
  formatCurrency,
  formatCompactNumber,
  formatPercentage,
  formatTokenAmount,
  responsive,
  hexToRgb,
  rgbToHsl,
  createStaggeredAnimation,
  smoothScrollTo,
  safeLocalStorage,
  debounce,
  throttle,
  isValidEthereumAddress,
  isValidEmail,
  isValidUrl,
  copyToClipboard,
  generateId,
  deepMerge,
  formatTimeAgo,
  breakpoints,
  useMediaQuery,
} from './utils';

// Error Boundary Component
export { ErrorBoundary } from './ErrorBoundary';

// Re-export everything for convenience
export * from './tokens';
export * from './components';
export * from './grid';
export * from './typography';
export * from './animations';
export * from './utils';
# Responsive UI Component Library
*Phase 2 Design Deliverable*

## Overview
Comprehensive UI component library designed specifically for the CAESAR ecosystem, emphasizing demurrage awareness, anti-speculation features, and cross-chain functionality with responsive design across all device types.

## Design System Foundation

### Color Palette
```scss
// Primary Colors - CAESAR Branding
$caesar-gold: #FFD700;        // Primary brand color
$caesar-amber: #FFA500;       // Secondary actions
$caesar-bronze: #CD7F32;      // Tertiary elements

// Semantic Colors
$success: #10B981;            // Positive actions, profits
$warning: #F59E0B;            // Caution, demurrage alerts  
$error: #EF4444;              // Errors, penalties
$info: #3B82F6;               // Information, tips

// Neutral Palette
$background-primary: #FFFFFF;    // Light theme primary
$background-secondary: #F8FAFC;  // Light theme secondary
$background-dark: #0F172A;       // Dark theme primary
$background-dark-secondary: #1E293B; // Dark theme secondary
$text-primary: #1E293B;         // Primary text
$text-secondary: #64748B;       // Secondary text
$border: #E2E8F0;              // Borders, dividers
```

### Typography Scale
```scss
// Font Stack
$font-family-primary: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
$font-family-mono: 'JetBrains Mono', 'Fira Code', monospace;

// Font Sizes (Mobile-first responsive)
$text-xs: 0.75rem;    // 12px - Small labels
$text-sm: 0.875rem;   // 14px - Secondary text  
$text-base: 1rem;     // 16px - Body text
$text-lg: 1.125rem;   // 18px - Large body
$text-xl: 1.25rem;    // 20px - Small headings
$text-2xl: 1.5rem;    // 24px - Medium headings
$text-3xl: 1.875rem;  // 30px - Large headings
$text-4xl: 2.25rem;   // 36px - Display text

// Font Weights
$font-light: 300;
$font-regular: 400;
$font-medium: 500;
$font-semibold: 600;
$font-bold: 700;
```

### Spacing System
```scss
// Spacing Scale (8px base unit)
$space-1: 0.25rem;   // 4px
$space-2: 0.5rem;    // 8px
$space-3: 0.75rem;   // 12px
$space-4: 1rem;      // 16px
$space-5: 1.25rem;   // 20px
$space-6: 1.5rem;    // 24px
$space-8: 2rem;      // 32px
$space-10: 2.5rem;   // 40px
$space-12: 3rem;     // 48px
$space-16: 4rem;     // 64px
$space-20: 5rem;     // 80px
```

### Breakpoints
```scss
// Responsive Breakpoints
$breakpoint-sm: 640px;   // Small tablets
$breakpoint-md: 768px;   // Large tablets
$breakpoint-lg: 1024px;  // Small desktops
$breakpoint-xl: 1280px;  // Large desktops
$breakpoint-2xl: 1536px; // Extra large screens
```

## Core Components

### 1. CaesarButton Component
```jsx
interface CaesarButtonProps {
  variant: 'primary' | 'secondary' | 'warning' | 'danger' | 'ghost';
  size: 'sm' | 'md' | 'lg' | 'xl';
  fullWidth?: boolean;
  loading?: boolean;
  disabled?: boolean;
  icon?: ReactNode;
  children: ReactNode;
  onClick?: () => void;
}

// Usage Examples:
<CaesarButton variant="primary" size="lg" fullWidth>
  Execute Trade
</CaesarButton>

<CaesarButton variant="warning" icon={<AlertIcon />}>
  ⚠️ Anti-Speculation Alert
</CaesarButton>
```

### 2. DemurrageCard Component
```jsx
interface DemurrageCardProps {
  balance: BigNumber;
  demurrageRate: number;
  nextPaymentDate: Date;
  nextPaymentAmount: BigNumber;
  showOptimizations?: boolean;
  compact?: boolean;
}

// Component Features:
// - Real-time demurrage calculation
// - Countdown timer to next payment
// - Optimization suggestions
// - Historical demurrage tracking
// - Responsive layout adaptation
```

### 3. AntiSpeculationMonitor Component  
```jsx
interface AntiSpeculationMonitorProps {
  tradesRemaining: number;
  maxDailyTrades: number;
  recentTrades: Trade[];
  nextResetTime: Date;
  showPenaltyCalculator?: boolean;
  onTradeWarning?: (penalty: BigNumber) => void;
}

// Component Features:
// - Visual trade counter (dots/progress bar)
// - Recent trade history
// - Penalty calculator
// - Reset timer countdown
// - Warning notifications
```

### 4. CrossChainSelector Component
```jsx
interface CrossChainSelectorProps {
  supportedChains: Chain[];
  selectedChain: Chain;
  onChainSelect: (chain: Chain) => void;
  showBalance?: boolean;
  showGasCosts?: boolean;
  disabled?: boolean;
}

// Component Features:
// - Network icons and names
// - Balance display per chain
// - Gas cost estimates
// - Network status indicators
// - Mobile-optimized dropdown
```

### 5. TokenBalanceDisplay Component
```jsx
interface TokenBalanceDisplayProps {
  balance: BigNumber;
  symbol: string;
  usdValue?: BigNumber;
  showDemurrageImpact?: boolean;
  showFullPrecision?: boolean;
  size: 'sm' | 'md' | 'lg';
  animate?: boolean;
}

// Component Features:
// - Animated balance updates
// - USD conversion display
// - Demurrage impact visualization
// - Precision toggle
// - Copy to clipboard
```

### 6. TransactionProgressTracker Component
```jsx
interface TransactionProgressTrackerProps {
  transaction: CrossChainTransaction;
  onComplete?: () => void;
  onError?: (error: Error) => void;
  showDetails?: boolean;
  expandable?: boolean;
}

// Component Features:
// - Step-by-step progress visualization
// - Real-time status updates
// - Error handling and retry options
// - Explorer link integration
// - Estimated completion times
```

### 7. ResponsiveTable Component
```jsx
interface ResponsiveTableProps {
  columns: TableColumn[];
  data: any[];
  loading?: boolean;
  emptyMessage?: string;
  sortable?: boolean;
  pagination?: PaginationConfig;
  mobileCardView?: boolean;
}

// Mobile Adaptation:
// - Card layout on mobile
// - Horizontal scrolling on tablet
// - Sortable columns
// - Sticky headers
// - Virtual scrolling for large datasets
```

### 8. GasEstimator Component
```jsx
interface GasEstimatorProps {
  transaction: TransactionRequest;
  chain: Chain;
  onEstimateChange?: (estimate: GasEstimate) => void;
  showBreakdown?: boolean;
  allowCustomGas?: boolean;
}

// Component Features:
// - Real-time gas price fetching
// - Gas cost breakdown
// - Multiple speed options (slow/standard/fast)
// - Custom gas price input
// - USD cost conversion
```

### 9. LiquidityPoolCard Component
```jsx
interface LiquidityPoolCardProps {
  pool: LiquidityPool;
  userPosition?: UserPoolPosition;
  showActions?: boolean;
  compact?: boolean;
  highlightDemurrageBenefit?: boolean;
}

// Component Features:
// - Pool statistics (TVL, APY, volume)
// - User position tracking
// - Demurrage mitigation highlighting
// - Add/remove liquidity actions
// - Rewards claiming interface
```

### 10. NetworkStatusIndicator Component
```jsx
interface NetworkStatusIndicatorProps {
  chains: Chain[];
  showDetails?: boolean;
  onNetworkIssue?: (chain: Chain, issue: NetworkIssue) => void;
}

// Component Features:
// - Real-time network health monitoring
// - Block confirmation times
// - Gas price trends
// - Network congestion alerts
// - Maintenance notifications
```

## Layout Components

### 1. AppShell Component
```jsx
interface AppShellProps {
  navigation: NavigationItem[];
  wallet: WalletConnection;
  theme: 'light' | 'dark' | 'auto';
  children: ReactNode;
}

// Responsive Behavior:
// - Mobile: Bottom tab navigation
// - Tablet: Collapsible sidebar
// - Desktop: Fixed sidebar navigation
// - Persistent wallet connection status
```

### 2. ResponsiveGrid Component
```jsx
interface ResponsiveGridProps {
  columns: {
    mobile: number;
    tablet: number;
    desktop: number;
  };
  gap: number;
  children: ReactNode;
}

// Grid System:
// - CSS Grid based layout
// - Responsive column counts  
// - Consistent gap spacing
// - Auto-fit/auto-fill options
```

### 3. Modal Component
```jsx
interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  size: 'sm' | 'md' | 'lg' | 'xl' | 'fullscreen';
  children: ReactNode;
}

// Responsive Behavior:
// - Mobile: Full-screen overlay
// - Tablet/Desktop: Centered modal
// - Keyboard navigation support
// - Focus trap implementation
// - Backdrop click to close
```

## Form Components

### 1. TokenAmountInput Component
```jsx
interface TokenAmountInputProps {
  value: string;
  onChange: (value: string) => void;
  token: Token;
  balance?: BigNumber;
  showMaxButton?: boolean;
  showUSDValue?: boolean;
  error?: string;
  disabled?: boolean;
}

// Component Features:
// - Balance validation
// - USD conversion display
// - Percentage shortcuts (25%, 50%, 75%, Max)
// - Demurrage impact calculation
// - Number formatting and parsing
```

### 2. ChainSelectDropdown Component
```jsx
interface ChainSelectDropdownProps {
  chains: Chain[];
  selectedChain?: Chain;
  onSelect: (chain: Chain) => void;
  filterOptions?: ChainFilter;
  showTestnets?: boolean;
}

// Component Features:
// - Network search/filtering
// - Network icons and descriptions
// - Testnet toggle
// - Recently used chains
// - Keyboard navigation
```

### 3. SlippageSelector Component
```jsx
interface SlippageSelectorProps {
  value: number;
  onChange: (slippage: number) => void;
  presetValues?: number[];
  allowCustom?: boolean;
  warningThreshold?: number;
}

// Component Features:
// - Preset slippage options (0.1%, 0.5%, 1.0%)
// - Custom slippage input
// - Warning for high slippage
// - Auto-slippage calculation
// - MEV protection considerations
```

## Utility Components

### 1. LoadingSpinner Component
```jsx
interface LoadingSpinnerProps {
  size: 'sm' | 'md' | 'lg';
  color?: string;
  message?: string;
  overlay?: boolean;
}
```

### 2. CopyToClipboard Component
```jsx
interface CopyToClipboardProps {
  text: string;
  truncate?: boolean;
  showIcon?: boolean;
  successMessage?: string;
}
```

### 3. Tooltip Component
```jsx
interface TooltipProps {
  content: ReactNode;
  placement: 'top' | 'bottom' | 'left' | 'right';
  trigger: 'hover' | 'click' | 'focus';
  children: ReactNode;
}
```

### 4. Badge Component
```jsx
interface BadgeProps {
  variant: 'primary' | 'secondary' | 'success' | 'warning' | 'error';
  size: 'sm' | 'md' | 'lg';
  children: ReactNode;
}
```

## Responsive Design Patterns

### Mobile-First Approach
```scss
// Default styles for mobile
.component {
  padding: $space-4;
  font-size: $text-base;
  
  // Tablet and up
  @media (min-width: $breakpoint-md) {
    padding: $space-6;
    font-size: $text-lg;
  }
  
  // Desktop and up  
  @media (min-width: $breakpoint-lg) {
    padding: $space-8;
    display: flex;
  }
}
```

### Container Queries (Future Enhancement)
```scss
// Component-based responsive design
.card {
  container-type: inline-size;
  
  @container (min-width: 300px) {
    .card-content {
      display: flex;
    }
  }
}
```

### Touch-Friendly Interface
```scss
// Minimum touch target sizes
.touch-target {
  min-height: 44px; // iOS recommendation
  min-width: 44px;
  
  // Larger targets on mobile
  @media (max-width: $breakpoint-md) {
    min-height: 48px;
    min-width: 48px;
  }
}
```

## Accessibility Features

### WCAG 2.1 AA Compliance
- **Color Contrast**: Minimum 4.5:1 ratio for normal text
- **Focus Management**: Visible focus indicators
- **Screen Reader Support**: ARIA labels and descriptions  
- **Keyboard Navigation**: Full keyboard accessibility
- **Semantic HTML**: Proper heading hierarchy

### Implementation Examples
```jsx
// Accessible button with proper ARIA
<button
  className="caesar-button"
  aria-describedby="demurrage-warning"
  aria-pressed={isActive}
  disabled={isLoading}
>
  {isLoading ? (
    <span aria-hidden="true">Loading...</span>
  ) : (
    'Execute Trade'
  )}
</button>

// Screen reader friendly balance display
<div role="region" aria-labelledby="balance-heading">
  <h3 id="balance-heading">Your CAESAR Balance</h3>
  <span aria-live="polite" aria-atomic="true">
    {formatBalance(balance)} CAESAR
  </span>
</div>
```

## Performance Optimizations

### Code Splitting
```jsx
// Lazy load heavy components
const CrossChainTrader = lazy(() => import('./CrossChainTrader'));
const AdvancedAnalytics = lazy(() => import('./AdvancedAnalytics'));

// Suspense boundaries
<Suspense fallback={<ComponentSkeleton />}>
  <CrossChainTrader />
</Suspense>
```

### Virtual Scrolling
```jsx
// Large lists with virtual scrolling
import { FixedSizeList as List } from 'react-window';

const TransactionHistory = ({ transactions }) => (
  <List
    height={600}
    itemCount={transactions.length}
    itemSize={80}
    itemData={transactions}
  >
    {TransactionRow}
  </List>
);
```

### Memoization Strategy
```jsx
// Expensive calculations memoized
const DemurrageCalculator = memo(({ balance, rate }) => {
  const calculation = useMemo(() => 
    calculateDemurrage(balance, rate), 
    [balance, rate]
  );
  
  return <div>{calculation}</div>;
});
```

## Testing Strategy

### Component Testing
- **Unit Tests**: Individual component behavior
- **Integration Tests**: Component interaction
- **Visual Regression Tests**: UI consistency
- **Accessibility Tests**: WCAG compliance
- **Performance Tests**: Rendering optimization

### Testing Tools
- **Jest**: Unit test framework
- **React Testing Library**: Component testing
- **Playwright**: E2E testing
- **Chromatic**: Visual testing
- **axe-core**: Accessibility testing

---
*This component library provides a comprehensive foundation for building responsive, accessible, and performant interfaces for the CAESAR ecosystem while emphasizing the unique economic features of the token.*
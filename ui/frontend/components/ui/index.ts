// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// ---------------------------------------------------------------------------
// Barrel export — single entry-point for all ui primitives and composites
// ---------------------------------------------------------------------------

// Primitives (shadcn-style)
export { Alert, AlertTitle, AlertDescription } from './alert';
export { Badge, badgeVariants } from './badge';
export { Button, buttonVariants } from './button';
export {
  Card,
  CardHeader,
  CardFooter,
  CardTitle,
  CardAction,
  CardDescription,
  CardContent,
} from './card';
export { Collapsible, CollapsibleTrigger, CollapsibleContent } from './collapsible';
export {
  DropdownMenu,
  DropdownMenuPortal,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuItem,
  DropdownMenuCheckboxItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
} from './dropdown-menu';
export { Input } from './input';
export { Label } from './label';
export { Progress } from './progress';
export {
  Select,
  SelectGroup,
  SelectValue,
  SelectTrigger,
  SelectContent,
  SelectLabel,
  SelectItem,
  SelectSeparator,
  SelectScrollUpButton,
  SelectScrollDownButton,
} from './select';
export { Separator } from './separator';
export { Skeleton } from './skeleton';
export { Slider } from './slider';
export { Switch } from './switch';
export { Tabs, TabsList, TabsTrigger, TabsContent } from './tabs';
export {
  type ToastProps,
  type ToastActionElement,
  ToastProvider,
  ToastViewport,
  Toast,
  ToastTitle,
  ToastDescription,
  ToastClose,
  ToastAction,
} from './toast';
export { Toaster } from './toaster';
export { useToast, toast } from './use-toast';

// Charts (re-export the sub-barrel)
export {
  AreaChart,
  BarChart,
  LineChart,
  PieChart,
  SparklineChart,
  GaugeChart,
  ChartContainer,
  CHART_THEMES,
  CHART_CONFIGS,
} from './charts';
export type { ChartTheme } from './charts';

// Composites
export { MetricCard } from './MetricCard';
export { ModuleCard } from './ModuleCard';
export { ModuleHeader } from './ModuleHeader';
export { NetworkStatus } from './NetworkStatus';
export { StatusIndicator } from './StatusIndicator';
export { LoadingSpinner } from './LoadingSpinner';
export { DataCard } from './DataCard';
export { ProgressMetric } from './ProgressMetric';
export { AnimatedValue } from './AnimatedValue';
export { ActivityItem } from './ActivityItem';
export { FlowIndicator } from './FlowIndicator';
export { FeatureList } from './FeatureList';
export { ModuleConnections } from './ModuleConnections';

// Navigation
export { Breadcrumbs } from './Breadcrumbs';
export { TabNavigation } from './TabNavigation';
export { NavigationElement } from './NavigationElement';
export { NavigationHints } from './NavigationHints';

// Accessibility
export { AccessibilityWrapper } from './AccessibilityWrapper';
export { ScreenReaderOnly } from './ScreenReaderOnly';
export { LiveRegion } from './LiveRegion';
export { KeyboardNavigationProvider, useKeyboardNavigation } from './KeyboardNavigationProvider';

// Error handling & loading
export { ErrorBoundary } from './ErrorBoundary';
export { ModuleLoading } from './ModuleLoading';

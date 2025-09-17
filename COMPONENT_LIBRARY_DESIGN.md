# Web3 Ecosystem - Component Library & Design System

## 🎯 **Overview**

Comprehensive component library structure designed for rapid development across five interfaces while maintaining consistency, accessibility, and technical sophistication required for the Web3 ecosystem.

**Key Requirements**:
- **Reusability**: Components work across all interfaces
- **Performance**: < 100ms real-time updates, minimal bundle size
- **Accessibility**: WCAG 2.1 AA compliance for technical users
- **Byzantine Awareness**: Built-in security state visualization
- **IPv6 Native**: All networking components assume IPv6-only environment

---

## 🏗️ **Component Architecture**

### **Foundational Layer**

```typescript
// Core Design System Structure
src/
├── tokens/                    # Design tokens and theme configuration
│   ├── colors.ts             # Color palette with Byzantine states
│   ├── typography.ts         # Font scales and technical readability
│   ├── spacing.ts            # Consistent spacing system
│   ├── animation.ts          # Real-time update animations
│   └── breakpoints.ts        # Responsive design breakpoints
├── primitives/               # Basic building blocks
│   ├── Button/               # Interactive elements
│   ├── Input/                # Form controls
│   ├── Card/                 # Content containers
│   ├── Badge/                # Status indicators
│   └── Icon/                 # Iconography system
├── patterns/                 # Complex UI patterns
│   ├── Layout/               # App shell, navigation, grids
│   ├── Data/                 # Tables, charts, metrics
│   ├── Feedback/             # Alerts, notifications, loading
│   ├── Forms/                # Complex form patterns
│   └── Navigation/           # Multi-level navigation systems
├── domain/                   # Web3-specific components
│   ├── Consensus/            # Four-proof visualization
│   ├── Network/              # Node topology, Byzantine detection
│   ├── Assets/               # Resource management
│   ├── Crypto/               # Addresses, certificates, tokens
│   └── Performance/          # Real-time metrics
└── integration/              # Cross-interface coordination
    ├── RealTime/             # WebSocket data components
    ├── Auth/                 # Certificate-based authentication
    ├── Routing/              # IPv6-aware navigation
    └── State/                # Global state management
```

---

## 🎨 **Design Token System**

### **Color System with Byzantine States**

```typescript
// colors.ts - Comprehensive color system
export const colorTokens = {
  // Primary Brand Colors
  primary: {
    50: '#eef2ff',   // Lightest backgrounds
    100: '#e0e7ff',  // Light backgrounds
    500: '#6366f1',  // Primary brand color
    600: '#4f46e5',  // Primary interactions
    700: '#4338ca',  // Primary hover states
    900: '#312e81',  // Dark contexts
  },

  // Byzantine Security States
  byzantine: {
    trusted: {
      50: '#f0fdf4',   // Light green backgrounds
      500: '#10b981',  // Trusted nodes, verified states
      600: '#059669',  // Strong trust indicators
      900: '#064e3b',  // Dark trusted contexts
    },
    suspected: {
      50: '#fffbeb',   // Light amber backgrounds
      500: '#f59e0b',  // Warning states, suspected activity
      600: '#d97706',  // Strong warning indicators
      900: '#92400e',  // Dark warning contexts
    },
    malicious: {
      50: '#fef2f2',   // Light red backgrounds
      500: '#ef4444',  // Confirmed Byzantine behavior
      600: '#dc2626',  // Critical security alerts
      900: '#7f1d1d',  // Dark error contexts
    },
  },

  // Consensus States
  consensus: {
    pending: {
      500: '#6b7280',  // Proof validation in progress
      600: '#4b5563',  // Secondary pending states
    },
    validated: {
      500: '#10b981',  // Four-proof completion
      600: '#059669',  // Strong validation confirmation
    },
    failed: {
      500: '#ef4444',  // Validation failure
      600: '#dc2626',  // Critical validation errors
    },
  },

  // Performance Metrics
  performance: {
    excellent: '#10b981',  // >90% of target performance
    good: '#3b82f6',       // 70-90% of target
    warning: '#f59e0b',    // 50-70% of target
    critical: '#ef4444',   // <50% of target
  },

  // Technical Interface
  technical: {
    code: '#1e293b',       // Code backgrounds
    hash: '#64748b',       // Address/hash colors
    ipv6: '#3b82f6',       // IPv6 address highlighting
    certificate: '#8b5cf6', // Certificate-related elements
  },

  // Neutral Grays
  slate: {
    50: '#f8fafc',   // Lightest backgrounds
    100: '#f1f5f9',  // Light section backgrounds
    200: '#e2e8f0',  // Borders and dividers
    300: '#cbd5e1',  // Subtle borders
    400: '#94a3b8',  // Placeholder text
    500: '#64748b',  // Secondary text
    600: '#475569',  // Primary text on light
    700: '#334155',  // Headings on light
    800: '#1e293b',  // Strong text, dark backgrounds
    900: '#0f172a',  // Highest contrast text
  },
};

// Semantic color mapping
export const semanticColors = {
  // Interface background colors
  background: {
    primary: colorTokens.slate[50],     // Main app background
    secondary: colorTokens.slate[100],   // Card backgrounds
    tertiary: colorTokens.slate[200],    // Section dividers
    dark: colorTokens.slate[900],        // Dark mode primary
  },

  // Text colors with high contrast
  text: {
    primary: colorTokens.slate[900],     // Primary text (dark)
    secondary: colorTokens.slate[600],   // Secondary text
    tertiary: colorTokens.slate[400],    // Tertiary text
    inverse: colorTokens.slate[50],      // Text on dark backgrounds
    code: colorTokens.technical.code,    // Monospace code text
  },

  // Interactive element colors
  interactive: {
    primary: colorTokens.primary[600],   // Primary buttons, links
    primaryHover: colorTokens.primary[700], // Primary hover states
    secondary: colorTokens.slate[200],   // Secondary buttons
    danger: colorTokens.byzantine.malicious[500], // Destructive actions
    success: colorTokens.byzantine.trusted[500],  // Success actions
  },

  // Status and feedback colors
  status: {
    success: colorTokens.byzantine.trusted[500],
    warning: colorTokens.byzantine.suspected[500],
    error: colorTokens.byzantine.malicious[500],
    info: colorTokens.primary[500],
    neutral: colorTokens.slate[400],
  },
};
```

### **Typography System for Technical Content**

```typescript
// typography.ts - Technical-optimized typography
export const typographyTokens = {
  // Font families
  fontFamily: {
    sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'sans-serif'],
    mono: ['JetBrains Mono', 'Monaco', 'Cascadia Code', 'Fira Code', 'monospace'],
    display: ['Inter Display', 'Inter', 'sans-serif'],
  },

  // Font sizes with technical readability focus
  fontSize: {
    xs: '0.75rem',    // 12px - Small labels, metadata
    sm: '0.875rem',   // 14px - Secondary text, captions
    base: '1rem',     // 16px - Body text, primary content
    lg: '1.125rem',   // 18px - Large body text
    xl: '1.25rem',    // 20px - Small headings
    '2xl': '1.5rem',  // 24px - Medium headings
    '3xl': '1.875rem', // 30px - Large headings
    '4xl': '2.25rem', // 36px - Display headings
  },

  // Line heights optimized for technical content
  lineHeight: {
    none: '1',        // Tight technical displays
    tight: '1.25',    // Code, addresses, hashes
    snug: '1.375',    // Labels, buttons
    normal: '1.5',    // Body text, readable content
    relaxed: '1.625', // Long-form technical documentation
    loose: '2',       // Spacious layouts
  },

  // Font weights
  fontWeight: {
    thin: '100',
    light: '300',
    normal: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
    extrabold: '800',
    black: '900',
  },

  // Letter spacing for technical precision
  letterSpacing: {
    tighter: '-0.05em',  // Compact displays
    tight: '-0.025em',   // Headlines
    normal: '0',         // Body text
    wide: '0.025em',     // Technical labels
    wider: '0.05em',     // Emphasis
    widest: '0.1em',     // Strong emphasis
  },
};

// Semantic typography mapping
export const textStyles = {
  // Headings
  heading: {
    h1: {
      fontSize: typographyTokens.fontSize['4xl'],
      fontWeight: typographyTokens.fontWeight.bold,
      lineHeight: typographyTokens.lineHeight.tight,
      letterSpacing: typographyTokens.letterSpacing.tight,
    },
    h2: {
      fontSize: typographyTokens.fontSize['3xl'],
      fontWeight: typographyTokens.fontWeight.semibold,
      lineHeight: typographyTokens.lineHeight.snug,
    },
    h3: {
      fontSize: typographyTokens.fontSize['2xl'],
      fontWeight: typographyTokens.fontWeight.medium,
      lineHeight: typographyTokens.lineHeight.snug,
    },
  },

  // Body text variants
  body: {
    large: {
      fontSize: typographyTokens.fontSize.lg,
      lineHeight: typographyTokens.lineHeight.normal,
    },
    normal: {
      fontSize: typographyTokens.fontSize.base,
      lineHeight: typographyTokens.lineHeight.normal,
    },
    small: {
      fontSize: typographyTokens.fontSize.sm,
      lineHeight: typographyTokens.lineHeight.normal,
    },
  },

  // Technical content styles
  technical: {
    code: {
      fontFamily: typographyTokens.fontFamily.mono,
      fontSize: typographyTokens.fontSize.sm,
      lineHeight: typographyTokens.lineHeight.tight,
      letterSpacing: typographyTokens.letterSpacing.wide,
    },
    address: {
      fontFamily: typographyTokens.fontFamily.mono,
      fontSize: typographyTokens.fontSize.sm,
      lineHeight: typographyTokens.lineHeight.none,
      letterSpacing: typographyTokens.letterSpacing.wider,
      wordBreak: 'break-all',
    },
    hash: {
      fontFamily: typographyTokens.fontFamily.mono,
      fontSize: typographyTokens.fontSize.xs,
      lineHeight: typographyTokens.lineHeight.tight,
      letterSpacing: typographyTokens.letterSpacing.widest,
      textTransform: 'uppercase',
    },
  },

  // Interface labels
  label: {
    default: {
      fontSize: typographyTokens.fontSize.sm,
      fontWeight: typographyTokens.fontWeight.medium,
      letterSpacing: typographyTokens.letterSpacing.wide,
    },
    small: {
      fontSize: typographyTokens.fontSize.xs,
      fontWeight: typographyTokens.fontWeight.medium,
      letterSpacing: typographyTokens.letterSpacing.wider,
      textTransform: 'uppercase',
    },
  },
};
```

---

## 🧱 **Core Component Library**

### **1. Primitive Components**

#### **Button Component**

```typescript
// Button/Button.tsx - Accessible button with Byzantine states
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'ghost';
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  byzantineState?: 'trusted' | 'suspected' | 'malicious';
  loading?: boolean;
  disabled?: boolean;
  icon?: ReactNode;
  iconPosition?: 'left' | 'right';
  fullWidth?: boolean;
  children: ReactNode;
  onClick?: () => void;
}

const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  size = 'md',
  byzantineState,
  loading = false,
  disabled = false,
  icon,
  iconPosition = 'left',
  fullWidth = false,
  children,
  onClick,
  ...props
}) => {
  const baseClasses = [
    'inline-flex items-center justify-center',
    'font-medium rounded-lg transition-colors',
    'focus:outline-none focus:ring-2 focus:ring-offset-2',
    'disabled:opacity-50 disabled:cursor-not-allowed',
  ];

  const variantClasses = {
    primary: 'bg-primary-600 text-white hover:bg-primary-700 focus:ring-primary-500',
    secondary: 'bg-slate-200 text-slate-900 hover:bg-slate-300 focus:ring-slate-500',
    danger: 'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500',
    success: 'bg-emerald-600 text-white hover:bg-emerald-700 focus:ring-emerald-500',
    ghost: 'text-slate-600 hover:bg-slate-100 focus:ring-slate-500',
  };

  const sizeClasses = {
    xs: 'px-2 py-1 text-xs',
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-2.5 text-base',
    xl: 'px-8 py-3 text-base',
  };

  const byzantineClasses = byzantineState ? {
    trusted: 'ring-2 ring-emerald-500',
    suspected: 'ring-2 ring-amber-500',
    malicious: 'ring-2 ring-red-500',
  }[byzantineState] : '';

  return (
    <button
      className={cn(
        baseClasses,
        variantClasses[variant],
        sizeClasses[size],
        byzantineClasses,
        fullWidth && 'w-full',
        loading && 'cursor-wait'
      )}
      disabled={disabled || loading}
      onClick={onClick}
      aria-label={typeof children === 'string' ? children : undefined}
      {...props}
    >
      {loading && <Spinner className="w-4 h-4 mr-2" />}
      {icon && iconPosition === 'left' && (
        <span className="mr-2">{icon}</span>
      )}
      {children}
      {icon && iconPosition === 'right' && (
        <span className="ml-2">{icon}</span>
      )}
    </button>
  );
};
```

#### **Badge Component for Status Indication**

```typescript
// Badge/Badge.tsx - Status badges with Byzantine awareness
interface BadgeProps {
  variant?: 'default' | 'success' | 'warning' | 'danger' | 'info';
  size?: 'xs' | 'sm' | 'md' | 'lg';
  consensusState?: 'pending' | 'validated' | 'failed';
  pulse?: boolean;
  dot?: boolean;
  children: ReactNode;
}

const Badge: React.FC<BadgeProps> = ({
  variant = 'default',
  size = 'sm',
  consensusState,
  pulse = false,
  dot = false,
  children,
}) => {
  const baseClasses = [
    'inline-flex items-center font-medium rounded-full',
    'whitespace-nowrap',
  ];

  const variantClasses = {
    default: 'bg-slate-100 text-slate-800',
    success: 'bg-emerald-100 text-emerald-800',
    warning: 'bg-amber-100 text-amber-800',
    danger: 'bg-red-100 text-red-800',
    info: 'bg-blue-100 text-blue-800',
  };

  const sizeClasses = {
    xs: 'px-2 py-0.5 text-xs',
    sm: 'px-2.5 py-0.5 text-xs',
    md: 'px-3 py-1 text-sm',
    lg: 'px-4 py-1.5 text-sm',
  };

  const consensusClasses = consensusState ? {
    pending: 'bg-slate-100 text-slate-800 animate-pulse',
    validated: 'bg-emerald-100 text-emerald-800',
    failed: 'bg-red-100 text-red-800 animate-pulse',
  }[consensusState] : '';

  return (
    <span
      className={cn(
        baseClasses,
        consensusState ? consensusClasses : variantClasses[variant],
        sizeClasses[size],
        pulse && 'animate-pulse'
      )}
      role="status"
      aria-label={`Status: ${children}`}
    >
      {dot && (
        <span
          className={cn(
            'w-1.5 h-1.5 rounded-full mr-1.5',
            consensusState ? {
              pending: 'bg-slate-400',
              validated: 'bg-emerald-400',
              failed: 'bg-red-400',
            }[consensusState] : 'bg-current'
          )}
        />
      )}
      {children}
    </span>
  );
};
```

### **2. Pattern Components**

#### **MetricsPanel - Real-time Performance Display**

```typescript
// MetricsPanel/MetricsPanel.tsx - Performance metrics with real-time updates
interface Metric {
  id: string;
  label: string;
  value: string | number;
  unit?: string;
  target?: number;
  trend?: 'up' | 'down' | 'stable';
  status?: 'excellent' | 'good' | 'warning' | 'critical';
  lastUpdated: Date;
}

interface MetricsPanelProps {
  title: string;
  metrics: Metric[];
  refreshInterval?: number; // ms
  compact?: boolean;
  showTrends?: boolean;
  onMetricClick?: (metric: Metric) => void;
}

const MetricsPanel: React.FC<MetricsPanelProps> = ({
  title,
  metrics,
  refreshInterval = 2000,
  compact = false,
  showTrends = true,
  onMetricClick,
}) => {
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  
  // Real-time update simulation
  useEffect(() => {
    if (refreshInterval > 0) {
      const interval = setInterval(() => {
        setLastUpdate(new Date());
      }, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [refreshInterval]);

  const formatValue = (metric: Metric): string => {
    if (typeof metric.value === 'number') {
      return `${metric.value.toLocaleString()}${metric.unit || ''}`;
    }
    return metric.value;
  };

  const getStatusColor = (status?: string): string => {
    switch (status) {
      case 'excellent': return 'text-emerald-600';
      case 'good': return 'text-blue-600';
      case 'warning': return 'text-amber-600';
      case 'critical': return 'text-red-600';
      default: return 'text-slate-600';
    }
  };

  const getTrendIcon = (trend?: string): ReactNode => {
    switch (trend) {
      case 'up': return <ArrowTrendingUpIcon className="w-4 h-4 text-emerald-500" />;
      case 'down': return <ArrowTrendingDownIcon className="w-4 h-4 text-red-500" />;
      case 'stable': return <MinusIcon className="w-4 h-4 text-slate-400" />;
      default: return null;
    }
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className={compact ? 'text-sm' : 'text-base'}>
          {title}
        </CardTitle>
        <Badge variant="info" size="xs">
          Updated {formatDistanceToNow(lastUpdate)} ago
        </Badge>
      </CardHeader>
      <CardContent>
        <div className={cn(
          'grid gap-4',
          compact ? 'grid-cols-2' : 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-3'
        )}>
          {metrics.map((metric) => (
            <div
              key={metric.id}
              className={cn(
                'p-3 rounded-lg border cursor-pointer',
                'hover:bg-slate-50 transition-colors',
                onMetricClick && 'cursor-pointer'
              )}
              onClick={() => onMetricClick?.(metric)}
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-slate-600">
                  {metric.label}
                </span>
                {showTrends && getTrendIcon(metric.trend)}
              </div>
              <div className="mt-1">
                <span className={cn(
                  'text-2xl font-bold',
                  getStatusColor(metric.status)
                )}>
                  {formatValue(metric)}
                </span>
                {metric.target && (
                  <div className="mt-1">
                    <div className="flex justify-between text-xs text-slate-500 mb-1">
                      <span>Target: {metric.target}{metric.unit}</span>
                      <span>
                        {Math.round(((metric.value as number) / metric.target) * 100)}%
                      </span>
                    </div>
                    <div className="w-full bg-slate-200 rounded-full h-1">
                      <div
                        className={cn(
                          'h-1 rounded-full transition-all duration-300',
                          metric.status === 'excellent' && 'bg-emerald-500',
                          metric.status === 'good' && 'bg-blue-500',
                          metric.status === 'warning' && 'bg-amber-500',
                          metric.status === 'critical' && 'bg-red-500'
                        )}
                        style={{
                          width: `${Math.min(((metric.value as number) / metric.target) * 100, 100)}%`
                        }}
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
};
```

### **3. Domain-Specific Components**

#### **NetworkTopology - Interactive Node Visualization**

```typescript
// NetworkTopology/NetworkTopology.tsx - Interactive network visualization
interface NetworkNode {
  id: string;
  ipv6Address: string;
  status: 'healthy' | 'suspected' | 'byzantine' | 'offline';
  trustScore: number;
  position: { x: number; y: number };
  connections: string[]; // Connected node IDs
  lastSeen: Date;
  consensusParticipation: number; // 0-100%
}

interface NetworkTopologyProps {
  nodes: NetworkNode[];
  selectedNode?: string;
  onNodeSelect?: (nodeId: string) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  showConnections?: boolean;
  showLabels?: boolean;
  interactive?: boolean;
  height?: number;
}

const NetworkTopology: React.FC<NetworkTopologyProps> = ({
  nodes,
  selectedNode,
  onNodeSelect,
  onNodeDoubleClick,
  showConnections = true,
  showLabels = true,
  interactive = true,
  height = 400,
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const [viewBox, setViewBox] = useState('0 0 800 400');

  const getNodeColor = (status: string): string => {
    switch (status) {
      case 'healthy': return '#10b981'; // emerald-500
      case 'suspected': return '#f59e0b'; // amber-500  
      case 'byzantine': return '#ef4444'; // red-500
      case 'offline': return '#6b7280'; // slate-500
      default: return '#6b7280';
    }
  };

  const getNodeSize = (trustScore: number): number => {
    return 8 + (trustScore / 100) * 12; // 8-20px radius based on trust
  };

  const handleNodeClick = (nodeId: string) => {
    if (interactive && onNodeSelect) {
      onNodeSelect(nodeId);
    }
  };

  const handleNodeDoubleClick = (nodeId: string) => {
    if (interactive && onNodeDoubleClick) {
      onNodeDoubleClick(nodeId);
    }
  };

  const renderConnection = (fromNode: NetworkNode, toNodeId: string) => {
    const toNode = nodes.find(n => n.id === toNodeId);
    if (!toNode) return null;

    const connectionColor = fromNode.status === 'byzantine' || toNode.status === 'byzantine'
      ? '#ef4444'  // red for Byzantine connections
      : '#cbd5e1'; // slate-300 for normal connections

    return (
      <line
        key={`${fromNode.id}-${toNodeId}`}
        x1={fromNode.position.x}
        y1={fromNode.position.y}
        x2={toNode.position.x}
        y2={toNode.position.y}
        stroke={connectionColor}
        strokeWidth={fromNode.status === 'byzantine' ? 2 : 1}
        strokeDasharray={fromNode.status === 'byzantine' ? '5,5' : 'none'}
        opacity={0.6}
      />
    );
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          Network Topology
          <div className="flex items-center space-x-2 text-sm">
            <Badge variant="success" size="xs">
              {nodes.filter(n => n.status === 'healthy').length} Healthy
            </Badge>
            <Badge variant="warning" size="xs">
              {nodes.filter(n => n.status === 'suspected').length} Suspected
            </Badge>
            <Badge variant="danger" size="xs">
              {nodes.filter(n => n.status === 'byzantine').length} Byzantine
            </Badge>
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="relative">
          <svg
            ref={svgRef}
            width="100%"
            height={height}
            viewBox={viewBox}
            className="border rounded-lg bg-slate-50"
          >
            {/* Connection lines */}
            {showConnections && (
              <g className="connections">
                {nodes.map(node => 
                  node.connections.map(connectionId => 
                    renderConnection(node, connectionId)
                  )
                )}
              </g>
            )}

            {/* Nodes */}
            <g className="nodes">
              {nodes.map(node => (
                <g key={node.id}>
                  {/* Node circle */}
                  <circle
                    cx={node.position.x}
                    cy={node.position.y}
                    r={getNodeSize(node.trustScore)}
                    fill={getNodeColor(node.status)}
                    stroke={selectedNode === node.id ? '#4f46e5' : '#ffffff'}
                    strokeWidth={selectedNode === node.id ? 3 : 2}
                    className={cn(
                      interactive && 'cursor-pointer hover:opacity-80',
                      node.status === 'byzantine' && 'animate-pulse'
                    )}
                    onClick={() => handleNodeClick(node.id)}
                    onDoubleClick={() => handleNodeDoubleClick(node.id)}
                    onMouseEnter={() => setHoveredNode(node.id)}
                    onMouseLeave={() => setHoveredNode(null)}
                  />
                  
                  {/* Node label */}
                  {showLabels && (
                    <text
                      x={node.position.x}
                      y={node.position.y + getNodeSize(node.trustScore) + 16}
                      textAnchor="middle"
                      className="fill-slate-600 text-xs font-mono"
                      fontSize="10"
                    >
                      {node.id}
                    </text>
                  )}
                </g>
              ))}
            </g>
          </svg>

          {/* Node details tooltip */}
          {hoveredNode && (
            <NodeTooltip
              node={nodes.find(n => n.id === hoveredNode)!}
              position={{ x: 10, y: 10 }} // Position relative to cursor
            />
          )}
        </div>

        {/* Legend */}
        <div className="flex items-center justify-center space-x-6 mt-4 text-sm">
          <div className="flex items-center space-x-2">
            <div className="w-3 h-3 rounded-full bg-emerald-500" />
            <span>Healthy</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-3 h-3 rounded-full bg-amber-500" />
            <span>Suspected</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-3 h-3 rounded-full bg-red-500 animate-pulse" />
            <span>Byzantine</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-3 h-3 rounded-full bg-slate-500" />
            <span>Offline</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
```

#### **ConsensusFourProofPanel - Real-time Four-Proof Validation**

```typescript
// ConsensusFourProofPanel/ConsensusFourProofPanel.tsx
interface ProofState {
  type: 'PoSpace' | 'PoStake' | 'PoWork' | 'PoTime';
  status: 'pending' | 'validating' | 'validated' | 'failed';
  progress?: number; // 0-100 for validating state
  details: {
    location?: string;      // PoSpace: WHERE
    ownership?: string;     // PoStake: WHO  
    computation?: string;   // PoWork: WHAT/HOW
    timestamp?: Date;       // PoTime: WHEN
  };
  lastUpdated: Date;
}

interface ConsensusFourProofPanelProps {
  proofs: ProofState[];
  currentBlock?: number;
  consensusThreshold?: number; // Percentage required for consensus
  participatingNodes?: number;
  onProofClick?: (proof: ProofState) => void;
  showDetails?: boolean;
}

const ConsensusFourProofPanel: React.FC<ConsensusFourProofPanelProps> = ({
  proofs,
  currentBlock,
  consensusThreshold = 67,
  participatingNodes,
  onProofClick,
  showDetails = true,
}) => {
  const getProofIcon = (type: string): ReactNode => {
    switch (type) {
      case 'PoSpace': return <MapPinIcon className="w-5 h-5" />; // WHERE
      case 'PoStake': return <KeyIcon className="w-5 h-5" />;      // WHO
      case 'PoWork': return <CpuChipIcon className="w-5 h-5" />;   // WHAT/HOW
      case 'PoTime': return <ClockIcon className="w-5 h-5" />;     // WHEN
      default: return <QuestionMarkCircleIcon className="w-5 h-5" />;
    }
  };

  const getStatusBadge = (status: string): ReactNode => {
    switch (status) {
      case 'pending':
        return <Badge variant="default" size="xs">Pending</Badge>;
      case 'validating':
        return <Badge variant="warning" size="xs" pulse>Validating</Badge>;
      case 'validated':
        return <Badge variant="success" size="xs">✓ Validated</Badge>;
      case 'failed':
        return <Badge variant="danger" size="xs">✗ Failed</Badge>;
      default:
        return <Badge variant="default" size="xs">Unknown</Badge>;
    }
  };

  const getProofDescription = (proof: ProofState): string => {
    switch (proof.type) {
      case 'PoSpace': return `WHERE: Storage location and network positioning`;
      case 'PoStake': return `WHO: Ownership rights and economic commitment`;  
      case 'PoWork': return `WHAT/HOW: Computational resources and processing`;
      case 'PoTime': return `WHEN: Temporal ordering and timestamp validation`;
      default: return '';
    }
  };

  const validatedProofs = proofs.filter(p => p.status === 'validated').length;
  const overallProgress = (validatedProofs / proofs.length) * 100;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center space-x-2">
            <ShieldCheckIcon className="w-5 h-5" />
            <span>Four-Proof Consensus</span>
          </CardTitle>
          <div className="flex items-center space-x-2 text-sm">
            {currentBlock && (
              <Badge variant="info" size="xs">
                Block #{currentBlock}
              </Badge>
            )}
            <Badge 
              variant={validatedProofs === 4 ? "success" : "warning"} 
              size="xs"
            >
              {validatedProofs}/4 Validated
            </Badge>
          </div>
        </div>
        
        {/* Overall progress */}
        <div className="mt-3">
          <div className="flex justify-between text-sm text-slate-600 mb-1">
            <span>Consensus Progress</span>
            <span>{Math.round(overallProgress)}%</span>
          </div>
          <div className="w-full bg-slate-200 rounded-full h-2">
            <div
              className={cn(
                'h-2 rounded-full transition-all duration-300',
                overallProgress === 100 
                  ? 'bg-emerald-500' 
                  : 'bg-blue-500'
              )}
              style={{ width: `${overallProgress}%` }}
            />
          </div>
        </div>
      </CardHeader>
      
      <CardContent>
        <div className="space-y-4">
          {proofs.map((proof) => (
            <div
              key={proof.type}
              className={cn(
                'p-4 rounded-lg border transition-colors',
                onProofClick && 'cursor-pointer hover:bg-slate-50',
                proof.status === 'validated' && 'border-emerald-200 bg-emerald-50',
                proof.status === 'failed' && 'border-red-200 bg-red-50'
              )}
              onClick={() => onProofClick?.(proof)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <div className={cn(
                    'p-2 rounded-full',
                    proof.status === 'validated' && 'bg-emerald-100 text-emerald-600',
                    proof.status === 'validating' && 'bg-amber-100 text-amber-600',
                    proof.status === 'failed' && 'bg-red-100 text-red-600',
                    proof.status === 'pending' && 'bg-slate-100 text-slate-600'
                  )}>
                    {getProofIcon(proof.type)}
                  </div>
                  <div>
                    <h4 className="font-medium text-slate-900">
                      {proof.type}
                    </h4>
                    <p className="text-sm text-slate-600">
                      {getProofDescription(proof)}
                    </p>
                  </div>
                </div>
                
                <div className="flex items-center space-x-2">
                  {proof.status === 'validating' && proof.progress !== undefined && (
                    <div className="flex items-center space-x-2">
                      <span className="text-sm text-slate-600">
                        {proof.progress}%
                      </span>
                      <div className="w-16 bg-slate-200 rounded-full h-1">
                        <div
                          className="h-1 bg-amber-500 rounded-full transition-all duration-300"
                          style={{ width: `${proof.progress}%` }}
                        />
                      </div>
                    </div>
                  )}
                  {getStatusBadge(proof.status)}
                </div>
              </div>

              {/* Proof details */}
              {showDetails && proof.status !== 'pending' && (
                <div className="mt-3 pl-11 text-sm text-slate-600">
                  {proof.details.location && (
                    <div>Location: {proof.details.location}</div>
                  )}
                  {proof.details.ownership && (
                    <div>Stake: {proof.details.ownership}</div>
                  )}
                  {proof.details.computation && (
                    <div>Computation: {proof.details.computation}</div>
                  )}
                  {proof.details.timestamp && (
                    <div>
                      Timestamp: {formatDistanceToNow(proof.details.timestamp)} ago
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Consensus summary */}
        {participatingNodes && (
          <div className="mt-6 p-3 bg-slate-50 rounded-lg">
            <div className="flex items-center justify-between text-sm">
              <span className="text-slate-600">Network Consensus</span>
              <span className="font-medium">
                {participatingNodes} nodes participating
              </span>
            </div>
            <div className="flex items-center justify-between text-sm mt-1">
              <span className="text-slate-600">
                Byzantine Fault Tolerance
              </span>
              <span className="font-medium">
                {consensusThreshold}% threshold (≥{Math.ceil(participatingNodes * consensusThreshold / 100)} nodes)
              </span>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
```

---

## ♿ **Accessibility Guidelines**

### **WCAG 2.1 AA Compliance Requirements**

**Color & Contrast**:
- **Text Contrast**: Minimum 4.5:1 for normal text, 3:1 for large text
- **Non-text Contrast**: Minimum 3:1 for UI components and graphical objects
- **Color Independence**: Never rely solely on color to convey information

**Keyboard Navigation**:
- **Focus Management**: Visible focus indicators on all interactive elements
- **Tab Order**: Logical tab sequence through interface components
- **Keyboard Shortcuts**: Essential functions accessible via keyboard
- **Escape Routes**: Easy way to exit modal dialogs and complex workflows

**Screen Reader Support**:
- **Semantic HTML**: Proper heading hierarchy and landmark roles
- **ARIA Labels**: Descriptive labels for complex components
- **Live Regions**: Announce real-time updates to screen readers
- **Alternative Text**: Meaningful descriptions for data visualizations

### **Technical User Accommodations**

**Information Density Options**:
```typescript
// Accessibility configuration options
interface AccessibilityConfig {
  informationDensity: 'compact' | 'comfortable' | 'spacious';
  fontSize: 'small' | 'medium' | 'large';
  colorBlindSupport: boolean;
  motionPreferences: 'no-preference' | 'reduced';
  contrastMode: 'normal' | 'high';
}

// Responsive text scaling
const textScaling = {
  small: '0.875rem',   // 14px base
  medium: '1rem',      // 16px base (default)
  large: '1.125rem',   // 18px base
};

// High contrast mode colors
const highContrastColors = {
  text: '#000000',           // Pure black text
  background: '#ffffff',     // Pure white background
  primary: '#0000ff',        // Pure blue for links
  success: '#008000',        // Pure green for success
  warning: '#ff8c00',        // Orange for warnings
  danger: '#ff0000',         // Pure red for errors
};
```

**Screen Reader Optimizations**:
```typescript
// Screen reader friendly component patterns
const NetworkTopologyAccessible = () => {
  return (
    <div role="img" aria-labelledby="topology-title" aria-describedby="topology-desc">
      <h3 id="topology-title">Network Topology</h3>
      <p id="topology-desc">
        Interactive network diagram showing {nodes.length} nodes. 
        {healthyNodes} healthy nodes, {byzantineNodes} Byzantine nodes detected.
      </p>
      
      {/* Screen reader alternative to visual network */}
      <div className="sr-only">
        <h4>Node Status List</h4>
        <ul>
          {nodes.map(node => (
            <li key={node.id}>
              Node {node.id}: {node.status}, 
              Trust Score: {node.trustScore}/100,
              Last seen: {formatDistanceToNow(node.lastSeen)} ago
            </li>
          ))}
        </ul>
      </div>

      {/* Visual network for sighted users */}
      <div aria-hidden="true">
        <NetworkVisualization nodes={nodes} />
      </div>
    </div>
  );
};
```

**Real-time Update Announcements**:
```typescript
// Live region for consensus updates
const ConsensusLiveRegion = ({ latestUpdate }: { latestUpdate: string }) => {
  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="sr-only"
    >
      {latestUpdate}
    </div>
  );
};

// Usage in consensus monitoring
const useConsensusAnnouncements = () => {
  const [announcement, setAnnouncement] = useState('');

  useEffect(() => {
    // Announce significant consensus events
    const announceUpdate = (update: ConsensusUpdate) => {
      if (update.type === 'block_validated') {
        setAnnouncement(`Block ${update.blockNumber} validated successfully`);
      } else if (update.type === 'byzantine_detected') {
        setAnnouncement(`Byzantine behavior detected on node ${update.nodeId}`);
      }
    };

    // Subscribe to consensus updates
    consensusService.subscribe(announceUpdate);
  }, []);

  return announcement;
};
```

### **Error Handling & Recovery**

**Graceful Degradation**:
- **Network Failures**: Offline-first approach with cached data
- **Certificate Expiry**: Clear warnings with renewal instructions
- **Byzantine Faults**: Safe mode operation with reduced functionality
- **Performance Issues**: Automatic fallback to simplified interfaces

**User Feedback Patterns**:
```typescript
// Comprehensive error boundary with recovery options
interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
  errorInfo?: ErrorInfo;
  retryCount: number;
}

class Web3ErrorBoundary extends Component<PropsWithChildren, ErrorBoundaryState> {
  constructor(props: PropsWithChildren) {
    super(props);
    this.state = {
      hasError: false,
      retryCount: 0,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({ errorInfo });
    
    // Log to error tracking service
    errorTrackingService.captureException(error, {
      tags: { component: 'Web3ErrorBoundary' },
      extra: errorInfo,
    });
  }

  handleRetry = () => {
    this.setState(prevState => ({
      hasError: false,
      error: undefined,
      errorInfo: undefined,
      retryCount: prevState.retryCount + 1,
    }));
  };

  render() {
    if (this.state.hasError) {
      return (
        <Card className="max-w-md mx-auto mt-8">
          <CardHeader>
            <CardTitle className="flex items-center text-red-600">
              <ExclamationTriangleIcon className="w-5 h-5 mr-2" />
              Something went wrong
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-slate-600 mb-4">
              An error occurred while loading this interface. This might be due to:
            </p>
            <ul className="list-disc list-inside text-sm text-slate-600 mb-4">
              <li>Network connectivity issues</li>
              <li>Certificate expiration</li>
              <li>Byzantine network conditions</li>
              <li>Temporary service unavailability</li>
            </ul>
            
            <div className="flex space-x-2">
              <Button onClick={this.handleRetry} size="sm">
                <ArrowPathIcon className="w-4 h-4 mr-2" />
                Retry ({this.state.retryCount})
              </Button>
              <Button 
                variant="secondary" 
                size="sm"
                onClick={() => window.location.reload()}
              >
                <ArrowTopRightOnSquareIcon className="w-4 h-4 mr-2" />
                Reload Page
              </Button>
            </div>

            {process.env.NODE_ENV === 'development' && this.state.error && (
              <details className="mt-4 p-2 bg-slate-100 rounded text-xs">
                <summary className="cursor-pointer font-mono">
                  Error Details (Development)
                </summary>
                <pre className="mt-2 whitespace-pre-wrap">
                  {this.state.error.toString()}
                  {this.state.errorInfo?.componentStack}
                </pre>
              </details>
            )}
          </CardContent>
        </Card>
      );
    }

    return this.props.children;
  }
}
```

This comprehensive component library provides the foundation for building accessible, performant, and maintainable interfaces across the Web3 ecosystem while maintaining consistency and technical sophistication.
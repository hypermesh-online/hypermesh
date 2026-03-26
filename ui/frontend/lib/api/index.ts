// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Web3 API Integration - Complete API client library
 *
 * Provides unified access to the Web3 ecosystem services:
 * - HTTP API client with get/post/put/del helpers (lib/api.ts)
 * - React Query integration hooks
 * - Type-safe service interfaces
 */

// Service APIs
export { trustChainAPI } from './services/TrustChainAPI';
export type {
  Certificate,
  DNSRecord,
  TrustHierarchy,
  RotationPolicy,
  ValidationResult
} from './services/TrustChainAPI';

export { hyperMeshAPI } from './services/HyperMeshAPI';
export type {
  Asset,
  AssetType,
  PrivacyLevel,
  AssetAllocation,
  FourProofStateVerification,
  StateProof,
  ProofType,
  ByzantineDetection,
  RemoteProxy,
  NodeHealth,
  VMAsset,
  VMExecution,
  CatalogApplication
} from './services/HyperMeshAPI';

export { stoqAPI } from './services/STOQAPI';
export type {
  QUICConnection,
  PerformanceMetrics,
  NetworkQuality,
  TransportOptimization,
  ConnectionPool,
  StreamAnalytics,
  STOQSystemHealth
} from './services/STOQAPI';

// React Query Hooks
export {
  useSystemStatus,
  useServiceStatus,
  usePerformanceMetrics as useSystemPerformanceMetrics
} from './hooks/useSystemStatus';
export type { SystemStatus, ServiceStatus } from './hooks/useSystemStatus';

export {
  useCertificates,
  useCertificate,
  useCreateCertificate,
  useRevokeCertificate,
  useValidateCertificate,
  useTrustHierarchy,
  useRotationPolicies,
  useCreateRotationPolicy,
  useUpdateRotationPolicy,
  useRotateCertificate,
  useRotationHistory,
  useExportCertificate,
  useImportCertificate
} from './hooks/useCertificates';

export {
  useAssets,
  useAsset,
  useCreateAsset,
  useUpdateAsset,
  useDeleteAsset,
  useRequestAllocation,
  useAllocations,
  useReleaseAllocation,
  useValidateStateProof,
  useStateProofHistory,
  useSubmitProof,
  useByzantineDetections,
  useReportByzantineBehavior,
  useRemoteProxies,
  useCreateRemoteProxy,
  useUpdateRemoteProxy,
  useValidateProxyTrust,
  useNodeHealth,
  useNetworkTopology,
  useExecuteRemoteOperation,
  // VM Asset Integration Hooks
  useCatalogApplications,
  useCreateVMAsset,
  useInstallCatalogApplication,
  useExecuteVMAsset,
  useVMExecutions,
  useVMExecution,
  useCancelVMExecution,
  useVMAssets,
  useUpdateVMAsset
} from './hooks/useAssets';

export {
  useQUICConnections,
  useQUICConnection,
  useCreateConnection,
  useCloseConnection,
  usePerformanceMetrics,
  useNetworkQuality,
  useTransportOptimizations,
  useApplyOptimization,
  useConnectionPools,
  useCreateConnectionPool,
  useStreamAnalytics,
  useHistoricalMetrics,
  useRunDiagnostics,
  useRunBenchmark,
  useBenchmarkResult,
  useUpdateTransportSettings,
  useTransportSettings
} from './hooks/usePerformanceMetrics';

// STOQ data hooks (API polling)
export {
  useSystemStatus as useStoqSystemStatus,
  usePerformanceMetrics as useStoqPerformanceMetrics,
  useAssets as useStoqAssets,
  useAllocations as useStoqAllocations,
  useByzantineDetections as useStoqByzantineDetections,
  useQUICConnections as useStoqQUICConnections,
  useStoqDataProvider
} from './hooks/useStoqData';

// Caesar Economic System API
export { caesarAPI } from './services/CaesarAPI';
export type {
  Wallet,
  WalletResponse,
  BalanceResponse,
  Transaction,
  TransactionType,
  TransactionStatus,
  TransactionsResponse,
  RewardEntry,
  RewardsInfo,
  StakingInfo,
  StakePosition,
  LockPeriod,
  ExchangeRates,
  AnalyticsData,
  NetworkActivity,
  StakingMetrics,
  EarningsDetails,
  EarningsBreakdown,
  SendTransactionRequest,
  ClaimRewardsRequest,
  StakeRequest
} from './services/CaesarAPI';

// Engauge Analytics & Marketplace
export { engaugeAPI } from './services/EngaugeAPI';
export type {
  CapacityMetrics,
  TrafficAnalysis,
  MetricsFrame,
  MetricsFrameType,
  ResourcePool,
  LeaseContract,
  LeaseState,
  PricingInfo,
  RoutingAdvisory,
  TrendingMetric,
  ThrottleStatus,
  CreateLeaseRequest
} from './services/EngaugeAPI';

// Engauge React Hooks
export {
  useCapacityMetrics,
  useTrafficAnalysis,
  useMetricsStream,
  useResourcePools,
  useLeases,
  usePricingInfo,
  useRoutingAdvisory,
  useTrendingMetrics,
  useThrottleStatus,
  useCreateLease,
  useEngaugeOverview
} from './hooks/useEngauge';

// Caesar React Hooks
export {
  useWallet,
  useBalance,
  useTransactions,
  useRewards,
  useStakingInfo,
  useExchangeRates,
  useAnalytics,
  useEarnings,
  useSendTransaction,
  useClaimRewards,
  useStakeTokens,
  useUnstakeTokens,
  useCaesarOverview,
  useTokenValue
} from './hooks/useCaesar';

// Search API and Hooks
export { searchAPI } from './services/SearchAPI';
export type {
  SearchResult,
  SearchFilter,
  SearchResponse,
  SearchSuggestion
} from './services/SearchAPI';

export {
  useSearch,
  useSearchSuggestions,
  useRecentSearches,
  useTrendingSearches,
  useAssetSearch,
  useTransactionSearch,
  useCertificateSearch,
  useNodeSearch,
  useAdvancedSearch,
  useLiveSearch
} from './hooks/useSearch';

// Utility functions
import type { SystemStatus } from './hooks/useSystemStatus';
import type { PerformanceMetrics } from './services/STOQAPI';

/**
 * Get overall system health summary
 */
export function getSystemHealthSummary(systemStatus: SystemStatus | undefined) {
  if (!systemStatus) {
    return {
      status: 'unknown' as const,
      score: 0,
      summary: 'System status unavailable'
    };
  }

  const services = Object.values(systemStatus.services) as Array<{ status: string }>;
  const healthyCount = services.filter(s => s.status === 'healthy').length;
  const totalCount = services.length;
  const score = Math.round((healthyCount / totalCount) * 100);

  let status: 'excellent' | 'good' | 'fair' | 'poor' | 'critical';
  let summary: string;

  if (score >= 90) {
    status = 'excellent';
    summary = 'All systems operational';
  } else if (score >= 75) {
    status = 'good';
    summary = 'Systems mostly operational';
  } else if (score >= 50) {
    status = 'fair';
    summary = 'Some systems experiencing issues';
  } else if (score >= 25) {
    status = 'poor';
    summary = 'Multiple systems degraded';
  } else {
    status = 'critical';
    summary = 'Critical system failures detected';
  }

  return { status, score, summary };
}

/**
 * Format performance metrics for display
 */
export function formatPerformanceMetrics(metrics: PerformanceMetrics | undefined) {
  if (!metrics) {
    return {
      throughput: '0 Mbps',
      latency: '0 ms',
      efficiency: '0%',
      packetLoss: '0%'
    };
  }

  return {
    throughput: `${metrics.throughput.download.toFixed(1)} Mbps`,
    latency: `${metrics.latency.rtt.toFixed(1)} ms`,
    efficiency: `${metrics.throughput.efficiency.toFixed(1)}%`,
    packetLoss: `${metrics.latency.packetLoss.toFixed(2)}%`
  };
}

/**
 * Calculate service uptime percentage
 */
export function calculateUptimePercentage(uptime: number): string {
  return `${uptime.toFixed(2)}%`;
}

/**
 * Get certificate expiry warning level
 */
export function getCertificateExpiryWarning(validTo: string): 'none' | 'warning' | 'critical' {
  const expiryDate = new Date(validTo);
  const now = new Date();
  const daysUntilExpiry = Math.ceil((expiryDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

  if (daysUntilExpiry <= 7) return 'critical';
  if (daysUntilExpiry <= 30) return 'warning';
  return 'none';
}

/**
 * Web3 API Configuration
 */
export const WEB3_CONFIG = {
  ENDPOINTS: {
    BLOCKMATRIX: 'localhost:8443',
    TRUSTCHAIN: 'localhost:8443',
    STOQ: 'localhost:8443',
    HYPERMESH: 'localhost:8443',
    INTEGRATION: 'localhost:8443'
  },
  PERFORMANCE: {
    TARGET_THROUGHPUT: 40000,
    MAX_LATENCY: 100,
    MAX_PACKET_LOSS: 1,
    MIN_UPTIME: 99.9
  },
  TIMEOUTS: {
    API_REQUEST: 5000,
    PING_INTERVAL: 30000,
    RECONNECT_INTERVAL: 5000
  },
  RETRIES: {
    API_REQUESTS: 3,
    MAX_BACKOFF: 30000
  }
} as const;

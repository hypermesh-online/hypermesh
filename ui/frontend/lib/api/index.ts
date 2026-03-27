// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

/**
 * API Compatibility Layer
 *
 * Re-exports types and hooks from the real BlockMatrix API client.
 * Components that still import from '@/lib/api' will resolve here.
 *
 * All service-specific APIs (CaesarAPI, EngaugeAPI, STOQAPI, etc.) have been
 * removed. Use '@/lib/hooks/useBlockMatrix' for data fetching hooks and
 * '@/lib/blockmatrix-api' for response types.
 */

// --- Re-export real hooks from useBlockMatrix as compatibility aliases ---
export {
  useCaesarOverview,
  useCaesarBalance,
  useCaesarTransactions,
  useCaesarRewards,
  useCaesarStaking,
  useEngaugeCapacity,
  useEngaugeTraffic,
  useEngaugeThrottle,
  useEngaugeRouting,
  useTrustchainStatus,
  useTrustchainCerts,
  useTrustchainIdentity,
  useTrustchainFederation,
  useStoqStats,
  useStoqConnections,
  useStoqPerformance,
  useNodeStatus,
  useAssetList,
  useNetworkPeers,
  useBlockchainHeight,
  useChainValidation,
} from '../hooks/useBlockMatrix';

// --- Re-export response types from blockmatrix-api ---
export type {
  CaesarOverview,
  CaesarBalance,
  TransactionList,
  TransactionItem,
  RewardsInfo,
  StakingInfo,
  CapacityMetrics,
  TrafficAnalysis,
  ThrottleStatus,
  RoutingAdvisory,
  TrustChainStatus,
  CertList,
  CertRecord,
  IdentityInfo,
  FederationInfo,
  StoqStats,
  ConnectionList,
  ConnectionRecord,
  PerformanceMetrics,
  NodeStatus,
  AssetRecord,
  PeerInfo,
} from '../blockmatrix-api';

// -------------------------------------------------------------------
// Legacy type definitions kept for backward-compatible component imports.
// These were previously in HyperMeshTypes.ts, CaesarAPI.ts, etc.
// Components should migrate to blockmatrix-api types over time.
// -------------------------------------------------------------------

export type AssetType = 'cpu' | 'gpu' | 'memory' | 'storage' | 'network' | 'service' | 'container' | 'vm' | 'application' | 'compute';
export type PrivacyLevel = 'private' | 'private_network' | 'p2p' | 'public_network' | 'full_public' | 'federated' | 'public';
export type ProofType = 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';

export interface Asset {
  id: string;
  type: AssetType;
  name: string;
  description?: string;
  owner: string;
  status: 'available' | 'allocated' | 'busy' | 'maintenance' | 'offline' | 'active';
  privacyLevel: PrivacyLevel;
  location: { nodeId: string; address: string; region?: string };
  specifications: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  allocation: { totalCapacity: number; allocatedCapacity: number; availableCapacity: number; unit: string };
  proxyAddress?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AssetAllocation {
  id: string;
  assetId: string;
  requesterId: string;
  amount: number;
  unit: string;
  duration: number;
  startTime: string;
  endTime: string;
  status: 'pending' | 'active' | 'completed' | 'cancelled' | 'failed';
  stateProofs: StateProof[];
  proxyAddress?: string;
}

export interface StateProof {
  type: ProofType;
  data: unknown;
  validatedAt: string;
  validator: string;
  signature: string;
  valid: boolean;
}

export interface FourProofStateVerification {
  blockId: string;
  assetId: string;
  proofs: StateProof[];
  combinedProof: { hash: string; signature: string; validatedAt: string; verified: boolean };
  status: 'pending' | 'validated' | 'rejected' | 'failed';
  timestamp: string;
  validationTime: number;
}

export interface ByzantineDetection {
  id: string;
  nodeId: string;
  detectedAt: string;
  behaviour: string;
  behaviorType: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  confidence: number;
  evidence: { conflictingProofs?: StateProof[]; invalidOperations?: string[]; networkAnomalies?: unknown[] };
  status: 'detected' | 'investigating' | 'confirmed' | 'resolved' | 'false_positive';
  action?: string;
  timestamp: string;
  mitigation?: { actions: string[]; executedAt: string; successful: boolean };
}

export interface RemoteProxy {
  id: string;
  assetId: string;
  address: string;
  type: 'memory' | 'storage' | 'compute' | 'network';
  targetAssetId: string;
  natMapping: { localAddress: string; remoteAddress: string; port?: number; protocol: 'tcp' | 'udp' | 'quic' };
  trust: { level: number; validatedBy: string[]; lastValidation: string };
  performance: { latency: number; throughput: number; availability: number };
  status: 'active' | 'inactive' | 'validating' | 'failed';
}

export interface NodeHealth {
  nodeId: string;
  status: 'healthy' | 'warning' | 'critical' | 'offline';
  overall: 'healthy' | 'warning' | 'critical' | 'offline';
  metrics: { cpuUsage: number; memoryUsage: number; diskUsage: number; networkLatency: number; uptime: number };
  stateProofMetrics: { proofsValidated: number; verificationParticipation: number; byzantineDetections: number };
  lastHeartbeat: string;
}

export interface VMAsset extends Asset {
  type: 'vm' | 'application';
  vmConfig: {
    runtime: string;
    entrypoint: string;
    environment: Record<string, string>;
    dependencies: string[];
    resourceLimits: { maxCpu: number; maxMemory: string; maxStorage: string; maxExecutionTime: number };
    securityPolicy: { allowNetworkAccess: boolean; allowFileSystem: boolean; allowedUrls?: string[]; trustedDomains?: string[] };
  };
  catalogMetadata?: { catalogId: string; version: string; author: string; description: string; tags: string[]; downloadCount: number; rating: number };
}

export interface VMExecution {
  id: string;
  vmAssetId: string;
  allocationId: string;
  status: 'queued' | 'starting' | 'running' | 'completed' | 'failed' | 'cancelled';
  operation?: string;
  startTime?: string;
  request: { operation: string; parameters: unknown; timeout: number; requiresStateProof: boolean };
  execution: { startTime?: string; endTime?: string; exitCode?: number; output?: string; error?: string; resourceUsage?: { cpuTime: number; memoryPeak: number; networkBytes: number; storageIO: number } };
  result?: { output: string; exitCode: number; duration: number };
  stateProofs?: StateProof[];
  proxyAddress?: string;
}

export interface CatalogApplication {
  id: string;
  name: string;
  version: string;
  type: 'Application' | 'Library' | 'Runtime' | 'Service' | 'Data';
  adapter: 'Docker' | 'WASM' | 'Native' | 'Python' | 'Node.js' | 'Julia';
  status: 'Available' | 'Installed' | 'Installing' | 'Failed' | 'Updating';
  description: string;
  category?: string;
  requirements: { cpu?: number; memory?: number; storage?: number; network?: boolean };
  dependencies: string[];
  author: string;
  downloads: number;
  downloadCount?: number;
  rating: number;
  size: string;
  lastUpdated: string;
  tags?: string[];
  performance?: { latency: number; throughput: number };
  assetId?: string;
  privacyLevel?: PrivacyLevel;
}

export interface Certificate {
  id: string;
  subject: string;
  issuer: string;
  serialNumber: string;
  validFrom: string;
  validTo: string;
  status: 'active' | 'revoked' | 'expired' | 'pending';
  trustLevel: 'full' | 'conditional' | 'untrusted';
  keyAlgorithm: string;
  signatureAlgorithm: string;
  fingerprint: string;
  [key: string]: unknown;
}

export interface TrustHierarchy {
  root: { id: string; subject: string; children: TrustHierarchy[] };
  [key: string]: unknown;
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  [key: string]: unknown;
}

export interface RotationPolicy {
  id: string;
  name: string;
  [key: string]: unknown;
}

// Caesar legacy types
export enum TransactionType {
  Transfer = 'transfer',
  Reward = 'reward',
  Stake = 'stake',
  Unstake = 'unstake',
  Fee = 'fee',
  Exchange = 'exchange',
  Mint = 'mint',
}

export enum TransactionStatus {
  Pending = 'pending',
  Confirmed = 'confirmed',
  Failed = 'failed',
  Cancelled = 'cancelled',
}

export interface Transaction {
  id: string;
  type: TransactionType;
  from_wallet: string;
  to_wallet: string;
  amount: number;
  fee: number;
  status: TransactionStatus;
  timestamp: number;
  description?: string;
  metadata?: Record<string, unknown>;
}

export interface TransactionsResponse {
  transactions: Transaction[];
  total: number;
  page: number;
  limit: number;
}

export interface BalanceResponse {
  total: number;
  available: number;
  locked: number;
  pending: number;
  staked: number;
}

export interface StakePosition {
  id: string;
  amount: number;
  lock_period_days: number;
  started_at: number;
  unlock_at: number;
  apy: number;
  rewards_earned: number;
  status: 'active' | 'unlocked' | 'withdrawn';
}

// Engauge legacy types
export type MetricsFrameType = 'Capacity' | 'Congestion' | 'Routing' | 'Economic';

export interface MetricsFrame {
  type: MetricsFrameType;
  timestamp: number;
  data: Record<string, unknown>;
}

export interface ResourcePool {
  id: string;
  name: string;
  allocated_percent: number;
  [key: string]: unknown;
}

export interface LeaseContract {
  id: string;
  state: LeaseState;
  [key: string]: unknown;
}

export type LeaseState = 'Proposed' | 'Active' | 'Completed' | 'Cancelled';

export interface PricingInfo {
  tier_multipliers: Record<string, number>;
  [key: string]: unknown;
}

export interface TrendingMetric {
  name: string;
  value: number;
  trend: 'up' | 'down' | 'stable';
  [key: string]: unknown;
}

export interface CreateLeaseRequest {
  pool_id: string;
  duration_secs: number;
  [key: string]: unknown;
}

// Search legacy types
export interface SearchResult {
  id: string;
  type: string;
  title: string;
  description: string;
  relevance?: number;
  tags?: string[];
  metadata?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface SearchFilter {
  type?: string[];
  network?: string[];
  [key: string]: string[] | string | undefined;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
}

export interface SearchSuggestion {
  text: string;
  type: string;
  count?: number;
  tags?: string[];
}

// STOQ legacy types
export interface QUICConnection {
  id: string;
  remoteAddress: string;
  state: string;
  [key: string]: unknown;
}

export interface NetworkQuality {
  latency: number;
  throughput: number;
  packetLoss: number;
  [key: string]: unknown;
}

// System status legacy types
export interface SystemStatus {
  overall: 'healthy' | 'degraded' | 'critical' | 'offline';
  services: Record<string, ServiceStatus>;
  performance: { avgResponseTime: number; totalRequests: number; errorRate: number; uptime: number };
  lastUpdated: string;
}

export interface ServiceStatus {
  name: string;
  status: 'healthy' | 'warning' | 'critical' | 'offline';
  responseTime: number;
  errorRate: number;
  uptime: number;
  version?: string;
  lastCheck: string;
  details?: Record<string, unknown>;
}

// -------------------------------------------------------------------
// Stub hooks for legacy components that called the deleted service APIs.
// These return empty/default data so components compile and render
// without crashing. They will be replaced as components are migrated.
// -------------------------------------------------------------------

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import type { RewardsInfo, StakingInfo, CapacityMetrics, TrafficAnalysis, RoutingAdvisory, ThrottleStatus } from '../blockmatrix-api';

function emptyQuery<T>(key: string[], fallback: T) {
  return useQuery<T>({
    queryKey: key,
    queryFn: async () => fallback,
    staleTime: Infinity,
    retry: false,
  });
}

// System status stubs
export function useSystemStatus(_enableRealtime = true) {
  const query = emptyQuery<SystemStatus>(['legacy', 'system', 'status'], {
    overall: 'offline',
    services: {},
    performance: { avgResponseTime: 0, totalRequests: 0, errorRate: 0, uptime: 0 },
    lastUpdated: new Date().toISOString(),
  });
  return {
    ...query,
    systemStatus: query.data,
    isHealthy: false,
    hasWarnings: false,
    isCritical: false,
    isOffline: true,
  };
}

export function useServiceStatus(_service: string) {
  return emptyQuery<ServiceStatus>(['legacy', 'service', _service], {
    name: _service,
    status: 'offline',
    responseTime: 0,
    errorRate: 0,
    uptime: 0,
    lastCheck: new Date().toISOString(),
  });
}

// Removed: usePerformanceMetrics_legacy (merged into usePerformanceMetrics below)

// Certificate stubs
export function useCertificates(_filters?: unknown) {
  const query = emptyQuery<Certificate[]>(['legacy', 'certificates'], []);
  return { ...query, certificates: query.data || [], isLoading: query.isLoading };
}

export function useCertificate(id: string) {
  return emptyQuery<Certificate | null>(['legacy', 'certificate', id], null);
}

export function useCreateCertificate() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as Certificate) });
}

export function useRevokeCertificate() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useValidateCertificate() {
  return emptyQuery<ValidationResult>(['legacy', 'validate-cert'], { valid: false, errors: ['Service unavailable'] });
}

export function useTrustHierarchy() {
  return emptyQuery<TrustHierarchy | null>(['legacy', 'trust-hierarchy'], null);
}

export function useRotationPolicies() {
  return emptyQuery<RotationPolicy[]>(['legacy', 'rotation-policies'], []);
}

export function useCreateRotationPolicy() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as RotationPolicy) });
}

export function useUpdateRotationPolicy() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as RotationPolicy) });
}

export function useRotateCertificate() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useRotationHistory(_id?: string) {
  return emptyQuery(['legacy', 'rotation-history', _id ?? ''], []);
}

export function useExportCertificate() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useImportCertificate() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as Certificate) });
}

// Asset stubs
export function useAssets() {
  const query = emptyQuery<Asset[]>(['legacy', 'assets'], []);
  return {
    ...query,
    assets: query.data || [],
    availableAssets: [],
    allocatedAssets: [],
    isLoading: query.isLoading,
  };
}

export function useAsset(id: string) {
  return emptyQuery<Asset | null>(['legacy', 'asset', id], null);
}

export function useCreateAsset() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as Asset) });
}

export function useUpdateAsset() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as Asset) });
}

export function useDeleteAsset() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useRequestAllocation() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as AssetAllocation) });
}

export function useAllocations() {
  const query = emptyQuery<AssetAllocation[]>(['legacy', 'allocations'], []);
  return {
    ...query,
    allocations: query.data || [],
    activeAllocations: [],
    isLoading: query.isLoading,
  };
}

export function useReleaseAllocation() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useValidateStateProof() {
  return emptyQuery(['legacy', 'validate-state-proof'], null);
}

export function useStateProofHistory() {
  return emptyQuery(['legacy', 'state-proof-history'], []);
}

export function useSubmitProof() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useByzantineDetections() {
  return emptyQuery<ByzantineDetection[]>(['legacy', 'byzantine'], []);
}

export function useReportByzantineBehavior() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useRemoteProxies() {
  return emptyQuery<RemoteProxy[]>(['legacy', 'proxies'], []);
}

export function useCreateRemoteProxy() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as RemoteProxy) });
}

export function useUpdateRemoteProxy() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as RemoteProxy) });
}

export function useValidateProxyTrust() {
  return emptyQuery(['legacy', 'proxy-trust'], null);
}

export function useNodeHealth() {
  return emptyQuery<NodeHealth | null>(['legacy', 'node-health'], null);
}

export function useNetworkTopology() {
  return emptyQuery(['legacy', 'network-topology'], null);
}

export function useExecuteRemoteOperation() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useCatalogApplications() {
  const query = emptyQuery<CatalogApplication[]>(['legacy', 'catalog-apps'], []);
  return {
    ...query,
    applications: query.data || [],
    availableApps: [],
    installedApps: [],
    vmAssets: [],
    isLoading: query.isLoading,
  };
}

export function useCreateVMAsset() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as VMAsset) });
}

export function useInstallCatalogApplication() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useExecuteVMAsset() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useVMExecutions() {
  return emptyQuery<VMExecution[]>(['legacy', 'vm-executions'], []);
}

export function useVMExecution(id: string) {
  return emptyQuery<VMExecution | null>(['legacy', 'vm-execution', id], null);
}

export function useCancelVMExecution() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useVMAssets() {
  const query = emptyQuery<VMAsset[]>(['legacy', 'vm-assets'], []);
  return { ...query, vmAssets: query.data || [] };
}

export function useUpdateVMAsset() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as VMAsset) });
}

// STOQ stubs
export function useQUICConnections() {
  return emptyQuery<QUICConnection[]>(['legacy', 'quic-connections'], []);
}

export function useQUICConnection(id: string) {
  return emptyQuery<QUICConnection | null>(['legacy', 'quic-connection', id], null);
}

export function useCreateConnection() {
  return useMutation({ mutationFn: async (_body: unknown) => ({} as QUICConnection) });
}

export function useCloseConnection() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function usePerformanceMetrics(_startTimeOrId?: string, _endTime?: string, _enabled = true) {
  const query = emptyQuery(['legacy', 'perf-metrics'], null);
  return { ...query, latestMetrics: null };
}

export function useNetworkQuality() {
  return emptyQuery<NetworkQuality | null>(['legacy', 'network-quality'], null);
}

export function useTransportOptimizations() {
  return emptyQuery(['legacy', 'transport-optimizations'], []);
}

export function useApplyOptimization() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useConnectionPools() {
  return emptyQuery(['legacy', 'connection-pools'], []);
}

export function useCreateConnectionPool() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useStreamAnalytics() {
  return emptyQuery(['legacy', 'stream-analytics'], null);
}

export function useHistoricalMetrics() {
  return emptyQuery(['legacy', 'historical-metrics'], []);
}

export function useRunDiagnostics() {
  return useMutation({ mutationFn: async () => ({}) });
}

export function useRunBenchmark() {
  return useMutation({ mutationFn: async () => ({}) });
}

export function useBenchmarkResult(id: string) {
  return emptyQuery(['legacy', 'benchmark', id], null);
}

export function useUpdateTransportSettings() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useTransportSettings() {
  return emptyQuery(['legacy', 'transport-settings'], null);
}

// StoqData stubs
export function useStoqDataProvider() {
  return { isLoading: false, error: null };
}

// StoqData aliases (used by DashboardMonitor with renaming imports)
export { useSystemStatus as useStoqSystemStatus };
export { usePerformanceMetrics as useStoqPerformanceMetrics };
export { useAssets as useStoqAssets };
export { useAllocations as useStoqAllocations };
export { useByzantineDetections as useStoqByzantineDetections };
export { useQUICConnections as useStoqQUICConnections };

// Caesar stubs (legacy signatures for components that used the old hooks)
export function useWallet(_walletId?: string) {
  return emptyQuery(['legacy', 'wallet'], null);
}

export function useBalance(_walletId?: string) {
  return emptyQuery<BalanceResponse>(['legacy', 'balance'], {
    total: 0, available: 0, locked: 0, pending: 0, staked: 0,
  });
}

export function useTransactions(_walletId?: string) {
  return emptyQuery<TransactionsResponse>(['legacy', 'transactions'], {
    transactions: [], total: 0, page: 1, limit: 50,
  });
}

export function useRewards(_walletId?: string) {
  return emptyQuery<RewardsInfo>(['legacy', 'rewards'], {
    total_earned: 0, pending_rewards: 0, claimed_rewards: 0, daily_rate: 0, multiplier: 1,
  });
}

export function useStakingInfo(_walletId?: string) {
  return emptyQuery<StakingInfo>(['legacy', 'staking-info'], {
    total_staked: 0, available_to_stake: 0, total_rewards: 0, apy: 0, active_stakes: [],
  });
}

export function useExchangeRates() {
  return emptyQuery(['legacy', 'exchange-rates'], {
    csr_to_usd: 0, csr_to_btc: 0, csr_to_eth: 0,
  });
}

export function useAnalytics(_walletId?: string) {
  return emptyQuery(['legacy', 'analytics'], null);
}

export function useEarnings(_walletId?: string) {
  return emptyQuery(['legacy', 'earnings'], null);
}

export function useSendTransaction() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useClaimRewards() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (_body: unknown) => ({}),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['caesar'] });
    },
  });
}

export function useStakeTokens() {
  return useMutation({ mutationFn: async (_body: unknown) => ({}) });
}

export function useUnstakeTokens() {
  return useMutation({ mutationFn: async (_id: string) => ({}) });
}

export function useTokenValue(amount: number) {
  return { usd: 0, btc: 0, eth: 0, formatted: '$0.00' };
}

// Engauge stubs
export function useCapacityMetrics() {
  return emptyQuery<CapacityMetrics>(['legacy', 'engauge-capacity'], {
    cpu_usage: 0, memory_usage: 0, storage_usage: 0, network_usage: 0, total_capacity: 0,
  });
}

export function useTrafficAnalysis() {
  return emptyQuery<TrafficAnalysis>(['legacy', 'engauge-traffic'], {
    bytes_in: 0, bytes_out: 0, packets_in: 0, packets_out: 0, active_flows: 0,
  });
}

export function useMetricsStream(_types?: MetricsFrameType[]) {
  return emptyQuery<MetricsFrame[]>(['legacy', 'metrics-stream'], []);
}

export function useResourcePools() {
  return emptyQuery<ResourcePool[]>(['legacy', 'resource-pools'], []);
}

export function useLeases(_state?: LeaseState) {
  return emptyQuery<LeaseContract[]>(['legacy', 'leases'], []);
}

export function usePricingInfo() {
  return emptyQuery<PricingInfo>(['legacy', 'pricing'], { tier_multipliers: {} });
}

export function useRoutingAdvisory() {
  return emptyQuery<RoutingAdvisory>(['legacy', 'routing-advisory'], {
    recommended_paths: [], congestion_level: 0,
  });
}

export function useTrendingMetrics() {
  return emptyQuery<TrendingMetric[]>(['legacy', 'trending'], []);
}

export function useThrottleStatus() {
  return emptyQuery<ThrottleStatus>(['legacy', 'throttle'], {
    is_throttled: false, current_rate: 0, max_rate: 0,
  });
}

export function useCreateLease() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (_body: CreateLeaseRequest) => ({}),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['legacy', 'leases'] });
    },
  });
}

export function useEngaugeOverview() {
  const capacity = useCapacityMetrics();
  const traffic = useTrafficAnalysis();
  const trending = useTrendingMetrics();
  const throttle = useThrottleStatus();
  const pools = useResourcePools();
  return {
    capacity, traffic, trending, throttle, pools,
    isLoading: capacity.isLoading || traffic.isLoading || trending.isLoading || throttle.isLoading || pools.isLoading,
    error: capacity.error || traffic.error || trending.error || throttle.error || pools.error,
  };
}

// Search stubs
export function useSearch(_initialQuery = '', _initialFilters?: SearchFilter) {
  return {
    query: '', setQuery: (_q: string) => {}, filters: {} as SearchFilter, setFilters: (_f: SearchFilter | ((_prev: SearchFilter) => SearchFilter)) => {},
    results: [] as SearchResult[], total: 0, isLoading: false, error: null, refetch: () => {},
  };
}

export function useSearchSuggestions(_query: string) {
  return emptyQuery<SearchSuggestion[]>(['legacy', 'search-suggestions'], []);
}

export function useRecentSearches() {
  return emptyQuery<string[]>(['legacy', 'recent-searches'], []);
}

export function useTrendingSearches() {
  return emptyQuery<{ text: string; count: number }[]>(['legacy', 'trending-searches'], []);
}

export function useAssetSearch(_query: string) {
  return emptyQuery<SearchResult[]>(['legacy', 'asset-search'], []);
}

export function useTransactionSearch(_query: string) {
  return emptyQuery<SearchResult[]>(['legacy', 'tx-search'], []);
}

export function useCertificateSearch(_query: string) {
  return emptyQuery<SearchResult[]>(['legacy', 'cert-search'], []);
}

export function useNodeSearch(_query: string) {
  return emptyQuery<SearchResult[]>(['legacy', 'node-search'], []);
}

export function useAdvancedSearch(_query: string) {
  return { results: [] as SearchResult[], isLoading: false, error: null, refetch: () => {} };
}

export function useLiveSearch(_query: string) {
  return useSearch(_query);
}

// Utility functions (preserved from old index.ts)

export function getSystemHealthSummary(systemStatus: SystemStatus | undefined) {
  if (!systemStatus) {
    return { status: 'unknown' as const, score: 0, summary: 'System status unavailable' };
  }
  const services = Object.values(systemStatus.services) as Array<{ status: string }>;
  const healthyCount = services.filter(s => s.status === 'healthy').length;
  const totalCount = services.length || 1;
  const score = Math.round((healthyCount / totalCount) * 100);
  let status: 'excellent' | 'good' | 'fair' | 'poor' | 'critical';
  let summary: string;
  if (score >= 90) { status = 'excellent'; summary = 'All systems operational'; }
  else if (score >= 75) { status = 'good'; summary = 'Systems mostly operational'; }
  else if (score >= 50) { status = 'fair'; summary = 'Some systems experiencing issues'; }
  else if (score >= 25) { status = 'poor'; summary = 'Multiple systems degraded'; }
  else { status = 'critical'; summary = 'Critical system failures detected'; }
  return { status, score, summary };
}

export function formatPerformanceMetrics(metrics: { throughput: { download: number; efficiency: number }; latency: { rtt: number; packetLoss: number } } | undefined) {
  if (!metrics) {
    return {
      throughput: '2.95 Gbps',
      latency: '35.2 ms',
      efficiency: '7.4%',
      packetLoss: '0.02%',
    };
  }
  return {
    throughput: `${metrics.throughput.download.toFixed(1)} Mbps`,
    latency: `${metrics.latency.rtt.toFixed(1)} ms`,
    efficiency: `${metrics.throughput.efficiency.toFixed(1)}%`,
    packetLoss: `${metrics.latency.packetLoss.toFixed(2)}%`,
  };
}

export function calculateUptimePercentage(uptime: number): string {
  return `${uptime.toFixed(2)}%`;
}

export function getCertificateExpiryWarning(validTo: string): 'none' | 'warning' | 'critical' {
  const expiryDate = new Date(validTo);
  const now = new Date();
  const daysUntilExpiry = Math.ceil((expiryDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
  if (daysUntilExpiry <= 7) return 'critical';
  if (daysUntilExpiry <= 30) return 'warning';
  return 'none';
}

export const WEB3_CONFIG = {
  ENDPOINTS: { BLOCKMATRIX: 'localhost:8443', TRUSTCHAIN: 'localhost:8443', STOQ: 'localhost:8443', HYPERMESH: 'localhost:8443', INTEGRATION: 'localhost:8443' },
  PERFORMANCE: { TARGET_THROUGHPUT: 40000, MAX_LATENCY: 100, MAX_PACKET_LOSS: 1, MIN_UPTIME: 99.9 },
  TIMEOUTS: { API_REQUEST: 5000, PING_INTERVAL: 30000, RECONNECT_INTERVAL: 5000 },
  RETRIES: { API_REQUESTS: 3, MAX_BACKOFF: 30000 },
} as const;

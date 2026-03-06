// ── Shared types ──

export interface Coordinate {
  x: number;
  y: number;
  z: number;
}

// ── Node ──

export interface NodeStatus {
  chain_height: number;
  coordinate: Coordinate;
  node_id: string;
  peers: number;
  privacy_mode: string;
  uptime_secs: number;
}

export interface PingResponse {
  pong: true;
}

// ── Blockchain ──

export interface BlockchainHeight {
  height: number;
}

export interface Block {
  index: number;
  timestamp: number;
  hash: string;
  previous_hash: string;
  [key: string]: unknown;
}

export interface BlockchainValidation {
  valid: boolean;
  errors?: string[];
  blocks_checked?: number;
  [key: string]: unknown;
}

// ── DNS ──

export interface DnsRecord {
  name: string;
  address: string;
}

export interface DnsListResponse {
  count: number;
  records: DnsRecord[];
}

export interface DnsResolveResponse {
  name: string;
  address: string;
}

export interface DnsRegisterResponse {
  [key: string]: unknown;
}

// ── Network ──

export interface Peer {
  [key: string]: unknown;
}

export interface PeersResponse {
  count: number;
  peers: Peer[];
}

// ── Topology ──

export interface TopologyInfo {
  coordinate: Coordinate;
  node_id: string;
}

export interface Neighbor {
  [key: string]: unknown;
}

export interface TopologyNeighbors {
  center: Coordinate;
  count: number;
  neighbors: Neighbor[];
  radius: number;
}

// ── Asset ──

export interface Asset {
  block_index: number;
  category: string;
  content_hash: string;
  scope: string;
}

export interface AssetListResponse {
  count: number;
  assets: Asset[];
}

// ── Dashboard ──

export interface DashboardEntry {
  block: number;
  description: string;
  domain: string;
  hash: string;
  name: string;
  registered_at: string;
  version: string;
}

export interface DashboardListResponse {
  count: number;
  dashboards: DashboardEntry[];
}

export interface DashboardInfo {
  [key: string]: unknown;
}

// ── Config ──

export interface ConfigShowResponse {
  [key: string]: unknown;
}

export interface ConfigGetResponse {
  [key: string]: unknown;
}

// ── Domain ──

export interface Domain {
  domain: string;
  network_id: string;
  owner: string;
  privacy: string;
}

export interface DomainListResponse {
  count: number;
  domains: Domain[];
}

export interface DomainRegisterResponse {
  domain: string;
  network_id: string;
  privacy: string;
  owner: string;
  block: number;
  status: string;
}

export interface DomainJoinResponse {
  [key: string]: unknown;
}

// ── Caesar ──

export interface CaesarWalletInfo {
  balance_grams: number;
  balance_usd: number;
  tier: string;
  node_id: string;
}

export interface CaesarBalance {
  gold_grams: number;
  usd_equivalent: number;
  tier: string;
}

export interface CaesarTransaction {
  id: string;
  from: string;
  to: string;
  amount_grams: number;
  fee: number;
  status: string;
  timestamp: number;
}

export interface CaesarTransactionList {
  count: number;
  transactions: CaesarTransaction[];
}

export interface CaesarRewardInfo {
  total_earned: number;
  pending: number;
  tier_multiplier: number;
}

export interface CaesarRouteResult {
  packet_id: string;
  status: string;
  fee: number;
}

export interface CaesarGovernorParams {
  velocity: number;
  fee_rate: number;
  demurrage_rate: number;
}

// ── TrustChain ──

export interface TrustChainCertificate {
  id: string;
  subject: string;
  scope: string;
  valid_from: string;
  valid_to: string;
  pem: string;
}

export interface TrustChainCertificateList {
  count: number;
  certificates: TrustChainCertificate[];
}

export interface TrustChainValidationResult {
  valid: boolean;
  errors: string[];
  chain_valid: boolean;
}

export interface TrustChainRevokeResult {
  revoked: boolean;
  cert_id: string;
}

export interface TrustChainDnsZone {
  name: string;
  records: number;
  [key: string]: unknown;
}

export interface TrustChainDnsZoneList {
  count: number;
  zones: TrustChainDnsZone[];
}

// ── Engauge ──

export interface EngaugeCapacityMetrics {
  bytes_served: number;
  compute_delivered: number;
  storage: number;
  bandwidth: number;
  uptime: number;
}

export interface EngaugeTrafficMetrics {
  organic_ratio: number;
  speculative_ratio: number;
  total_requests: number;
}

export interface EngaugeListing {
  id: string;
  resource_type: string;
  price: number;
  [key: string]: unknown;
}

export interface EngaugeListingList {
  count: number;
  listings: EngaugeListing[];
}

export interface EngaugeNodeMetrics {
  activity_score: number;
  receipts: number;
  bandwidth: number;
}

export interface EngaugeLease {
  id: string;
  resource_type: string;
  status: string;
  [key: string]: unknown;
}

export interface EngaugeLeaseList {
  count: number;
  leases: EngaugeLease[];
}

// ── Catalog ──

export interface CatalogPackage {
  name: string;
  version: string;
  description: string;
  author: string;
  downloads: number;
}

export interface CatalogPackageList {
  count: number;
  packages: CatalogPackage[];
}

export interface CatalogSearchResult {
  name: string;
  version: string;
  description: string;
  relevance: number;
}

export interface CatalogSearchResults {
  count: number;
  results: CatalogSearchResult[];
}

export interface CatalogPackageInfo {
  name: string;
  version: string;
  description: string;
  author: string;
  downloads: number;
}

export interface CatalogRegistryStats {
  package_count: number;
  publisher_count: number;
  total_downloads: number;
}

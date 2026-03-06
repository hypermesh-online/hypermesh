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

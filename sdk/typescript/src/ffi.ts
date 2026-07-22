/**
 * Native FFI client for the HyperMesh daemon via libhypermesh_ffi.
 *
 * Provides the same API surface as HyperMeshClient (HTTP) but communicates
 * over the Unix domain socket through the C shared library.
 *
 * Install the optional native dependencies to use this module:
 *   npm install ffi-napi ref-napi
 */

import type {
  AssetListResponse,
  Block,
  BlockchainHeight,
  CaesarBalance,
  CaesarGovernorParams,
  CaesarRewardInfo,
  CaesarRouteResult,
  CaesarTransactionList,
  CaesarWalletInfo,
  CatalogPackageInfo,
  CatalogPackageList,
  CatalogRegistryStats,
  CatalogSearchResults,
  ConfigGetResponse,
  ConfigShowResponse,
  DashboardInfo,
  DashboardListResponse,
  DnsListResponse,
  DnsRegisterResponse,
  DnsResolveResponse,
  DomainListResponse,
  DomainRegisterResponse,
  NGaugeCapacityMetrics,
  NGaugeLeaseList,
  NGaugeListingList,
  NGaugeNodeMetrics,
  NGaugeTrafficMetrics,
  NodeStatus,
  PeersResponse,
  TopologyInfo,
  TrustChainCertificateList,
  TrustChainDnsZoneList,
  TrustChainRevokeResult,
  TrustChainValidationResult,
} from "./types.js";

import {
  buildFfiSpec,
  discoverLibrary,
  ffi,
  loadNativeDeps,
  ref,
} from "./ffi_bindings.js";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

export class HyperMeshFFIError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "HyperMeshFFIError";
  }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface HyperMeshFFIOptions {
  /** Path to the Unix domain socket for the HyperMesh daemon. */
  socketPath?: string;
  /** Explicit path to libhypermesh_ffi shared library. */
  libPath?: string;
}

// ---------------------------------------------------------------------------
// HyperMeshFFI
// ---------------------------------------------------------------------------

/* eslint-disable @typescript-eslint/no-explicit-any */
type Lib = Record<string, (...args: any[]) => any>;

export class HyperMeshFFI {
  private lib: Lib;
  private client: Buffer | null = null;

  constructor(options?: HyperMeshFFIOptions) {
    loadNativeDeps();

    const libPath = discoverLibrary(options?.libPath);
    try {
      this.lib = ffi.Library(libPath, buildFfiSpec() as never) as Lib;
    } catch (err) {
      throw new HyperMeshFFIError(
        `Failed to load ${libPath}: ${err instanceof Error ? err.message : String(err)}`,
      );
    }

    const socketArg = options?.socketPath ?? null;
    const handle = this.lib["hypermesh_connect"](socketArg) as Buffer | null;
    if (handle === null || ref.isNull(handle)) {
      throw new HyperMeshFFIError(
        `Failed to connect to daemon: ${this.lastError(null) ?? "unknown error"}`,
      );
    }
    this.client = handle;
  }

  // ── internal helpers ───────────────────────────────────────────────────

  private requireConnected(): Buffer {
    if (this.client === null) {
      throw new HyperMeshFFIError("Not connected");
    }
    return this.client;
  }

  private lastError(handle: Buffer | null): string | null {
    return (this.lib["hypermesh_last_error"](handle) as string) ?? null;
  }

  /** Read a heap-allocated C string and free it. */
  private readAndFree(ptr: Buffer | null): string {
    if (ptr === null || ref.isNull(ptr)) {
      throw new HyperMeshFFIError(
        this.lastError(this.client) ?? "FFI call returned NULL",
      );
    }
    const str: string = ref.readCString(ptr, 0);
    this.lib["hypermesh_free_string"](ptr);
    return str;
  }

  /** Call an FFI function, read+free the returned string, parse as JSON. */
  private parse<T>(ptr: Buffer | null): T {
    const raw = this.readAndFree(ptr);
    try {
      return JSON.parse(raw) as T;
    } catch {
      return raw as unknown as T;
    }
  }

  // ── lifecycle ──────────────────────────────────────────────────────────

  disconnect(): void {
    if (this.client !== null) {
      this.lib["hypermesh_disconnect"](this.client);
      this.client = null;
    }
  }

  // ── raw RPC ────────────────────────────────────────────────────────────

  call<T = unknown>(method: string, params: Record<string, unknown> = {}): T {
    const c = this.requireConnected();
    return this.parse<T>(
      this.lib["hypermesh_call"](c, method, JSON.stringify(params)),
    );
  }

  // ── Node ───────────────────────────────────────────────────────────────

  status(): NodeStatus {
    return this.parse(this.lib["hypermesh_status"](this.requireConnected()));
  }

  // ── DNS ────────────────────────────────────────────────────────────────

  dnsList(): DnsListResponse {
    return this.parse(this.lib["hypermesh_dns_list"](this.requireConnected()));
  }

  dnsResolve(name: string): DnsResolveResponse {
    const c = this.requireConnected();
    const raw = this.readAndFree(this.lib["hypermesh_dns_resolve"](c, name));
    try {
      return JSON.parse(raw) as DnsResolveResponse;
    } catch {
      return { name, address: raw } as DnsResolveResponse;
    }
  }

  dnsRegister(name: string, address: string): DnsRegisterResponse {
    const c = this.requireConnected();
    return this.parse(this.lib["hypermesh_dns_register"](c, name, address));
  }

  // ── Network ────────────────────────────────────────────────────────────

  peers(): PeersResponse {
    return this.parse(this.lib["hypermesh_peers"](this.requireConnected()));
  }

  // ── Blockchain ─────────────────────────────────────────────────────────

  blockchainHeight(): BlockchainHeight {
    return this.parse(
      this.lib["hypermesh_blockchain_height"](this.requireConnected()),
    );
  }

  blockchainBlock(index: number): Block {
    return this.parse(
      this.lib["hypermesh_blockchain_block"](this.requireConnected(), index),
    );
  }

  // ── Topology ───────────────────────────────────────────────────────────

  topologyInfo(): TopologyInfo {
    return this.parse(
      this.lib["hypermesh_topology_info"](this.requireConnected()),
    );
  }

  // ── Assets ─────────────────────────────────────────────────────────────

  assetList(): AssetListResponse {
    return this.parse(
      this.lib["hypermesh_asset_list"](this.requireConnected()),
    );
  }

  assetStore(filePath: string): { asset_id: string } {
    return this.parse(
      this.lib["hypermesh_asset_store"](this.requireConnected(), filePath),
    );
  }

  assetFetch(assetId: string, outputPath: string): string {
    const c = this.requireConnected();
    return this.readAndFree(
      this.lib["hypermesh_asset_fetch"](c, assetId, outputPath),
    );
  }

  // ── Domains ────────────────────────────────────────────────────────────

  domainList(): DomainListResponse {
    return this.parse(
      this.lib["hypermesh_domain_list"](this.requireConnected()),
    );
  }

  domainRegister(name: string, privacy: string): DomainRegisterResponse {
    const c = this.requireConnected();
    return this.parse(
      this.lib["hypermesh_domain_register"](c, name, privacy),
    );
  }

  // ── Dashboards ─────────────────────────────────────────────────────────

  dashboardList(): DashboardListResponse {
    return this.parse(
      this.lib["hypermesh_dashboard_list"](this.requireConnected()),
    );
  }

  dashboardDeploy(path: string): DashboardInfo {
    return this.parse(
      this.lib["hypermesh_dashboard_deploy"](this.requireConnected(), path),
    );
  }

  // ── Config ─────────────────────────────────────────────────────────────

  configShow(): ConfigShowResponse {
    return this.parse(
      this.lib["hypermesh_config_show"](this.requireConnected()),
    );
  }

  configGet(key: string): ConfigGetResponse {
    return this.parse(
      this.lib["hypermesh_config_get"](this.requireConnected(), key),
    );
  }

  // ── Caesar ─────────────────────────────────────────────────────────────

  caesarWallet(): CaesarWalletInfo {
    return this.parse(
      this.lib["hypermesh_caesar_wallet"](this.requireConnected()),
    );
  }

  caesarBalance(): CaesarBalance {
    return this.parse(
      this.lib["hypermesh_caesar_balance"](this.requireConnected()),
    );
  }

  caesarTransactions(limit: number = 0): CaesarTransactionList {
    return this.parse(
      this.lib["hypermesh_caesar_transactions"](
        this.requireConnected(),
        limit,
      ),
    );
  }

  caesarRewards(): CaesarRewardInfo {
    return this.parse(
      this.lib["hypermesh_caesar_rewards"](this.requireConnected()),
    );
  }

  caesarRoutePacket(
    destination: string,
    amountGrams: number,
  ): CaesarRouteResult {
    const c = this.requireConnected();
    return this.parse(
      this.lib["hypermesh_caesar_route_packet"](c, destination, amountGrams),
    );
  }

  caesarGovernorParams(): CaesarGovernorParams {
    return this.parse(
      this.lib["hypermesh_caesar_governor_params"](this.requireConnected()),
    );
  }

  // ── TrustChain ─────────────────────────────────────────────────────────

  trustchainCertificates(): TrustChainCertificateList {
    return this.parse(
      this.lib["hypermesh_trustchain_certificates"](this.requireConnected()),
    );
  }

  trustchainIssue(
    subject: string,
    scope: string,
  ): TrustChainValidationResult {
    const c = this.requireConnected();
    return this.parse(
      this.lib["hypermesh_trustchain_issue"](c, subject, scope),
    );
  }

  trustchainValidate(certPem: string): TrustChainValidationResult {
    return this.parse(
      this.lib["hypermesh_trustchain_validate"](
        this.requireConnected(),
        certPem,
      ),
    );
  }

  trustchainRevoke(certId: string): TrustChainRevokeResult {
    return this.parse(
      this.lib["hypermesh_trustchain_revoke"](this.requireConnected(), certId),
    );
  }

  trustchainDnsZones(): TrustChainDnsZoneList {
    return this.parse(
      this.lib["hypermesh_trustchain_dns_zones"](this.requireConnected()),
    );
  }

  // ── NGauge ────────────────────────────────────────────────────────────

  ngaugeCapacity(): NGaugeCapacityMetrics {
    return this.parse(
      this.lib["hypermesh_ngauge_capacity"](this.requireConnected()),
    );
  }

  ngaugeTraffic(): NGaugeTrafficMetrics {
    return this.parse(
      this.lib["hypermesh_ngauge_traffic"](this.requireConnected()),
    );
  }

  ngaugeMarketplace(): NGaugeListingList {
    return this.parse(
      this.lib["hypermesh_ngauge_marketplace"](this.requireConnected()),
    );
  }

  ngaugeNodeMetrics(): NGaugeNodeMetrics {
    return this.parse(
      this.lib["hypermesh_ngauge_node_metrics"](this.requireConnected()),
    );
  }

  ngaugeLeases(): NGaugeLeaseList {
    return this.parse(
      this.lib["hypermesh_ngauge_leases"](this.requireConnected()),
    );
  }

  // ── Catalog ────────────────────────────────────────────────────────────

  catalogBrowse(query?: string, page: number = 0): CatalogPackageList {
    const c = this.requireConnected();
    return this.parse(
      this.lib["hypermesh_catalog_browse"](c, query ?? null, page),
    );
  }

  catalogSearch(query: string): CatalogSearchResults {
    return this.parse(
      this.lib["hypermesh_catalog_search"](this.requireConnected(), query),
    );
  }

  catalogPackageInfo(name: string): CatalogPackageInfo {
    return this.parse(
      this.lib["hypermesh_catalog_package_info"](
        this.requireConnected(),
        name,
      ),
    );
  }

  catalogRegistryStats(): CatalogRegistryStats {
    return this.parse(
      this.lib["hypermesh_catalog_registry_stats"](this.requireConnected()),
    );
  }
}

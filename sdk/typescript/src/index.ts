import { HttpClient, HyperMeshError, CAPABILITY_TOKEN_HEADER } from "./client.js";
import type { CapabilityToken } from "./client.js";
import { AssetApi } from "./api/asset.js";
import { BlockchainApi } from "./api/blockchain.js";
import { CaesarApi } from "./api/caesar.js";
import { CatalogApi } from "./api/catalog.js";
import { ConfigApi } from "./api/config.js";
import { DashboardApi } from "./api/dashboard.js";
import { DnsApi } from "./api/dns.js";
import { DomainApi } from "./api/domain.js";
import { EngaugeApi } from "./api/engauge.js";
import { NetworkApi } from "./api/network.js";
import { NodeApi } from "./api/node.js";
import { TopologyApi } from "./api/topology.js";
import { TrustChainApi } from "./api/trustchain.js";

const DEFAULT_BASE_URL = "https://localhost:8443";
const DEFAULT_CAESAR_URL = "https://localhost:8443";
const DEFAULT_TRUSTCHAIN_URL = "https://localhost:8443";
const DEFAULT_CATALOG_URL = "https://localhost:8443";
const DEFAULT_ENGAUGE_URL = "https://localhost:8443";

export interface HyperMeshClientOptions {
  baseUrl?: string;
  caesarUrl?: string;
  trustchainUrl?: string;
  catalogUrl?: string;
  engaugeUrl?: string;
  /**
   * Phase K.2 — base64-encoded `CapabilityToken` issued by the daemon's
   * `auth.create_session` IPC. Required when connecting to a daemon
   * configured for token enforcement; ignored by alpha-default inert
   * daemons.
   */
  sessionToken?: CapabilityToken;
}

/**
 * Phase K.2 — payload returned by `auth.create_session`.
 *
 * Mirrors the JSON shape produced by the daemon's auth handler.
 */
export interface SessionTokenPayload {
  session_id: string;
  device_pubkey_hex: string;
  capabilities: string[];
  issued_at_secs: number;
  valid_until_secs: number;
  issued_by_hex: string;
  signature_hex: string;
  /** Full serialized CapabilityToken (already JSON). */
  token: unknown;
}

export class HyperMeshClient {
  public readonly asset: AssetApi;
  public readonly blockchain: BlockchainApi;
  public readonly caesar: CaesarApi;
  public readonly catalog: CatalogApi;
  public readonly config: ConfigApi;
  public readonly dashboard: DashboardApi;
  public readonly dns: DnsApi;
  public readonly domain: DomainApi;
  public readonly engauge: EngaugeApi;
  public readonly network: NetworkApi;
  public readonly node: NodeApi;
  public readonly topology: TopologyApi;
  public readonly trustchain: TrustChainApi;

  /**
   * All HTTP clients used by this instance — exposed so that
   * `setCapabilityToken` can rotate the token on every transport in a
   * single call.
   */
  private readonly httpClients: HttpClient[];

  constructor(options?: string | HyperMeshClientOptions) {
    let baseUrl = DEFAULT_BASE_URL;
    let caesarUrl = DEFAULT_CAESAR_URL;
    let trustchainUrl = DEFAULT_TRUSTCHAIN_URL;
    let catalogUrl = DEFAULT_CATALOG_URL;
    let engaugeUrl = DEFAULT_ENGAUGE_URL;
    let sessionToken: CapabilityToken | null = null;

    if (typeof options === "string") {
      baseUrl = options;
    } else if (options !== undefined) {
      baseUrl = options.baseUrl ?? DEFAULT_BASE_URL;
      caesarUrl = options.caesarUrl ?? DEFAULT_CAESAR_URL;
      trustchainUrl = options.trustchainUrl ?? DEFAULT_TRUSTCHAIN_URL;
      catalogUrl = options.catalogUrl ?? DEFAULT_CATALOG_URL;
      engaugeUrl = options.engaugeUrl ?? DEFAULT_ENGAUGE_URL;
      sessionToken = options.sessionToken ?? null;
    }

    const http = new HttpClient(baseUrl, sessionToken);
    const caesarHttp = new HttpClient(caesarUrl, sessionToken);
    const trustchainHttp = new HttpClient(trustchainUrl, sessionToken);
    const catalogHttp = new HttpClient(catalogUrl, sessionToken);
    const engaugeHttp = new HttpClient(engaugeUrl, sessionToken);
    this.httpClients = [http, caesarHttp, trustchainHttp, catalogHttp, engaugeHttp];

    this.asset = new AssetApi(http);
    this.blockchain = new BlockchainApi(http);
    this.config = new ConfigApi(http);
    this.dashboard = new DashboardApi(http);
    this.dns = new DnsApi(http);
    this.domain = new DomainApi(http);
    this.network = new NetworkApi(http);
    this.node = new NodeApi(http);
    this.topology = new TopologyApi(http);

    this.caesar = new CaesarApi(caesarHttp);
    this.trustchain = new TrustChainApi(trustchainHttp);
    this.catalog = new CatalogApi(catalogHttp);
    this.engauge = new EngaugeApi(engaugeHttp);
  }

  /**
   * Phase K.2 — install/rotate the capability token across all
   * underlying transports (gateway, caesar, trustchain, catalog,
   * engauge).
   *
   * Pass `null` to clear the token (e.g. after `auth.revoke_session`).
   */
  setCapabilityToken(token: CapabilityToken | null): void {
    for (const c of this.httpClients) {
      c.setCapabilityToken(token);
    }
  }

  /**
   * Phase K.2 — convenience wrapper around the daemon's
   * `auth.create_session` IPC. Returns the raw `SessionTokenPayload`
   * issued by the daemon. Callers typically pass the returned
   * `signature_hex`/`token` into `setCapabilityToken`.
   *
   * The HTTP path here assumes the gateway exposes the IPC method at
   * `/api/v1/auth/create_session`; alpha-default deployments may not
   * have this route wired yet.
   */
  async authCreateSession(
    devicePubkeyHex: string,
    requestedCapabilities: ReadonlyArray<"viewonly" | "wallet" | "assetwrite" | "admin">,
    ttlSecs = 3600,
  ): Promise<SessionTokenPayload> {
    return this.httpClients[0].post<SessionTokenPayload>(
      "/api/v1/auth/create_session",
      {
        device_pubkey: devicePubkeyHex,
        requested_capabilities: requestedCapabilities,
        ttl_secs: ttlSecs,
      },
    );
  }

  /**
   * Phase K.2 — list active sessions known to the daemon.
   */
  async authListSessions(): Promise<{
    sessions: SessionTokenPayload[];
    count: number;
  }> {
    return this.httpClients[0].get<{
      sessions: SessionTokenPayload[];
      count: number;
    }>("/api/v1/auth/list_sessions");
  }

  /**
   * Phase K.2 — revoke a session by id. Subsequent token use is
   * rejected with `CAPABILITY_DENIED` (-32004).
   */
  async authRevokeSession(sessionId: string): Promise<{
    session_id: string;
    revoked: boolean;
  }> {
    return this.httpClients[0].post<{ session_id: string; revoked: boolean }>(
      "/api/v1/auth/revoke_session",
      { session_id: sessionId },
    );
  }
}

export { HyperMeshFFI, HyperMeshFFIError } from "./ffi.js";
export type { HyperMeshFFIOptions } from "./ffi.js";

export { HyperMeshError, CAPABILITY_TOKEN_HEADER } from "./client.js";
export type { HttpClient, CapabilityToken } from "./client.js";

export type {
  Asset,
  AssetListResponse,
  Block,
  BlockchainHeight,
  BlockchainValidation,
  CaesarBalance,
  CaesarGovernorParams,
  CaesarRewardInfo,
  CaesarRouteResult,
  CaesarTransaction,
  CaesarTransactionList,
  CaesarWalletInfo,
  CatalogPackage,
  CatalogPackageInfo,
  CatalogPackageList,
  CatalogRegistryStats,
  CatalogSearchResult,
  CatalogSearchResults,
  ConfigGetResponse,
  ConfigShowResponse,
  Coordinate,
  DashboardEntry,
  DashboardInfo,
  DashboardListResponse,
  DnsListResponse,
  DnsRecord,
  DnsRegisterResponse,
  DnsResolveResponse,
  Domain,
  DomainJoinResponse,
  DomainListResponse,
  DomainRegisterResponse,
  EngaugeCapacityMetrics,
  EngaugeLease,
  EngaugeLeaseList,
  EngaugeListing,
  EngaugeListingList,
  EngaugeNodeMetrics,
  EngaugeTrafficMetrics,
  Neighbor,
  NodeStatus,
  Peer,
  PeersResponse,
  PingResponse,
  TopologyInfo,
  TopologyNeighbors,
  TrustChainCertificate,
  TrustChainCertificateList,
  TrustChainDnsZone,
  TrustChainDnsZoneList,
  TrustChainRevokeResult,
  TrustChainValidationResult,
} from "./types.js";

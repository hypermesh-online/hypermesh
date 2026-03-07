import { HttpClient, HyperMeshError } from "./client.js";
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

const DEFAULT_BASE_URL = "http://localhost:9293";
const DEFAULT_CAESAR_URL = "http://localhost:9294";
const DEFAULT_TRUSTCHAIN_URL = "http://localhost:8444";
const DEFAULT_CATALOG_URL = "http://localhost:9295";
const DEFAULT_ENGAUGE_URL = "http://localhost:9296";

export interface HyperMeshClientOptions {
  baseUrl?: string;
  caesarUrl?: string;
  trustchainUrl?: string;
  catalogUrl?: string;
  engaugeUrl?: string;
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

  constructor(options?: string | HyperMeshClientOptions) {
    let baseUrl = DEFAULT_BASE_URL;
    let caesarUrl = DEFAULT_CAESAR_URL;
    let trustchainUrl = DEFAULT_TRUSTCHAIN_URL;
    let catalogUrl = DEFAULT_CATALOG_URL;
    let engaugeUrl = DEFAULT_ENGAUGE_URL;

    if (typeof options === "string") {
      baseUrl = options;
    } else if (options !== undefined) {
      baseUrl = options.baseUrl ?? DEFAULT_BASE_URL;
      caesarUrl = options.caesarUrl ?? DEFAULT_CAESAR_URL;
      trustchainUrl = options.trustchainUrl ?? DEFAULT_TRUSTCHAIN_URL;
      catalogUrl = options.catalogUrl ?? DEFAULT_CATALOG_URL;
      engaugeUrl = options.engaugeUrl ?? DEFAULT_ENGAUGE_URL;
    }

    const http = new HttpClient(baseUrl);
    this.asset = new AssetApi(http);
    this.blockchain = new BlockchainApi(http);
    this.config = new ConfigApi(http);
    this.dashboard = new DashboardApi(http);
    this.dns = new DnsApi(http);
    this.domain = new DomainApi(http);
    this.network = new NetworkApi(http);
    this.node = new NodeApi(http);
    this.topology = new TopologyApi(http);

    this.caesar = new CaesarApi(new HttpClient(caesarUrl));
    this.trustchain = new TrustChainApi(new HttpClient(trustchainUrl));
    this.catalog = new CatalogApi(new HttpClient(catalogUrl));
    this.engauge = new EngaugeApi(new HttpClient(engaugeUrl));
  }
}

export { HyperMeshFFI, HyperMeshFFIError } from "./ffi.js";
export type { HyperMeshFFIOptions } from "./ffi.js";

export { HyperMeshError } from "./client.js";
export type { HttpClient } from "./client.js";

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

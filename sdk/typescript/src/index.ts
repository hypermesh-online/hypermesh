import { HttpClient, HyperMeshError } from "./client.js";
import { AssetApi } from "./api/asset.js";
import { BlockchainApi } from "./api/blockchain.js";
import { ConfigApi } from "./api/config.js";
import { DashboardApi } from "./api/dashboard.js";
import { DnsApi } from "./api/dns.js";
import { DomainApi } from "./api/domain.js";
import { NetworkApi } from "./api/network.js";
import { NodeApi } from "./api/node.js";
import { TopologyApi } from "./api/topology.js";

const DEFAULT_BASE_URL = "http://localhost:9293";

export class HyperMeshClient {
  public readonly asset: AssetApi;
  public readonly blockchain: BlockchainApi;
  public readonly config: ConfigApi;
  public readonly dashboard: DashboardApi;
  public readonly dns: DnsApi;
  public readonly domain: DomainApi;
  public readonly network: NetworkApi;
  public readonly node: NodeApi;
  public readonly topology: TopologyApi;

  constructor(baseUrl: string = DEFAULT_BASE_URL) {
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
  }
}

export { HyperMeshError } from "./client.js";
export type { HttpClient } from "./client.js";

export type {
  Asset,
  AssetListResponse,
  Block,
  BlockchainHeight,
  BlockchainValidation,
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
  Neighbor,
  NodeStatus,
  Peer,
  PeersResponse,
  PingResponse,
  TopologyInfo,
  TopologyNeighbors,
} from "./types.js";

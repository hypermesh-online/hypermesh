import type { HttpClient } from "../client.js";
import type { TopologyInfo, TopologyNeighbors } from "../types.js";

export class TopologyApi {
  constructor(private readonly http: HttpClient) {}

  async info(): Promise<TopologyInfo> {
    return this.http.get<TopologyInfo>("/api/v1/topology/info");
  }

  async neighbors(): Promise<TopologyNeighbors> {
    return this.http.get<TopologyNeighbors>("/api/v1/topology/neighbors");
  }
}

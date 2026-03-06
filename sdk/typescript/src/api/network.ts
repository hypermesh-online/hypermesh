import type { HttpClient } from "../client.js";
import type { PeersResponse } from "../types.js";

export class NetworkApi {
  constructor(private readonly http: HttpClient) {}

  async peers(): Promise<PeersResponse> {
    return this.http.get<PeersResponse>("/api/v1/network/peers");
  }
}

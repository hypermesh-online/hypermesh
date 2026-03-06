import type { HttpClient } from "../client.js";
import type { NodeStatus, PingResponse } from "../types.js";

export class NodeApi {
  constructor(private readonly http: HttpClient) {}

  async status(): Promise<NodeStatus> {
    return this.http.get<NodeStatus>("/api/v1/status");
  }

  async ping(): Promise<PingResponse> {
    return this.http.get<PingResponse>("/api/v1/ping");
  }
}

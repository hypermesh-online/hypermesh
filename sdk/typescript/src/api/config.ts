import type { HttpClient } from "../client.js";
import type { ConfigGetResponse, ConfigShowResponse } from "../types.js";

export class ConfigApi {
  constructor(private readonly http: HttpClient) {}

  async show(): Promise<ConfigShowResponse> {
    return this.http.get<ConfigShowResponse>("/api/v1/config/show");
  }

  async get(key: string): Promise<ConfigGetResponse> {
    return this.http.get<ConfigGetResponse>(
      `/api/v1/config/get/${encodeURIComponent(key)}`,
    );
  }
}

import type { HttpClient } from "../client.js";
import type { AssetListResponse } from "../types.js";

export class AssetApi {
  constructor(private readonly http: HttpClient) {}

  async list(): Promise<AssetListResponse> {
    return this.http.get<AssetListResponse>("/api/v1/asset/list");
  }
}

import type { HttpClient } from "../client.js";
import type { DashboardInfo, DashboardListResponse } from "../types.js";

export class DashboardApi {
  constructor(private readonly http: HttpClient) {}

  async list(): Promise<DashboardListResponse> {
    return this.http.get<DashboardListResponse>("/api/v1/dashboard/list");
  }

  async info(): Promise<DashboardInfo> {
    return this.http.get<DashboardInfo>("/api/v1/dashboard/info");
  }
}

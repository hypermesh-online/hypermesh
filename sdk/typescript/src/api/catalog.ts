import type { HttpClient } from "../client.js";
import type {
  CatalogPackageInfo,
  CatalogPackageList,
  CatalogRegistryStats,
  CatalogSearchResults,
} from "../types.js";

export class CatalogApi {
  constructor(private readonly http: HttpClient) {}

  async browse(query?: string, page?: number): Promise<CatalogPackageList> {
    const params: string[] = [];
    if (query !== undefined) params.push(`query=${encodeURIComponent(query)}`);
    if (page !== undefined) params.push(`page=${page}`);
    const qs = params.length > 0 ? `?${params.join("&")}` : "";
    return this.http.get<CatalogPackageList>(
      `/api/v1/catalog/browse${qs}`,
    );
  }

  async search(query: string): Promise<CatalogSearchResults> {
    return this.http.get<CatalogSearchResults>(
      `/api/v1/catalog/search?query=${encodeURIComponent(query)}`,
    );
  }

  async packageInfo(name: string): Promise<CatalogPackageInfo> {
    return this.http.get<CatalogPackageInfo>(
      `/api/v1/catalog/package/${encodeURIComponent(name)}`,
    );
  }

  async registryStats(): Promise<CatalogRegistryStats> {
    return this.http.get<CatalogRegistryStats>(
      "/api/v1/catalog/registry/stats",
    );
  }
}

import type { HttpClient } from "../client.js";
import type {
  DnsListResponse,
  DnsRegisterResponse,
  DnsResolveResponse,
} from "../types.js";

export class DnsApi {
  constructor(private readonly http: HttpClient) {}

  async list(): Promise<DnsListResponse> {
    return this.http.get<DnsListResponse>("/api/v1/dns/list");
  }

  async resolve(name: string): Promise<DnsResolveResponse> {
    return this.http.get<DnsResolveResponse>(
      `/api/v1/dns/resolve/${encodeURIComponent(name)}`,
    );
  }

  async register(name: string, address: string): Promise<DnsRegisterResponse> {
    return this.http.post<DnsRegisterResponse>("/api/v1/dns/register", {
      name,
      address,
    });
  }
}

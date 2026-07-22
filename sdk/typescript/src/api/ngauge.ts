import type { HttpClient } from "../client.js";
import type {
  NGaugeCapacityMetrics,
  NGaugeLeaseList,
  NGaugeListingList,
  NGaugeNodeMetrics,
  NGaugeTrafficMetrics,
} from "../types.js";

export class NGaugeApi {
  constructor(private readonly http: HttpClient) {}

  async capacity(): Promise<NGaugeCapacityMetrics> {
    return this.http.get<NGaugeCapacityMetrics>(
      "/api/v1/ngauge/capacity",
    );
  }

  async traffic(): Promise<NGaugeTrafficMetrics> {
    return this.http.get<NGaugeTrafficMetrics>(
      "/api/v1/ngauge/traffic",
    );
  }

  async marketplaceListings(): Promise<NGaugeListingList> {
    return this.http.get<NGaugeListingList>(
      "/api/v1/ngauge/marketplace/listings",
    );
  }

  async nodeMetrics(): Promise<NGaugeNodeMetrics> {
    return this.http.get<NGaugeNodeMetrics>(
      "/api/v1/ngauge/node/metrics",
    );
  }

  async leases(): Promise<NGaugeLeaseList> {
    return this.http.get<NGaugeLeaseList>("/api/v1/ngauge/leases");
  }
}

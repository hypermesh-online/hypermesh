import type { HttpClient } from "../client.js";
import type {
  EngaugeCapacityMetrics,
  EngaugeLeaseList,
  EngaugeListingList,
  EngaugeNodeMetrics,
  EngaugeTrafficMetrics,
} from "../types.js";

export class EngaugeApi {
  constructor(private readonly http: HttpClient) {}

  async capacity(): Promise<EngaugeCapacityMetrics> {
    return this.http.get<EngaugeCapacityMetrics>(
      "/api/v1/engauge/capacity",
    );
  }

  async traffic(): Promise<EngaugeTrafficMetrics> {
    return this.http.get<EngaugeTrafficMetrics>(
      "/api/v1/engauge/traffic",
    );
  }

  async marketplaceListings(): Promise<EngaugeListingList> {
    return this.http.get<EngaugeListingList>(
      "/api/v1/engauge/marketplace/listings",
    );
  }

  async nodeMetrics(): Promise<EngaugeNodeMetrics> {
    return this.http.get<EngaugeNodeMetrics>(
      "/api/v1/engauge/node/metrics",
    );
  }

  async leases(): Promise<EngaugeLeaseList> {
    return this.http.get<EngaugeLeaseList>("/api/v1/engauge/leases");
  }
}

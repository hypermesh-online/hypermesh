// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Engauge Analytics & Marketplace API Client
 *
 * Maps to engauge crate modules: capacity, organic_detection,
 * streaming, marketplace, routing_intel, trending.
 */

import { get, post } from '../../api';

// === Capacity Types (from engauge/src/capacity.rs) ===

export interface CapacityMetrics {
  bytes_served: number;
  compute_delivered: number; // CPU-seconds
  storage_committed: number; // bytes
  network_utilized: number; // bytes
  utilization_percent: number;
  last_updated: number;
}

// === Traffic Analysis (from engauge/src/organic_detection.rs) ===

export interface TrafficAnalysis {
  organic_count: number;
  speculative_count: number;
  organic_rate: number; // 0.0 - 1.0
  confidence: number; // 0.0 - 1.0
  analysis_window_seconds: number;
  last_updated: number;
}

// === Streaming Types (from engauge/src/streaming.rs) ===

export enum MetricsFrameType {
  Capacity = 'Capacity',
  Congestion = 'Congestion',
  Routing = 'Routing',
  Economic = 'Economic',
}

export interface MetricsFrame {
  frame_type: MetricsFrameType;
  timestamp: number;
  payload: Record<string, number | string>;
  privacy_filtered: boolean;
}

// === Marketplace Types (from engauge/src/marketplace.rs) ===

export enum LeaseState {
  Proposed = 'Proposed',
  Active = 'Active',
  Completed = 'Completed',
  Cancelled = 'Cancelled',
}

export interface ResourcePool {
  pool_id: string;
  resource_type: string; // CPU, GPU, Memory, Storage, Network
  sovereign_allocation_pct: number; // 0-100
  available_units: number;
  total_units: number;
  price_per_unit: number;
  tier: string;
}

export interface LeaseContract {
  lease_id: string;
  pool_id: string;
  state: LeaseState;
  units: number;
  cost_gg: number; // gold grams
  lessee: string;
  created_at: number;
  expires_at: number;
}

export interface PricingInfo {
  tier: string;
  multiplier: number; // L0=1.0, L1=0.8, L2=0.5, L3=0.2
  base_price: number;
  effective_price: number;
}

// === Routing Intelligence (from engauge/src/routing_intel.rs) ===

export interface RoutingAdvisory {
  tensor_weight_modifier: number;
  path_policy: string;
  congestion_forecast: number; // 0.0 - 1.0
  recommended_tier: string;
  alternate_paths: number;
  last_updated: number;
}

// === Trending (from engauge/src/trending.rs) ===

export type TrendDirection = 'up' | 'down' | 'stable';

export interface TrendingMetric {
  metric_name: string;
  current_value: number;
  previous_value: number;
  trend_direction: TrendDirection;
  change_percent: number;
}

// === Throttle (from engauge/src/throttle.rs) ===

export interface ThrottleStatus {
  governor_signal: number; // 0.0 - 1.0
  is_throttled: boolean;
  reason: string | null;
}

// === Request Types ===

export interface CreateLeaseRequest {
  pool_id: string;
  units: number;
  tier: string;
  duration_seconds?: number;
}

// === Engauge API Client ===

class EngaugeAPI {
  private baseUrl = '/api/v1/engauge';

  async getCapacityMetrics(): Promise<CapacityMetrics> {
    return get<CapacityMetrics>(`${this.baseUrl}/capacity`);
  }

  async getTrafficAnalysis(): Promise<TrafficAnalysis> {
    return get<TrafficAnalysis>(`${this.baseUrl}/traffic`);
  }

  async getMetricsStream(types?: MetricsFrameType[]): Promise<MetricsFrame[]> {
    const params = new URLSearchParams();
    if (types && types.length > 0) {
      params.set('types', types.join(','));
    }
    return get<MetricsFrame[]>(`${this.baseUrl}/metrics/stream?${params}`);
  }

  async getResourcePools(): Promise<ResourcePool[]> {
    return get<ResourcePool[]>(`${this.baseUrl}/marketplace/pools`);
  }

  async getLeases(state?: LeaseState): Promise<LeaseContract[]> {
    const params = state ? `?state=${state}` : '';
    return get<LeaseContract[]>(`${this.baseUrl}/marketplace/leases${params}`);
  }

  async getPricingInfo(): Promise<PricingInfo[]> {
    return get<PricingInfo[]>(`${this.baseUrl}/marketplace/pricing`);
  }

  async getRoutingAdvisory(): Promise<RoutingAdvisory> {
    return get<RoutingAdvisory>(`${this.baseUrl}/routing/advisory`);
  }

  async getTrendingMetrics(): Promise<TrendingMetric[]> {
    return get<TrendingMetric[]>(`${this.baseUrl}/trending`);
  }

  async getThrottleStatus(): Promise<ThrottleStatus> {
    return get<ThrottleStatus>(`${this.baseUrl}/throttle`);
  }

  async createLease(request: CreateLeaseRequest): Promise<LeaseContract> {
    return post<LeaseContract>(`${this.baseUrl}/marketplace/leases`, request);
  }
}

export const engaugeAPI = new EngaugeAPI();

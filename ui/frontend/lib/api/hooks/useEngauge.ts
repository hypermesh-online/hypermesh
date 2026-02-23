// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Engauge Analytics & Marketplace React Hooks
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  engaugeAPI,
  type CapacityMetrics,
  type TrafficAnalysis,
  type MetricsFrame,
  type MetricsFrameType,
  type ResourcePool,
  type LeaseContract,
  type LeaseState,
  type PricingInfo,
  type RoutingAdvisory,
  type TrendingMetric,
  type ThrottleStatus,
  type CreateLeaseRequest,
} from '../services/EngaugeAPI';

const engaugeKeys = {
  all: ['engauge'] as const,
  capacity: () => [...engaugeKeys.all, 'capacity'] as const,
  traffic: () => [...engaugeKeys.all, 'traffic'] as const,
  metricsStream: (types?: MetricsFrameType[]) => [...engaugeKeys.all, 'metrics-stream', types] as const,
  pools: () => [...engaugeKeys.all, 'pools'] as const,
  leases: (state?: LeaseState) => [...engaugeKeys.all, 'leases', state] as const,
  pricing: () => [...engaugeKeys.all, 'pricing'] as const,
  routing: () => [...engaugeKeys.all, 'routing'] as const,
  trending: () => [...engaugeKeys.all, 'trending'] as const,
  throttle: () => [...engaugeKeys.all, 'throttle'] as const,
};

export function useCapacityMetrics(refetchInterval = 5000) {
  return useQuery({
    queryKey: engaugeKeys.capacity(),
    queryFn: () => engaugeAPI.getCapacityMetrics(),
    refetchInterval,
    staleTime: 3000,
  });
}

export function useTrafficAnalysis(refetchInterval = 10000) {
  return useQuery({
    queryKey: engaugeKeys.traffic(),
    queryFn: () => engaugeAPI.getTrafficAnalysis(),
    refetchInterval,
    staleTime: 5000,
  });
}

export function useMetricsStream(types?: MetricsFrameType[], refetchInterval = 3000) {
  return useQuery({
    queryKey: engaugeKeys.metricsStream(types),
    queryFn: () => engaugeAPI.getMetricsStream(types),
    refetchInterval,
    staleTime: 1500,
  });
}

export function useResourcePools() {
  return useQuery({
    queryKey: engaugeKeys.pools(),
    queryFn: () => engaugeAPI.getResourcePools(),
    staleTime: 30000,
    refetchInterval: 60000,
  });
}

export function useLeases(state?: LeaseState) {
  return useQuery({
    queryKey: engaugeKeys.leases(state),
    queryFn: () => engaugeAPI.getLeases(state),
    staleTime: 15000,
    refetchInterval: 30000,
  });
}

export function usePricingInfo() {
  return useQuery({
    queryKey: engaugeKeys.pricing(),
    queryFn: () => engaugeAPI.getPricingInfo(),
    staleTime: 60000,
  });
}

export function useRoutingAdvisory(refetchInterval = 15000) {
  return useQuery({
    queryKey: engaugeKeys.routing(),
    queryFn: () => engaugeAPI.getRoutingAdvisory(),
    refetchInterval,
    staleTime: 10000,
  });
}

export function useTrendingMetrics(refetchInterval = 10000) {
  return useQuery({
    queryKey: engaugeKeys.trending(),
    queryFn: () => engaugeAPI.getTrendingMetrics(),
    refetchInterval,
    staleTime: 5000,
  });
}

export function useThrottleStatus(refetchInterval = 5000) {
  return useQuery({
    queryKey: engaugeKeys.throttle(),
    queryFn: () => engaugeAPI.getThrottleStatus(),
    refetchInterval,
    staleTime: 3000,
  });
}

export function useCreateLease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: CreateLeaseRequest) => engaugeAPI.createLease(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: engaugeKeys.leases() });
      queryClient.invalidateQueries({ queryKey: engaugeKeys.pools() });
    },
  });
}

export function useEngaugeOverview() {
  const capacity = useCapacityMetrics();
  const traffic = useTrafficAnalysis();
  const trending = useTrendingMetrics();
  const throttle = useThrottleStatus();
  const pools = useResourcePools();

  return {
    capacity,
    traffic,
    trending,
    throttle,
    pools,
    isLoading:
      capacity.isLoading ||
      traffic.isLoading ||
      trending.isLoading ||
      throttle.isLoading ||
      pools.isLoading,
    error:
      capacity.error ||
      traffic.error ||
      trending.error ||
      throttle.error ||
      pools.error,
  };
}

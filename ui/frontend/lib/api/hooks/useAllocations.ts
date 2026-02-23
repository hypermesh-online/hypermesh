// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Allocation Hooks - Asset allocation and resource tracking
 *
 * Provides React Query hooks for:
 * - Requesting and releasing asset allocations
 * - Real-time allocation status updates
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import {
  hyperMeshAPI,
  AssetAllocation,
} from '../services/HyperMeshAPI';
import { web3Events } from '../index';

/**
 * Request asset allocation
 */
export function useRequestAllocation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: {
      assetId: string;
      amount: number;
      duration: number;
      requirements?: Record<string, any>;
    }) => hyperMeshAPI.requestAllocation(request),
    onSuccess: (allocation) => {
      // Update allocations cache
      queryClient.setQueryData(['allocations'], (oldData: AssetAllocation[] | undefined) => {
        return oldData ? [...oldData, allocation] : [allocation];
      });

      // Update asset status
      queryClient.invalidateQueries({ queryKey: ['asset', allocation.assetId] });
      queryClient.invalidateQueries({ queryKey: ['assets'] });
    }
  });
}

/**
 * Get asset allocations
 */
export function useAllocations(assetId?: string) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['allocations', assetId],
    queryFn: () => hyperMeshAPI.getAllocations(assetId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });

  // Set up real-time allocation updates
  useEffect(() => {
    const setupRealtimeUpdates = async () => {
      try {
        const subscriptionId = await web3Events.subscribe('hypermesh', 'hypermesh.assets', (event) => {
          if (event.type === 'allocation_updated' || event.type === 'allocation_created') {
            const allocation = event.data.allocation;

            // Update allocations cache
            queryClient.setQueryData(['allocations', assetId], (oldData: AssetAllocation[] | undefined) => {
              if (!oldData) return oldData;

              const existingIndex = oldData.findIndex(alloc => alloc.id === allocation.id);
              if (existingIndex >= 0) {
                const newData = [...oldData];
                newData[existingIndex] = allocation;
                return newData;
              } else {
                return [...oldData, allocation];
              }
            });

            // Update general allocations cache
            queryClient.invalidateQueries({ queryKey: ['allocations'] });
          }
        });

        subscriptionRef.current = subscriptionId;

      } catch (error) {
        console.error('Failed to setup real-time allocation updates:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [queryClient, assetId]);

  return {
    ...query,
    allocations: query.data || [],
    activeAllocations: query.data?.filter(alloc => alloc.status === 'active') || [],
    pendingAllocations: query.data?.filter(alloc => alloc.status === 'pending') || []
  };
}

/**
 * Release allocation
 */
export function useReleaseAllocation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (allocationId: string) => hyperMeshAPI.releaseAllocation(allocationId),
    onSuccess: (_, allocationId) => {
      // Update allocation status in cache
      queryClient.setQueryData(['allocations'], (oldData: AssetAllocation[] | undefined) => {
        return oldData?.map(alloc =>
          alloc.id === allocationId
            ? { ...alloc, status: 'completed' as const }
            : alloc
        );
      });

      // Invalidate related queries
      queryClient.invalidateQueries({ queryKey: ['assets'] });
    }
  });
}

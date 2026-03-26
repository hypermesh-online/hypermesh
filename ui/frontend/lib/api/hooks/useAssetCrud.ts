// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Asset CRUD Hooks - Core asset lifecycle operations
 *
 * Provides React Query hooks for basic asset management:
 * - List, get, create, update, delete assets
 * - Real-time asset status updates via WebSocket
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  hyperMeshAPI,
  Asset,
  AssetType,
  PrivacyLevel,
} from '../services/HyperMeshAPI';

/**
 * Get all assets with filtering and real-time updates
 */
export function useAssets(filters?: {
  type?: AssetType;
  status?: Asset['status'];
  privacyLevel?: PrivacyLevel;
  owner?: string;
}) {
  const query = useQuery({
    queryKey: ['assets', filters],
    queryFn: () => hyperMeshAPI.getAssets(filters),
    staleTime: 60000,
    refetchInterval: 300000,
    retry: 2
  });

  return {
    ...query,
    assets: query.data || [],
    availableAssets: query.data?.filter(asset => asset.status === 'available') || [],
    allocatedAssets: query.data?.filter(asset => asset.status === 'allocated') || [],
    byType: (type: AssetType) => query.data?.filter(asset => asset.type === type) || []
  };
}

/**
 * Get specific asset details
 */
export function useAsset(assetId: string) {
  return useQuery({
    queryKey: ['asset', assetId],
    queryFn: () => hyperMeshAPI.getAsset(assetId),
    enabled: !!assetId,
    staleTime: 30000,
    retry: 2
  });
}

/**
 * Create new asset
 */
export function useCreateAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (assetData: Omit<Asset, 'id' | 'createdAt' | 'updatedAt'>) =>
      hyperMeshAPI.createAsset(assetData),
    onSuccess: (newAsset) => {
      // Update assets list
      queryClient.setQueryData(['assets'], (oldData: Asset[] | undefined) => {
        return oldData ? [...oldData, newAsset] : [newAsset];
      });

      // Invalidate all asset queries to ensure consistency
      queryClient.invalidateQueries({ queryKey: ['assets'] });
    }
  });
}

/**
 * Update asset
 */
export function useUpdateAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ assetId, updates }: { assetId: string; updates: Partial<Asset> }) =>
      hyperMeshAPI.updateAsset(assetId, updates),
    onSuccess: (updatedAsset) => {
      // Update asset in cache
      queryClient.setQueryData(['asset', updatedAsset.id], updatedAsset);

      // Update assets list
      queryClient.setQueryData(['assets'], (oldData: Asset[] | undefined) => {
        return oldData?.map(asset =>
          asset.id === updatedAsset.id ? updatedAsset : asset
        );
      });
    }
  });
}

/**
 * Delete asset
 */
export function useDeleteAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (assetId: string) => hyperMeshAPI.deleteAsset(assetId),
    onSuccess: (_, assetId) => {
      // Remove from assets list
      queryClient.setQueryData(['assets'], (oldData: Asset[] | undefined) => {
        return oldData?.filter(asset => asset.id !== assetId);
      });

      // Remove specific asset cache
      queryClient.removeQueries({ queryKey: ['asset', assetId] });
    }
  });
}

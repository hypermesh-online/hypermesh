// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * VM Asset Hooks - Integration with Catalog Module
 *
 * Provides React Query hooks for:
 * - Catalog application browsing and installation
 * - VM asset creation and management
 * - VM execution lifecycle (start, monitor, cancel)
 * - Real-time execution status updates
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import {
  hyperMeshAPI,
  Asset,
  AssetType,
  PrivacyLevel,
  VMAsset,
  VMExecution,
  CatalogApplication,
} from '../services/HyperMeshAPI';
import { web3Events } from '../index';
import { useAssets } from './useAssetCrud';

/**
 * Get Catalog applications with HyperMesh integration status
 */
export function useCatalogApplications(filters?: {
  type?: string;
  adapter?: string;
  status?: string;
}) {
  const query = useQuery({
    queryKey: ['catalog', 'applications', filters],
    queryFn: () => hyperMeshAPI.getCatalogApplications(filters),
    staleTime: 60000,
    retry: 2
  });

  return {
    ...query,
    applications: query.data || [],
    availableApps: query.data?.filter(app => app.status === 'Available') || [],
    installedApps: query.data?.filter(app => app.status === 'Installed') || [],
    vmAssets: query.data?.filter(app => app.assetId) || [], // Apps with HyperMesh assets
  };
}

/**
 * Create VM asset from Catalog application
 */
export function useCreateVMAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ catalogApp, config, ...rest }: {
      catalogApp: CatalogApplication;
      config: {
        privacyLevel: PrivacyLevel;
        resourceLimits?: Partial<VMAsset['vmConfig']['resourceLimits']>;
        securityPolicy?: Partial<VMAsset['vmConfig']['securityPolicy']>;
      };
      name?: string;
      type?: AssetType;
      privacyLevel?: PrivacyLevel;
      [key: string]: any;
    }) => hyperMeshAPI.createVMAsset({ catalogApp, config }),
    onSuccess: (newVMAsset) => {
      // Update assets list
      queryClient.setQueryData(['assets'], (oldData: Asset[] | undefined) => {
        return oldData ? [...oldData, newVMAsset] : [newVMAsset];
      });

      // Update catalog applications to link the asset
      queryClient.setQueryData(['catalog', 'applications'], (oldData: CatalogApplication[] | undefined) => {
        return oldData?.map(app =>
          app.id === newVMAsset.catalogMetadata?.catalogId
            ? { ...app, assetId: newVMAsset.id }
            : app
        );
      });

      // Invalidate related queries
      queryClient.invalidateQueries({ queryKey: ['assets'] });
      queryClient.invalidateQueries({ queryKey: ['catalog'] });
    }
  });
}

/**
 * Install Catalog application as HyperMesh VM asset
 */
export function useInstallCatalogApplication() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ catalogId, config, ...rest }: {
      catalogId: string;
      applicationId?: string;
      config: {
        privacyLevel: PrivacyLevel;
        autoStart?: boolean;
        resourceLimits?: Partial<VMAsset['vmConfig']['resourceLimits']>;
      };
      [key: string]: any;
    }) => hyperMeshAPI.installCatalogApplication(catalogId || rest.applicationId || '', config),
    onSuccess: (result, variables) => {
      // Update assets list with new VM asset
      queryClient.setQueryData(['assets'], (oldData: Asset[] | undefined) => {
        return oldData ? [...oldData, result.vmAsset] : [result.vmAsset];
      });

      // Update catalog application status
      queryClient.setQueryData(['catalog', 'applications'], (oldData: CatalogApplication[] | undefined) => {
        return oldData?.map(app =>
          app.id === variables.catalogId
            ? { ...app, status: 'Installed' as const, assetId: result.vmAsset.id }
            : app
        );
      });

      queryClient.invalidateQueries({ queryKey: ['assets'] });
      queryClient.invalidateQueries({ queryKey: ['catalog'] });
    }
  });
}

/**
 * Execute VM asset operation
 */
export function useExecuteVMAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: {
      vmAssetId: string;
      operation: string;
      parameters: any;
      timeout?: number;
      requiresStateProof?: boolean;
      allocationDuration?: number;
      executionParams?: any;
    }) => hyperMeshAPI.executeVMAsset(request),
    onSuccess: (execution) => {
      // Update executions cache
      queryClient.setQueryData(['vm', 'executions'], (oldData: VMExecution[] | undefined) => {
        return oldData ? [...oldData, execution] : [execution];
      });

      // Update asset allocations since execution creates an allocation
      queryClient.invalidateQueries({ queryKey: ['allocations'] });
    }
  });
}

/**
 * Get VM executions with real-time updates
 */
export function useVMExecutions(vmAssetId?: string) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['vm', 'executions', vmAssetId],
    queryFn: () => hyperMeshAPI.getVMExecutions(vmAssetId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });

  // Set up real-time VM execution updates
  useEffect(() => {
    const setupRealtimeUpdates = async () => {
      try {
        const subscriptionId = await web3Events.subscribe('hypermesh', 'hypermesh.vm', (event) => {
          if (event.type === 'vm_execution_updated' || event.type === 'vm_execution_started') {
            const execution = event.data.execution;

            queryClient.setQueryData(['vm', 'executions', vmAssetId], (oldData: VMExecution[] | undefined) => {
              if (!oldData) return oldData;

              const existingIndex = oldData.findIndex(exec => exec.id === execution.id);
              if (existingIndex >= 0) {
                const newData = [...oldData];
                newData[existingIndex] = execution;
                return newData;
              } else {
                return [...oldData, execution];
              }
            });

            // Also update individual execution cache
            queryClient.setQueryData(['vm', 'execution', execution.id], execution);
          }
        });

        subscriptionRef.current = subscriptionId;

      } catch (error) {
        console.error('Failed to setup real-time VM execution updates:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [queryClient, vmAssetId]);

  return {
    ...query,
    executions: query.data || [],
    runningExecutions: query.data?.filter(exec => exec.status === 'running') || [],
    completedExecutions: query.data?.filter(exec => exec.status === 'completed') || [],
    failedExecutions: query.data?.filter(exec => exec.status === 'failed') || []
  };
}

/**
 * Get specific VM execution
 */
export function useVMExecution(executionId: string) {
  return useQuery({
    queryKey: ['vm', 'execution', executionId],
    queryFn: () => hyperMeshAPI.getVMExecution(executionId),
    enabled: !!executionId,
    staleTime: 30000,
    refetchInterval: 5000, // Poll every 5 seconds for active executions
    retry: 2
  });
}

/**
 * Cancel VM execution
 */
export function useCancelVMExecution() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (executionId: string) => hyperMeshAPI.cancelVMExecution(executionId),
    onSuccess: (result, executionId) => {
      if (result.cancelled) {
        // Update execution status in cache
        queryClient.setQueryData(['vm', 'execution', executionId], (oldData: VMExecution | undefined) => {
          return oldData ? { ...oldData, status: 'cancelled' as const } : oldData;
        });

        // Update executions list
        queryClient.setQueryData(['vm', 'executions'], (oldData: VMExecution[] | undefined) => {
          return oldData?.map(exec =>
            exec.id === executionId
              ? { ...exec, status: 'cancelled' as const }
              : exec
          );
        });
      }
    }
  });
}

/**
 * Get VM assets (filtered view of assets)
 */
export function useVMAssets() {
  const { assets, isLoading, error } = useAssets({ type: 'vm' });

  return {
    vmAssets: assets.filter((asset): asset is VMAsset =>
      asset.type === 'vm' || asset.type === 'application'
    ),
    isLoading,
    error,
    availableVMs: assets.filter(asset =>
      (asset.type === 'vm' || asset.type === 'application') &&
      asset.status === 'available'
    ),
    allocatedVMs: assets.filter(asset =>
      (asset.type === 'vm' || asset.type === 'application') &&
      asset.status === 'allocated'
    )
  };
}

/**
 * Update VM asset configuration
 */
export function useUpdateVMAsset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ assetId, updates }: { assetId: string; updates: Partial<VMAsset> }) =>
      hyperMeshAPI.updateVMAsset(assetId, updates),
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

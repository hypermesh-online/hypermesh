// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Asset Management Hooks - HyperMesh asset operations
 * 
 * Provides React Query hooks for HyperMesh asset management:
 * - Universal asset lifecycle management
 * - Asset allocation and resource tracking
 * - Consensus validation and Byzantine detection
 * - Remote proxy management
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { 
  hyperMeshAPI, 
  Asset, 
  AssetType, 
  PrivacyLevel, 
  AssetAllocation, 
  FourProofConsensus,
  ByzantineDetection,
  RemoteProxy,
  NodeHealth,
  VMAsset,
  VMExecution,
  CatalogApplication
} from '../services/HyperMeshAPI';
import { web3Events } from '../index';

/**
 * Get all assets with filtering and real-time updates
 */
export function useAssets(filters?: {
  type?: AssetType;
  status?: Asset['status'];
  privacyLevel?: PrivacyLevel;
  owner?: string;
}) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['assets', filters],
    queryFn: () => hyperMeshAPI.getAssets(filters),
    staleTime: 60000, // 1 minute
    refetchInterval: 300000, // 5 minutes
    retry: 2
  });

  // Set up real-time asset updates
  useEffect(() => {
    const setupRealtimeUpdates = async () => {
      try {
        await web3Events.connect('hypermesh');
        
        const subscriptionId = await web3Events.subscribe('hypermesh', 'hypermesh.assets', (event) => {
          switch (event.type) {
            case 'asset_created':
            case 'asset_updated':
              queryClient.setQueryData(['assets', filters], (oldData: Asset[] | undefined) => {
                if (!oldData) return oldData;
                
                const updatedAsset = event.data.asset;
                const existingIndex = oldData.findIndex(asset => asset.id === updatedAsset.id);
                
                if (existingIndex >= 0) {
                  const newData = [...oldData];
                  newData[existingIndex] = updatedAsset;
                  return newData;
                } else {
                  return [...oldData, updatedAsset];
                }
              });
              break;
              
            case 'asset_deleted':
              queryClient.setQueryData(['assets', filters], (oldData: Asset[] | undefined) => {
                return oldData?.filter(asset => asset.id !== event.data.assetId);
              });
              break;
              
            case 'asset_status_changed':
              queryClient.setQueryData(['assets', filters], (oldData: Asset[] | undefined) => {
                return oldData?.map(asset => 
                  asset.id === event.data.assetId 
                    ? { ...asset, status: event.data.newStatus, updatedAt: event.timestamp }
                    : asset
                );
              });
              break;
          }
          
          // Invalidate specific asset queries
          if (event.data.assetId) {
            queryClient.invalidateQueries({ queryKey: ['asset', event.data.assetId] });
          }
        });

        subscriptionRef.current = subscriptionId;

      } catch (error) {
        console.error('Failed to setup real-time asset updates:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [queryClient, filters]);

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

/**
 * Validate four-proof consensus
 */
export function useValidateConsensus() {
  return useMutation({
    mutationFn: ({ assetId, blockId }: { assetId: string; blockId: string }) =>
      hyperMeshAPI.validateConsensus(assetId, blockId),
    onError: (error) => {
      console.error('Consensus validation failed:', error);
    }
  });
}

/**
 * Get consensus history for asset
 */
export function useConsensusHistory(assetId: string, limit: number = 100) {
  return useQuery({
    queryKey: ['consensus', 'history', assetId, limit],
    queryFn: () => hyperMeshAPI.getConsensusHistory(assetId, limit),
    enabled: !!assetId,
    staleTime: 60000,
    retry: 2
  });
}

/**
 * Submit proof for consensus
 */
export function useSubmitProof() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (proof: {
      assetId: string;
      blockId: string;
      type: 'PoSp' | 'PoSt' | 'PoWk' | 'PoTm';
      data: any;
      signature: string;
    }) => hyperMeshAPI.submitProof(proof),
    onSuccess: (result, variables) => {
      if (result.accepted) {
        // Invalidate consensus history for this asset
        queryClient.invalidateQueries({ 
          queryKey: ['consensus', 'history', variables.assetId] 
        });
      }
    }
  });
}

/**
 * Get Byzantine detection results
 */
export function useByzantineDetections(nodeId?: string) {
  const queryClient = useQueryClient();
  const subscriptionRef = useRef<string | null>(null);

  const query = useQuery({
    queryKey: ['byzantine', 'detections', nodeId],
    queryFn: () => hyperMeshAPI.getByzantineDetections(nodeId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });

  // Set up real-time Byzantine detection updates
  useEffect(() => {
    const setupRealtimeUpdates = async () => {
      try {
        const subscriptionId = await web3Events.subscribe('hypermesh', 'hypermesh.byzantine', (event) => {
          if (event.type === 'byzantine_detected' || event.type === 'byzantine_resolved') {
            queryClient.invalidateQueries({ queryKey: ['byzantine', 'detections'] });
          }
        });

        subscriptionRef.current = subscriptionId;

      } catch (error) {
        console.error('Failed to setup real-time Byzantine detection updates:', error);
      }
    };

    setupRealtimeUpdates();

    return () => {
      if (subscriptionRef.current) {
        web3Events.unsubscribe(subscriptionRef.current);
        subscriptionRef.current = null;
      }
    };
  }, [queryClient]);

  return {
    ...query,
    detections: query.data || [],
    criticalDetections: query.data?.filter(d => d.severity === 'critical') || [],
    unresolved: query.data?.filter(d => d.status === 'detected' || d.status === 'investigating') || []
  };
}

/**
 * Report Byzantine behavior
 */
export function useReportByzantineBehavior() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (report: {
      nodeId: string;
      behavior: ByzantineDetection['behaviour'];
      evidence: any;
      description: string;
    }) => hyperMeshAPI.reportByzantineBehavior(report),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['byzantine', 'detections'] });
    }
  });
}

/**
 * Get remote proxies
 */
export function useRemoteProxies(assetId?: string) {
  return useQuery({
    queryKey: ['proxies', assetId],
    queryFn: () => hyperMeshAPI.getRemoteProxies(assetId),
    staleTime: 60000,
    retry: 2
  });
}

/**
 * Create remote proxy
 */
export function useCreateRemoteProxy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: {
      assetId: string;
      type?: RemoteProxy['type'];
      remoteAddress?: string;
      protocol?: 'tcp' | 'udp' | 'quic';
      port?: number;
      virtualAddress?: string;
      accessLevel?: string;
      trustRequirement?: string;
    }) => hyperMeshAPI.createRemoteProxy(config as Parameters<typeof hyperMeshAPI.createRemoteProxy>[0]),
    onSuccess: (newProxy) => {
      // Update proxies cache
      queryClient.setQueryData(['proxies'], (oldData: RemoteProxy[] | undefined) => {
        return oldData ? [...oldData, newProxy] : [newProxy];
      });
      
      queryClient.invalidateQueries({ queryKey: ['proxies'] });
    }
  });
}

/**
 * Update remote proxy
 */
export function useUpdateRemoteProxy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (args: string | { proxyId: string; updates: Partial<RemoteProxy> } | { proxyAddress: string; trustLevel?: string }) => {
      if (typeof args === 'string') {
        return hyperMeshAPI.updateRemoteProxy(args, {});
      }
      if ('proxyId' in args) {
        return hyperMeshAPI.updateRemoteProxy(args.proxyId, args.updates);
      }
      // Handle { proxyAddress, trustLevel } pattern
      return hyperMeshAPI.updateRemoteProxy(args.proxyAddress, { trust: { level: 50, validatedBy: [], lastValidation: new Date().toISOString() } });
    },
    onSuccess: (updatedProxy) => {
      // Update proxy in cache
      queryClient.setQueryData(['proxies'], (oldData: RemoteProxy[] | undefined) => {
        return oldData?.map(proxy => 
          proxy.id === updatedProxy.id ? updatedProxy : proxy
        );
      });
    }
  });
}

/**
 * Validate proxy trust
 */
export function useValidateProxyTrust() {
  return useMutation({
    mutationFn: (args: string | { proxyAddress: string; trustLevel?: string }) => {
      const proxyId = typeof args === 'string' ? args : args.proxyAddress;
      return hyperMeshAPI.validateProxyTrust(proxyId);
    }
  });
}

/**
 * Get node health status
 */
export function useNodeHealth(nodeId?: string) {
  return useQuery({
    queryKey: ['nodes', 'health', nodeId],
    queryFn: () => hyperMeshAPI.getNodeHealth(nodeId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });
}

/**
 * Get network topology
 */
export function useNetworkTopology() {
  return useQuery({
    queryKey: ['network', 'topology'],
    queryFn: () => hyperMeshAPI.getNetworkTopology(),
    staleTime: 300000, // 5 minutes
    refetchInterval: 600000, // 10 minutes
    retry: 2
  });
}

/**
 * Execute remote operation through proxy
 */
export function useExecuteRemoteOperation() {
  return useMutation({
    mutationFn: (operation: {
      proxyId?: string;
      proxyAddress?: string;
      operation: string;
      parameters?: any;
      params?: any;
      timeout?: number;
    }) => hyperMeshAPI.executeRemoteOperation({
      proxyId: operation.proxyId || '',
      operation: operation.operation,
      parameters: operation.parameters || operation.params,
      timeout: operation.timeout,
      proxyAddress: operation.proxyAddress,
    })
  });
}

/**
 * Get system status with real-time updates
 */
export function useSystemStatus(enablePolling: boolean = false) {
  return useQuery({
    queryKey: ['system', 'status'],
    queryFn: () => hyperMeshAPI.getSystemStatus(),
    staleTime: 30000,
    refetchInterval: enablePolling ? 60000 : false,
    retry: 2
  });
}

// ============================================================================
// VM ASSET HOOKS - Integration with Catalog Module
// ============================================================================

/**
 * Get Catalog applications with HyperMesh integration status
 */
export function useCatalogApplications(filters?: {
  type?: string;
  adapter?: string;
  status?: string;
}) {
  const queryClient = useQueryClient();
  
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
      requiresConsensus?: boolean;
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
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Network and System Hooks - Node health, topology, and remote operations
 *
 * Provides React Query hooks for:
 * - Node health monitoring
 * - Network topology queries
 * - Remote operation execution
 * - System status polling
 */

import { useQuery, useMutation } from '@tanstack/react-query';
import { hyperMeshAPI } from '../services/HyperMeshAPI';

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
export function useHyperMeshSystemStatus(enablePolling: boolean = false) {
  return useQuery({
    queryKey: ['system', 'status'],
    queryFn: () => hyperMeshAPI.getSystemStatus(),
    staleTime: 30000,
    refetchInterval: enablePolling ? 60000 : false,
    retry: 2
  });
}

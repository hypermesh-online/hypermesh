// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Proxy Hooks - Remote proxy management and trust validation
 *
 * Provides React Query hooks for:
 * - Listing and creating remote proxies
 * - Updating proxy configurations
 * - Validating proxy trust levels
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  hyperMeshAPI,
  RemoteProxy,
} from '../services/HyperMeshAPI';

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

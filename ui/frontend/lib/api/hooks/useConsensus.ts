// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Consensus Hooks - Four-proof consensus validation
 *
 * Provides React Query hooks for:
 * - Validating four-proof consensus (PoSp, PoSt, PoWk, PoTm)
 * - Querying consensus history
 * - Submitting individual proofs
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { hyperMeshAPI } from '../services/HyperMeshAPI';

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

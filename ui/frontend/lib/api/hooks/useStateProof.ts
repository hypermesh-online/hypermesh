// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * State Proof Hooks - Four-proof state verification
 *
 * HyperMesh does NOT use consensus. It uses Proof of State -- bilateral binary
 * authentication. Something is either authentic or it is not.
 *
 * Provides React Query hooks for:
 * - Validating four-proof state proofs (PoSp, PoSt, PoWk, PoTm)
 * - Querying state proof history
 * - Submitting individual proofs
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { hyperMeshAPI } from '../services/HyperMeshAPI';

/**
 * Validate four-proof state proof
 */
export function useValidateStateProof() {
  return useMutation({
    mutationFn: ({ assetId, blockId }: { assetId: string; blockId: string }) =>
      hyperMeshAPI.validateStateProof(assetId, blockId),
    onError: (error) => {
      console.error('State proof validation failed:', error);
    }
  });
}

/**
 * Get state proof history for asset
 */
export function useStateProofHistory(assetId: string, limit: number = 100) {
  return useQuery({
    queryKey: ['stateProof', 'history', assetId, limit],
    queryFn: () => hyperMeshAPI.getStateProofHistory(assetId, limit),
    enabled: !!assetId,
    staleTime: 60000,
    retry: 2
  });
}

/**
 * Submit proof for state verification
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
        // Invalidate state proof history for this asset
        queryClient.invalidateQueries({
          queryKey: ['stateProof', 'history', variables.assetId]
        });
      }
    }
  });
}

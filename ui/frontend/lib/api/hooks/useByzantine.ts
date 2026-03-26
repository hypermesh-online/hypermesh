// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Byzantine Detection Hooks - Byzantine fault detection and reporting
 *
 * Provides React Query hooks for:
 * - Monitoring Byzantine behavior detections
 * - Reporting suspected Byzantine behavior
 * - Real-time security alert updates
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  hyperMeshAPI,
  ByzantineDetection,
} from '../services/HyperMeshAPI';

/**
 * Get Byzantine detection results
 */
export function useByzantineDetections(nodeId?: string) {
  const query = useQuery({
    queryKey: ['byzantine', 'detections', nodeId],
    queryFn: () => hyperMeshAPI.getByzantineDetections(nodeId),
    staleTime: 30000,
    refetchInterval: 60000,
    retry: 2
  });

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

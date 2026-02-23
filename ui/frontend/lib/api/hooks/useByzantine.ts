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
import { useEffect, useRef } from 'react';
import {
  hyperMeshAPI,
  ByzantineDetection,
} from '../services/HyperMeshAPI';
import { web3Events } from '../index';

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

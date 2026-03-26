// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Certificate Management Hooks - TrustChain certificate operations
 * 
 * Provides React Query hooks for TrustChain certificate management:
 * - Certificate lifecycle operations
 * - Real-time certificate status updates
 * - Rotation policy management
 * - Trust hierarchy validation
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { trustChainAPI, Certificate, RotationPolicy, ValidationResult, TrustHierarchy } from '../services/TrustChainAPI';

/**
 * Get all certificates with optional filtering
 */
export function useCertificates(filters?: {
  status?: Certificate['status'];
  trustLevel?: Certificate['trustLevel'];
  expiringWithinDays?: number;
}) {
  const query = useQuery({
    queryKey: ['certificates', filters],
    queryFn: async (): Promise<Certificate[]> => {
      const certificates = await trustChainAPI.getCertificates();
      
      // Apply client-side filtering
      let filtered = certificates;
      
      if (filters?.status) {
        filtered = filtered.filter(cert => cert.status === filters.status);
      }
      
      if (filters?.trustLevel) {
        filtered = filtered.filter(cert => cert.trustLevel === filters.trustLevel);
      }
      
      if (filters?.expiringWithinDays) {
        const cutoffDate = new Date();
        cutoffDate.setDate(cutoffDate.getDate() + filters.expiringWithinDays);
        
        filtered = filtered.filter(cert => {
          const validTo = new Date(cert.validTo);
          return validTo <= cutoffDate && cert.status === 'active';
        });
      }
      
      return filtered;
    },
    staleTime: 60000, // 1 minute
    refetchInterval: 300000, // 5 minutes
    retry: 2
  });

  return {
    ...query,
    certificates: query.data || [],
    activeCertificates: query.data?.filter(cert => cert.status === 'active') || [],
    expiringSoon: query.data?.filter(cert => {
      const validTo = new Date(cert.validTo);
      const warningDate = new Date();
      warningDate.setDate(warningDate.getDate() + 30); // 30 days warning
      return validTo <= warningDate && cert.status === 'active';
    }) || []
  };
}

/**
 * Get specific certificate details
 */
export function useCertificate(certificateId: string) {
  return useQuery({
    queryKey: ['certificate', certificateId],
    queryFn: () => trustChainAPI.getCertificate(certificateId),
    enabled: !!certificateId,
    staleTime: 30000,
    retry: 2
  });
}

/**
 * Create new certificate
 */
export function useCreateCertificate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (certificateData: {
      subject: string;
      validityDays: number;
      keySize: number;
      usage: string[];
    }) => trustChainAPI.createCertificate(certificateData),
    onSuccess: (newCertificate) => {
      // Update certificates list
      queryClient.setQueryData(['certificates'], (oldData: Certificate[] | undefined) => {
        return oldData ? [...oldData, newCertificate] : [newCertificate];
      });
      
      // Invalidate certificates queries to ensure consistency
      queryClient.invalidateQueries({ queryKey: ['certificates'] });
    },
    onError: (error) => {
      console.error('Failed to create certificate:', error);
    }
  });
}

/**
 * Revoke certificate
 */
export function useRevokeCertificate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ certificateId, reason }: { certificateId: string; reason: string }) =>
      trustChainAPI.revokeCertificate(certificateId, reason),
    onSuccess: (_, variables) => {
      // Update certificate status in cache
      queryClient.setQueryData(['certificates'], (oldData: Certificate[] | undefined) => {
        return oldData?.map(cert => 
          cert.id === variables.certificateId 
            ? { ...cert, status: 'revoked' as const }
            : cert
        );
      });
      
      // Update specific certificate cache
      queryClient.setQueryData(['certificate', variables.certificateId], (oldData: Certificate | undefined) => {
        return oldData ? { ...oldData, status: 'revoked' as const } : oldData;
      });
    }
  });
}

/**
 * Validate certificate
 */
export function useValidateCertificate(certificateId: string) {
  return useQuery({
    queryKey: ['certificate', certificateId, 'validation'],
    queryFn: () => trustChainAPI.validateCertificate(certificateId),
    enabled: !!certificateId,
    staleTime: 30000,
    retry: 1
  });
}

/**
 * Get trust hierarchy
 */
export function useTrustHierarchy() {
  return useQuery({
    queryKey: ['trust', 'hierarchy'],
    queryFn: () => trustChainAPI.getTrustHierarchy(),
    staleTime: 300000, // 5 minutes
    refetchInterval: 600000, // 10 minutes
    retry: 2
  });
}

/**
 * Get rotation policies
 */
export function useRotationPolicies() {
  return useQuery({
    queryKey: ['rotation', 'policies'],
    queryFn: () => trustChainAPI.getRotationPolicies(),
    staleTime: 60000,
    refetchInterval: 300000,
    retry: 2
  });
}

/**
 * Create rotation policy
 */
export function useCreateRotationPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (policy: Omit<RotationPolicy, 'id'>) => 
      trustChainAPI.createRotationPolicy(policy),
    onSuccess: (newPolicy) => {
      queryClient.setQueryData(['rotation', 'policies'], (oldData: RotationPolicy[] | undefined) => {
        return oldData ? [...oldData, newPolicy] : [newPolicy];
      });
    }
  });
}

/**
 * Update rotation policy
 */
export function useUpdateRotationPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ policyId, updates }: { policyId: string; updates: Partial<RotationPolicy> }) =>
      trustChainAPI.updateRotationPolicy(policyId, updates),
    onSuccess: (updatedPolicy) => {
      queryClient.setQueryData(['rotation', 'policies'], (oldData: RotationPolicy[] | undefined) => {
        return oldData?.map(policy => 
          policy.id === updatedPolicy.id ? updatedPolicy : policy
        );
      });
    }
  });
}

/**
 * Execute manual certificate rotation
 */
export function useRotateCertificate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (certificateId: string) => trustChainAPI.rotateCertificate(certificateId),
    onSuccess: (result) => {
      // Update both old and new certificates in cache
      queryClient.setQueryData(['certificates'], (oldData: Certificate[] | undefined) => {
        if (!oldData) return oldData;
        
        return oldData.map(cert => {
          if (cert.id === result.oldCertificate.id) {
            return result.oldCertificate;
          }
          return cert;
        }).concat(result.newCertificate);
      });
      
      // Invalidate related queries
      queryClient.invalidateQueries({ queryKey: ['certificate', result.oldCertificate.id] });
      queryClient.invalidateQueries({ queryKey: ['rotation', 'history'] });
    }
  });
}

/**
 * Get rotation history
 */
export function useRotationHistory(certificateId?: string) {
  return useQuery({
    queryKey: ['rotation', 'history', certificateId],
    queryFn: () => trustChainAPI.getRotationHistory(certificateId),
    staleTime: 60000,
    retry: 2
  });
}

/**
 * Export certificate
 */
export function useExportCertificate() {
  return useMutation({
    mutationFn: ({ certificateId, format }: { certificateId: string; format: 'pem' | 'der' | 'p12' }) =>
      trustChainAPI.exportCertificate(certificateId, format),
    onSuccess: (blob, variables) => {
      // Create download link
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `certificate-${variables.certificateId}.${variables.format}`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    }
  });
}

/**
 * Import certificate
 */
export function useImportCertificate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ certificateData, format }: { 
      certificateData: string | ArrayBuffer; 
      format: 'pem' | 'der' | 'p12' 
    }) => trustChainAPI.importCertificate(certificateData, format),
    onSuccess: (importedCertificate) => {
      // Add imported certificate to cache
      queryClient.setQueryData(['certificates'], (oldData: Certificate[] | undefined) => {
        return oldData ? [...oldData, importedCertificate] : [importedCertificate];
      });
      
      queryClient.invalidateQueries({ queryKey: ['certificates'] });
    }
  });
}
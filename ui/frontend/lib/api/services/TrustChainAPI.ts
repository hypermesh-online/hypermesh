// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * TrustChain API - Certificate management, DNS resolution, and trust rotation
 * 
 * Provides typed interface for TrustChain service operations:
 * - X.509 certificate lifecycle management
 * - DNS resolution and management
 * - Certificate rotation and renewal
 * - Trust hierarchy validation
 */

import { web3ApiClient } from '../index';
import type { ServiceType } from '../Web3APIClient';

export interface Certificate {
  id: string;
  subject: string;
  issuer: string;
  serialNumber: string;
  commonName?: string;
  signatureAlgorithm?: string;
  validFrom: string;
  validTo: string;
  fingerprint: string;
  publicKey: string;
  status: 'active' | 'expired' | 'revoked' | 'pending';
  trustLevel: 'root' | 'intermediate' | 'leaf';
}

export interface DNSRecord {
  id: string;
  domain: string;
  type: 'A' | 'AAAA' | 'CNAME' | 'TXT' | 'MX' | 'SRV';
  value: string;
  ttl: number;
  priority?: number;
  weight?: number;
  port?: number;
  lastUpdated: string;
  status: 'active' | 'pending' | 'failed';
}

export interface TrustHierarchy {
  rootCA: Certificate;
  intermediates: Certificate[];
  leaves: Certificate[];
  validationChain: string[];
  lastValidated: string;
}

export interface RotationPolicy {
  id: string;
  certificateId: string;
  rotationType: 'automatic' | 'manual' | 'emergency';
  schedule: {
    intervalDays: number;
    warningDays: number;
    gracePeriodDays: number;
  };
  enabled: boolean;
  lastRotation?: string;
  nextRotation?: string;
}

export interface ValidationResult {
  valid: boolean;
  certificateId: string;
  validationPath: string[];
  errors: string[];
  warnings: string[];
  validatedAt: string;
}

export class TrustChainAPI {
  private readonly service: ServiceType = 'trustchain';

  /**
   * Get all certificates in the trust store
   */
  async getCertificates(): Promise<Certificate[]> {
    return web3ApiClient.request<Certificate[]>(this.service, '/api/v1/trustchain/certificates');
  }

  /**
   * Get specific certificate by ID
   */
  async getCertificate(certificateId: string): Promise<Certificate> {
    return web3ApiClient.request<Certificate>(this.service, `/api/v1/trustchain/certificates/${certificateId}`);
  }

  /**
   * Create new certificate
   */
  async createCertificate(certificateData: {
    subject: string;
    validityDays: number;
    keySize: number;
    usage: string[];
  }): Promise<Certificate> {
    return web3ApiClient.request<Certificate>(this.service, '/api/v1/trustchain/certificates', {
      method: 'POST',
      body: certificateData
    });
  }

  /**
   * Revoke certificate
   */
  async revokeCertificate(certificateId: string, reason: string): Promise<void> {
    await web3ApiClient.request(this.service, `/api/v1/trustchain/certificates/${certificateId}/revoke`, {
      method: 'POST',
      body: { reason }
    });
  }

  /**
   * Validate certificate chain
   */
  async validateCertificate(certificateId: string): Promise<ValidationResult> {
    return web3ApiClient.request<ValidationResult>(this.service, `/api/v1/trustchain/certificates/${certificateId}/validate`);
  }

  /**
   * Get trust hierarchy
   */
  async getTrustHierarchy(): Promise<TrustHierarchy> {
    return web3ApiClient.request<TrustHierarchy>(this.service, '/api/v1/trustchain/trust/hierarchy');
  }

  /**
   * Get DNS records
   */
  async getDNSRecords(domain?: string): Promise<DNSRecord[]> {
    const endpoint = domain ? `/api/v1/trustchain/dns/records?domain=${encodeURIComponent(domain)}` : '/api/v1/trustchain/dns/records';
    return web3ApiClient.request<DNSRecord[]>(this.service, endpoint);
  }

  /**
   * Create DNS record
   */
  async createDNSRecord(record: Omit<DNSRecord, 'id' | 'lastUpdated' | 'status'>): Promise<DNSRecord> {
    return web3ApiClient.request<DNSRecord>(this.service, '/api/v1/trustchain/dns/records', {
      method: 'POST',
      body: record
    });
  }

  /**
   * Update DNS record
   */
  async updateDNSRecord(recordId: string, updates: Partial<DNSRecord>): Promise<DNSRecord> {
    return web3ApiClient.request<DNSRecord>(this.service, `/api/v1/trustchain/dns/records/${recordId}`, {
      method: 'PUT',
      body: updates
    });
  }

  /**
   * Delete DNS record
   */
  async deleteDNSRecord(recordId: string): Promise<void> {
    await web3ApiClient.request(this.service, `/api/v1/trustchain/dns/records/${recordId}`, {
      method: 'DELETE'
    });
  }

  /**
   * Resolve domain
   */
  async resolveDomain(domain: string, recordType: string = 'A'): Promise<DNSRecord[]> {
    return web3ApiClient.request<DNSRecord[]>(this.service, `/api/v1/trustchain/dns/resolve`, {
      method: 'POST',
      body: { domain, type: recordType }
    });
  }

  /**
   * Get rotation policies
   */
  async getRotationPolicies(): Promise<RotationPolicy[]> {
    return web3ApiClient.request<RotationPolicy[]>(this.service, '/api/v1/trustchain/rotation/policies');
  }

  /**
   * Create rotation policy
   */
  async createRotationPolicy(policy: Omit<RotationPolicy, 'id'>): Promise<RotationPolicy> {
    return web3ApiClient.request<RotationPolicy>(this.service, '/api/v1/trustchain/rotation/policies', {
      method: 'POST',
      body: policy
    });
  }

  /**
   * Update rotation policy
   */
  async updateRotationPolicy(policyId: string, updates: Partial<RotationPolicy>): Promise<RotationPolicy> {
    return web3ApiClient.request<RotationPolicy>(this.service, `/api/v1/trustchain/rotation/policies/${policyId}`, {
      method: 'PUT',
      body: updates
    });
  }

  /**
   * Execute manual rotation
   */
  async rotateCertificate(certificateId: string): Promise<{
    oldCertificate: Certificate;
    newCertificate: Certificate;
    rotationId: string;
  }> {
    return web3ApiClient.request(this.service, `/api/v1/trustchain/rotation/execute`, {
      method: 'POST',
      body: { certificateId }
    });
  }

  /**
   * Get rotation history
   */
  async getRotationHistory(certificateId?: string): Promise<Array<{
    id: string;
    certificateId: string;
    rotationType: string;
    executedAt: string;
    oldFingerprint: string;
    newFingerprint: string;
    status: 'success' | 'failed' | 'partial';
  }>> {
    const endpoint = certificateId 
      ? `/api/v1/trustchain/rotation/history?certificateId=${certificateId}`
      : '/api/v1/trustchain/rotation/history';
    return web3ApiClient.request(this.service, endpoint);
  }

  /**
   * Get TrustChain health status - connects to real backend /health endpoint
   */
  async getHealthStatus(): Promise<{
    status: 'healthy' | 'warning' | 'critical';
    certificateCount: number;
    expiringSoon: number;
    revokedCount: number;
    dnsRecordCount: number;
    rotationPolicyCount: number;
    lastValidation: string;
    uptime: number;
  }> {
    try {
      // Call real TrustChain health endpoint
      const healthResponse = await web3ApiClient.request<{
        status: string;
        timestamp: string;
        version: string;
        services: {
          ca: boolean;
          ct: boolean;
          dns: boolean;
          stateProof: boolean;
        };
      }>(this.service, '/api/v1/trustchain/health');

      // Get additional stats from the /stats endpoint
      const statsResponse = await web3ApiClient.request<{
        requests_total: number;
        requests_successful: number;
        requests_failed: number;
        ca_requests: number;
        ct_requests: number;
        dns_requests: number;
        average_response_time_ms: number;
        active_connections: number;
        rate_limited_requests: number;
        last_update: string;
      }>(this.service, '/api/v1/trustchain/stats');

      // Calculate uptime percentage from success rate
      const uptime = statsResponse.requests_total > 0 
        ? (statsResponse.requests_successful / statsResponse.requests_total) * 100
        : 100;

      // Determine overall status from service health
      const allServicesHealthy = Object.values(healthResponse.services).every(service => service);
      const overallStatus = allServicesHealthy ? 'healthy' : 'warning';

      // Fetch additional metrics from the new endpoints
      let expiringSoon = 0;
      let revokedCount = 0;
      let rotationPolicyCount = 0;

      try {
        // Get expiring certificates count
        const expiringResponse = await web3ApiClient.request<{
          certificates: Certificate[],
          count: number,
          days_threshold: number
        }>(this.service, '/api/v1/trustchain/certificates/expiring?days=30');
        expiringSoon = expiringResponse.count;

        // Get revoked certificates count
        const revokedResponse = await web3ApiClient.request<{
          count: number,
          revocations: Array<any>
        }>(this.service, '/api/v1/trustchain/certificates/revoked');
        revokedCount = revokedResponse.count;

        // Get rotation policies count
        const policiesResponse = await web3ApiClient.request<{
          policies: Array<any>,
          count: number
        }>(this.service, '/api/v1/trustchain/policies/rotation');
        rotationPolicyCount = policiesResponse.count;
      } catch (error) {
        console.warn('Failed to fetch additional TrustChain metrics:', error);
      }

      return {
        status: overallStatus,
        certificateCount: statsResponse.ca_requests, // Use CA requests as proxy for cert count
        expiringSoon: expiringSoon,
        revokedCount: revokedCount,
        dnsRecordCount: statsResponse.dns_requests,
        rotationPolicyCount: rotationPolicyCount,
        lastValidation: healthResponse.timestamp,
        uptime: uptime
      };
    } catch (error) {
      console.error('Failed to get TrustChain health status:', error);
      // Return offline status if backend is not available
      return {
        status: 'critical',
        certificateCount: 0,
        expiringSoon: 0,
        revokedCount: 0,
        dnsRecordCount: 0,
        rotationPolicyCount: 0,
        lastValidation: new Date().toISOString(),
        uptime: 0
      };
    }
  }

  /**
   * Get root certificate for authentication
   */
  async getRootCertificate(): Promise<{
    certificate: string;
    format: string;
    fingerprint: string;
    status?: string;
  }> {
    return web3ApiClient.request(this.service, '/api/v1/trustchain/certificates/root');
  }

  /**
   * Export certificate in various formats
   */
  async exportCertificate(certificateId: string, format: 'pem' | 'der' | 'p12'): Promise<Blob> {
    const response = await web3ApiClient.request<ArrayBuffer>(this.service, 
      `/api/v1/trustchain/certificates/${certificateId}/export?format=${format}`, {
      headers: { 'Accept': 'application/octet-stream' }
    });
    
    return new Blob([response], { type: 'application/octet-stream' });
  }

  /**
   * Import certificate
   */
  async importCertificate(certificateData: string | ArrayBuffer, format: 'pem' | 'der' | 'p12'): Promise<Certificate> {
    const body = typeof certificateData === 'string' 
      ? { certificate: certificateData, format }
      : { certificate: Array.from(new Uint8Array(certificateData)), format };

    return web3ApiClient.request<Certificate>(this.service, '/api/v1/trustchain/certificates/import', {
      method: 'POST',
      body
    });
  }
}

// Singleton instance
export const trustChainAPI = new TrustChainAPI();
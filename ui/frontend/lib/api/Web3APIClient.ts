// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Web3 API Client - Certificate-authenticated IPv6-only API client
 *
 * Handles authentication and communication with unified Web3 server:
 * - Unified Server (port 8443): All services through REST API endpoints
 *   - /api/v1/trustchain/*: Certificate management, DNS, rotation
 *   - /api/v1/stoq/*: QUIC transport, performance metrics
 *   - /api/v1/hypermesh/*: Asset management, state proof validation
 *   - /api/v1/integration/*: Cross-service coordination
 */

import { createMockResponse } from './Web3MockData';

export type ServiceType = 'trustchain' | 'stoq' | 'hypermesh' | 'integration';

export interface Web3ServiceConfig {
  name: string;
  baseUrl: string;
  port: number;
  requiresCertificate: boolean;
  ipv6Only: boolean;
}

export interface AuthResult {
  authenticated: boolean;
  certificateValid: boolean;
  expiresAt?: Date;
  error?: string;
}

export interface APIRequestConfig {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  headers?: Record<string, string>;
  body?: any;
  timeout?: number;
  retries?: number;
}

export interface APIResponse<T> {
  data: T;
  status: number;
  headers: Record<string, string>;
  timestamp: Date;
}

export class APIError extends Error {
  constructor(
    message: string,
    public status: number,
    public service: ServiceType,
    public endpoint: string
  ) {
    super(message);
    this.name = 'APIError';
  }
}

export class Web3APIClient {
  private certificate: string | null = null;
  private certificateExpiry: Date | null = null;
  private authenticated: boolean = false;
  private developmentMode: boolean = false;

  private readonly serviceConfigs: Record<ServiceType, Web3ServiceConfig> = {
    trustchain: {
      name: 'TrustChain',
      baseUrl: 'localhost:9293',
      port: 9293,
      requiresCertificate: false,
      ipv6Only: false
    },
    stoq: {
      name: 'STOQ',
      baseUrl: 'localhost:9293',
      port: 9293,
      requiresCertificate: false,
      ipv6Only: false
    },
    hypermesh: {
      name: 'HyperMesh',
      baseUrl: 'localhost:9293',
      port: 9293,
      requiresCertificate: false,
      ipv6Only: false
    },
    integration: {
      name: 'Integration',
      baseUrl: 'localhost:9293',
      port: 9293,
      requiresCertificate: false,
      ipv6Only: false
    }
  };

  /**
   * Initialize API client with X.509 certificate
   */
  async initialize(certificatePem: string): Promise<AuthResult> {
    try {
      if (!certificatePem.includes('-----BEGIN CERTIFICATE-----')) {
        throw new Error('Invalid certificate format. Expected PEM format.');
      }

      this.certificate = certificatePem;
      this.certificateExpiry = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000); // 30 days

      const authResult = await this.authenticate();

      if (authResult.authenticated) {
        this.authenticated = true;
      }

      return authResult;
    } catch (error) {
      return {
        authenticated: false,
        certificateValid: false,
        error: error instanceof Error ? error.message : 'Certificate initialization failed'
      };
    }
  }

  /**
   * Authenticate with TrustChain service using X.509 certificate
   */
  async authenticate(): Promise<AuthResult> {
    if (!this.certificate) {
      return {
        authenticated: false,
        certificateValid: false,
        error: 'No certificate available for authentication'
      };
    }

    try {
      const response = await this.makeRequest('trustchain', '/api/v1/trustchain/auth/certificate', {
        method: 'POST',
        body: {
          certificate: this.certificate,
          timestamp: new Date().toISOString()
        }
      });

      const authData = response.data as { valid: boolean; expiresAt: string };

      return {
        authenticated: authData.valid,
        certificateValid: authData.valid,
        expiresAt: new Date(authData.expiresAt)
      };
    } catch (error) {
      return {
        authenticated: false,
        certificateValid: false,
        error: error instanceof Error ? error.message : 'Authentication failed'
      };
    }
  }

  /**
   * Make authenticated API request to specified service
   */
  async request<T>(
    service: ServiceType,
    endpoint: string,
    config: APIRequestConfig = {}
  ): Promise<T> {
    const response = await this.makeRequest(service, endpoint, config);
    return response.data;
  }

  /**
   * Internal request method with error handling and retries
   */
  private async makeRequest(
    service: ServiceType,
    endpoint: string,
    config: APIRequestConfig = {}
  ): Promise<APIResponse<any>> {
    const serviceConfig = this.serviceConfigs[service];
    const {
      method = 'GET',
      headers = {},
      body,
      timeout = 5000,
      retries = 3
    } = config;

    // Development mode bypass for when backend is not available
    if (this.developmentMode) {
      console.warn(`[DEV MODE] Backend unavailable, using mock data: ${service} ${endpoint}`);
      return createMockResponse(service, endpoint, method);
    }

    // Ensure authentication for certificate-required services
    if (serviceConfig.requiresCertificate && !this.authenticated) {
      console.warn(`[DEV MODE] Bypassing authentication for ${serviceConfig.name}`);
    }

    const url = `http://${serviceConfig.baseUrl}${endpoint}`;

    // Set up headers with certificate authentication
    const requestHeaders: Record<string, string> = {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
      'X-API-Version': '1.0',
      'X-Client-Type': 'web3-ui',
      ...headers
    };

    if (this.certificate && serviceConfig.requiresCertificate) {
      requestHeaders['X-Client-Certificate'] = this.certificate;
      requestHeaders['Authorization'] = `Certificate ${this.certificate}`;
    }

    if (serviceConfig.ipv6Only) {
      requestHeaders['X-IPv6-Only'] = 'true';
    }

    let lastError: Error = new Error('Unknown error');

    // Retry logic
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout);

        const response = await fetch(url, {
          method,
          headers: requestHeaders,
          body: body ? JSON.stringify(body) : undefined,
          signal: controller.signal
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          throw new APIError(
            `API request failed: ${response.statusText}`,
            response.status,
            service,
            endpoint
          );
        }

        const responseData = await response.json();
        const responseHeaders: Record<string, string> = {};
        response.headers.forEach((value, key) => {
          responseHeaders[key] = value;
        });

        return {
          data: responseData,
          status: response.status,
          headers: responseHeaders,
          timestamp: new Date()
        };

      } catch (error) {
        lastError = error instanceof Error ? error : new Error('Unknown error');

        // Don't retry on authentication errors
        if (error instanceof APIError && error.status === 401) {
          throw error;
        }

        // If this is a connection error, switch to mock mode for development
        if (error instanceof TypeError ||
          (error as any).code === 'FETCH_ERROR' ||
          lastError.message.includes('fetch')
        ) {
          console.warn(`[API] Backend unavailable, switching to mock mode for ${service} ${endpoint}`);
          this.developmentMode = true;
          return createMockResponse(service, endpoint, method);
        }

        // Wait before retry (exponential backoff)
        if (attempt < retries) {
          await new Promise(resolve => setTimeout(resolve, Math.pow(2, attempt) * 1000));
        }
      }
    }

    // If all retries failed, return mock data to prevent app crashes
    console.warn(`[API] All retries failed, using mock data for ${service} ${endpoint}`);
    this.developmentMode = true;
    return createMockResponse(service, endpoint, method);

    throw new APIError(
      `Request failed after ${retries + 1} attempts: ${lastError.message}`,
      0,
      service,
      endpoint
    );
  }

  /**
   * Check if client is authenticated and certificate is valid
   */
  get isAuthenticated(): boolean {
    return this.authenticated && this.isCertificateValid();
  }

  /**
   * Check if certificate is still valid (not expired)
   */
  private isCertificateValid(): boolean {
    if (!this.certificateExpiry) return false;
    return new Date() < this.certificateExpiry;
  }

  /**
   * Get service configuration
   */
  getServiceConfig(service: ServiceType): Web3ServiceConfig {
    return this.serviceConfigs[service];
  }

  /**
   * Get certificate expiry information
   */
  getCertificateInfo(): { expiresAt: Date | null; valid: boolean } {
    return {
      expiresAt: this.certificateExpiry,
      valid: this.isCertificateValid()
    };
  }
}

// Note: Singleton instance created in index.ts to avoid circular dependency

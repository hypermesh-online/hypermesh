// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * STOQ Native Protocol Client
 * 
 * Replaces HTTP-based Web3 API client with pure STOQ protocol communication.
 * Uses WebAssembly STOQ client for direct QUIC connections with TrustChain
 * certificate authentication - true Internet 2.0 architecture.
 */

import { StoqWasmClient, createStoqWasmClient, type StoqWasmConfig } from './StoqWasmClient';
import type { WasmConnectionStatus } from '../stoq-wasm';
import { getConfig } from '../config';

export type ServiceType = 'trustchain' | 'stoq' | 'hypermesh' | 'catalog' | 'caesar' | 'integration';

export interface StoqAuthResult {
  authenticated: boolean;
  certificateValid: boolean;
  connectionId?: string;
  expiresAt?: Date;
  error?: string;
}

export interface StoqRequestConfig {
  timeout?: number;
  retries?: number;
  correlationId?: string;
}

export interface StoqResponse<T> {
  status: 'success' | 'error';
  data?: T;
  error?: string;
  timestamp: string;
  correlationId?: string;
}

export class StoqNativeAPIError extends Error {
  constructor(
    message: string,
    public service: ServiceType,
    public messageType: string,
    public correlationId?: string
  ) {
    super(message);
    this.name = 'StoqNativeAPIError';
  }
}

/**
 * Main STOQ Native API Client for Internet 2.0 communication
 */
export class StoqNativeClient {
  private stoqClient: StoqWasmClient | null = null;
  private certificate: string | null = null;
  private authenticated: boolean = false;
  private pendingRequests = new Map<string, {
    resolve: (value: any) => void;
    reject: (error: any) => void;
    timeout: number;
  }>();
  private messageHandlers = new Map<string, (payload: any) => void>();

  private readonly config: StoqWasmConfig;

  constructor() {
    const appConfig = getConfig();
    this.config = {
      serverAddress: appConfig.stoq.serverAddress,
      serverPort: appConfig.stoq.serverPort,
      certificatePem: '',   // Will be set during initialization
      serverName: appConfig.stoq.serverName,
      autoReconnect: true,
      reconnectIntervalMs: 5000,
      maxReconnectAttempts: 10,
      timeoutMs: 30000,
    };
  }

  /**
   * Initialize the STOQ native client with TrustChain certificate
   */
  async initialize(certificatePem: string): Promise<StoqAuthResult> {
    try {
      const appConfig = getConfig();
      console.log('Initializing STOQ Native client with TrustChain certificate...');
      console.log(`Connecting to: ${this.config.serverAddress}:${this.config.serverPort}`);
      console.log(`Environment: ${appConfig.environment}`);

      // Validate certificate format
      if (!certificatePem.includes('-----BEGIN CERTIFICATE-----')) {
        throw new Error('Invalid certificate format. Expected PEM format.');
      }

      this.certificate = certificatePem;
      
      // Update config with certificate
      const clientConfig = {
        ...this.config,
        certificatePem,
      };

      // Create STOQ WASM client
      this.stoqClient = createStoqWasmClient(clientConfig, {
        onStatusChange: (status, connectionId) => this.handleStatusChange(status, connectionId),
        onMessage: (messageType, payload) => this.handleMessage(messageType, payload),
        onError: (error, details) => this.handleError(error, details),
        onConnect: (connectionId) => this.handleConnect(connectionId),
        onDisconnect: (reason) => this.handleDisconnect(reason),
      });

      // Initialize WASM client
      await this.stoqClient.initialize();

      // Connect to STOQ server
      await this.stoqClient.connect();

      // Wait for authentication
      await this.waitForAuthentication();

      this.authenticated = true;

      console.log('STOQ Native client initialized and authenticated successfully');

      return {
        authenticated: true,
        certificateValid: true,
        connectionId: this.stoqClient.getConnectionId() || undefined,
        expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000), // 30 days
      };

    } catch (error) {
      console.error('Failed to initialize STOQ Native client:', error);
      return {
        authenticated: false,
        certificateValid: false,
        error: error instanceof Error ? error.message : 'Initialization failed'
      };
    }
  }

  /**
   * Send a request through STOQ protocol and wait for response
   */
  async request<T>(service: ServiceType, messageType: string, payload: any, config?: StoqRequestConfig): Promise<T> {
    if (!this.stoqClient || !this.authenticated) {
      throw new StoqNativeAPIError('Not connected or authenticated', service, messageType);
    }

    const correlationId = config?.correlationId || this.generateCorrelationId();
    const timeout = config?.timeout || 10000; // 10 seconds default

    return new Promise((resolve, reject) => {
      // Set up timeout
      const timeoutHandle = setTimeout(() => {
        this.pendingRequests.delete(correlationId);
        reject(new StoqNativeAPIError('Request timeout', service, messageType, correlationId));
      }, timeout);

      // Store request for response handling
      this.pendingRequests.set(correlationId, {
        resolve,
        reject,
        timeout: timeoutHandle,
      });

      // Send request message
      const requestMessage = {
        service,
        messageType,
        payload,
        correlationId,
        timestamp: new Date().toISOString(),
      };

      this.stoqClient!.sendMessage(`${service}_request`, requestMessage)
        .catch((error) => {
          this.pendingRequests.delete(correlationId);
          clearTimeout(timeoutHandle);
          reject(new StoqNativeAPIError(
            `Failed to send request: ${error.message}`,
            service,
            messageType,
            correlationId
          ));
        });
    });
  }

  /**
   * Register a handler for specific message types
   */
  registerMessageHandler(messageType: string, handler: (payload: any) => void): void {
    this.messageHandlers.set(messageType, handler);
    
    if (this.stoqClient) {
      this.stoqClient.registerMessageHandler(messageType, handler);
    }
  }

  /**
   * Get system status from all services
   */
  async getSystemStatus(): Promise<any> {
    if (!this.stoqClient) {
      throw new Error('STOQ client not initialized');
    }

    return new Promise((resolve, reject) => {
      const correlationId = this.generateCorrelationId();
      const timeout = setTimeout(() => {
        reject(new Error('System status request timeout'));
      }, 10000);

      // Register one-time handler for response
      this.stoqClient!.registerMessageHandler('system_status_response', (payload: any) => {
        clearTimeout(timeout);
        resolve(payload);
      });

      // Send system status request
      this.stoqClient!.requestSystemStatus().catch(reject);
    });
  }

  /**
   * Get performance metrics
   */
  async getPerformanceMetrics(timeRange = '1h'): Promise<any> {
    if (!this.stoqClient) {
      throw new Error('STOQ client not initialized');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Performance metrics request timeout'));
      }, 10000);

      // Register one-time handler for response
      this.stoqClient!.registerMessageHandler('performance_metrics_response', (payload: any) => {
        clearTimeout(timeout);
        resolve(payload);
      });

      // Send performance metrics request
      this.stoqClient!.requestPerformanceMetrics(timeRange).catch(reject);
    });
  }

  /**
   * Get dashboard data
   */
  async getDashboardData(dashboardType: string): Promise<any> {
    if (!this.stoqClient) {
      throw new Error('STOQ client not initialized');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Dashboard data request timeout'));
      }, 10000);

      // Register one-time handler for response
      this.stoqClient!.registerMessageHandler('dashboard_response', (payload: any) => {
        clearTimeout(timeout);
        resolve(payload);
      });

      // Send dashboard data request
      this.stoqClient!.requestDashboardData(dashboardType).catch(reject);
    });
  }

  /**
   * Check if client is connected and authenticated
   */
  isAuthenticated(): boolean {
    return this.authenticated && this.stoqClient?.isConnected() === true;
  }

  /**
   * Get connection status
   */
  getConnectionStatus(): WasmConnectionStatus | null {
    return this.stoqClient?.getStatus() || null;
  }

  /**
   * Get connection ID
   */
  getConnectionId(): string | null {
    return this.stoqClient?.getConnectionId() || null;
  }

  /**
   * Disconnect and cleanup
   */
  async disconnect(): Promise<void> {
    if (this.stoqClient) {
      await this.stoqClient.disconnect();
      this.stoqClient.destroy();
      this.stoqClient = null;
    }

    this.authenticated = false;
    this.certificate = null;

    // Clear pending requests
    for (const [correlationId, request] of this.pendingRequests.entries()) {
      clearTimeout(request.timeout);
      request.reject(new Error('Client disconnected'));
    }
    this.pendingRequests.clear();

    console.log('STOQ Native client disconnected');
  }

  /**
   * Generate unique correlation ID for request tracking
   */
  private generateCorrelationId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Wait for authentication to complete
   */
  private async waitForAuthentication(timeoutMs = 30000): Promise<void> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeoutMs) {
      if (this.stoqClient?.getStatus() === 4) { // Authenticated
        return;
      }
      
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    throw new Error('Authentication timeout');
  }

  /**
   * Handle connection status changes
   */
  private handleStatusChange(status: WasmConnectionStatus, connectionId?: string): void {
    console.log(`STOQ connection status changed: ${status}${connectionId ? ` (${connectionId})` : ''}`);
    
    if (status === 4) { // Authenticated
      this.authenticated = true;
    } else if (status === 0 || status === 5) { // Disconnected or Error
      this.authenticated = false;
    }
  }

  /**
   * Handle incoming messages
   */
  private handleMessage(messageType: string, payload: any): void {
    console.log(`Received STOQ message: ${messageType}`, payload);

    // Handle response messages
    if (payload.correlationId && this.pendingRequests.has(payload.correlationId)) {
      const request = this.pendingRequests.get(payload.correlationId)!;
      this.pendingRequests.delete(payload.correlationId);
      clearTimeout(request.timeout);

      if (payload.status === 'success') {
        request.resolve(payload.data || payload);
      } else {
        request.reject(new Error(payload.error || 'Request failed'));
      }
      return;
    }

    // Handle other message types
    const handler = this.messageHandlers.get(messageType);
    if (handler) {
      handler(payload);
    }
  }

  /**
   * Handle connection errors
   */
  private handleError(error: string, details?: string): void {
    console.error('STOQ connection error:', error, details);
    this.authenticated = false;
  }

  /**
   * Handle successful connection
   */
  private handleConnect(connectionId: string): void {
    console.log(`STOQ connected with ID: ${connectionId}`);
  }

  /**
   * Handle disconnection
   */
  private handleDisconnect(reason?: string): void {
    console.log(`STOQ disconnected: ${reason || 'Unknown reason'}`);
    this.authenticated = false;
  }
}

/**
 * Singleton instance for global use
 */
export const stoqNativeClient = new StoqNativeClient();

/**
 * Initialize STOQ native client with certificate
 */
export async function initializeStoqNative(certificatePem: string): Promise<StoqAuthResult> {
  return stoqNativeClient.initialize(certificatePem);
}

/**
 * Check if STOQ native client is available
 */
export function isStoqNativeAvailable(): boolean {
  return typeof WebAssembly !== 'undefined' && 'instantiate' in WebAssembly;
}
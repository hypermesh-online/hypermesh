// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { StoqWasmClient, StoqWasmConfig, StoqEventCallbacks } from './StoqWasmClient';
import { BehaviorSubject, Observable } from 'rxjs';

export interface SystemStatus {
  services: {
    [key: string]: {
      status: 'healthy' | 'degraded' | 'offline';
      uptime: number;
      lastHealthCheck: string;
    };
  };
  overall: 'healthy' | 'degraded' | 'critical';
}

export interface PerformanceMetrics {
  throughput: {
    download: number;
    upload: number;
    efficiency: number;
  };
  latency: {
    rtt: number;
    packetLoss: number;
  };
  timestamp: string;
}

export interface Asset {
  id: string;
  type: 'CPU' | 'GPU' | 'Memory' | 'Storage';
  status: 'available' | 'allocated' | 'maintenance';
  proxyAddress: string;
  consensusProof?: string;
}

export interface AssetAllocation {
  id: string;
  assetId: string;
  status: 'active' | 'pending' | 'completed';
  allocatedAt: string;
}

export interface ByzantineDetection {
  nodeId: string;
  behaviour: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  status: 'detected' | 'investigating' | 'resolved';
  detectedAt: string;
}

export interface QUICConnection {
  id: string;
  status: 'active' | 'idle' | 'closed';
  throughput: number;
  latency: number;
  createdAt: string;
}

/**
 * STOQ-native data provider for dashboard components
 * Replaces HTTP API calls with pure STOQ protocol messaging
 */
export class StoqDataProvider {
  private stoqClient: StoqWasmClient | null = null;
  private isInitialized = false;
  
  // Data streams using RxJS observables for real-time updates
  private systemStatusSubject = new BehaviorSubject<SystemStatus | null>(null);
  private performanceMetricsSubject = new BehaviorSubject<PerformanceMetrics | null>(null);
  private assetsSubject = new BehaviorSubject<Asset[]>([]);
  private allocationsSubject = new BehaviorSubject<AssetAllocation[]>([]);
  private byzantineDetectionsSubject = new BehaviorSubject<ByzantineDetection[]>([]);
  private quicConnectionsSubject = new BehaviorSubject<QUICConnection[]>([]);
  
  // Expose observables for components to subscribe to
  public systemStatus$ = this.systemStatusSubject.asObservable();
  public performanceMetrics$ = this.performanceMetricsSubject.asObservable();
  public assets$ = this.assetsSubject.asObservable();
  public allocations$ = this.allocationsSubject.asObservable();
  public byzantineDetections$ = this.byzantineDetectionsSubject.asObservable();
  public quicConnections$ = this.quicConnectionsSubject.asObservable();

  constructor() {
    console.log('🚀 Initializing STOQ Data Provider for pure protocol communication');
  }

  /**
   * Initialize STOQ client with certificate authentication
   */
  async initialize(certificatePem: string): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    try {
      console.log('🔗 Setting up STOQ client for dashboard data streaming...');

      const config: StoqWasmConfig = {
        serverAddress: '[::1]',
        serverPort: 8443,
        certificatePem: certificatePem,
        autoReconnect: true,
        reconnectIntervalMs: 5000,
        maxReconnectAttempts: 10,
        timeoutMs: 30000
      };

      const callbacks: StoqEventCallbacks = {
        onStatusChange: (status, connectionId) => {
          console.log('📡 STOQ connection status:', status.is_connected && status.is_authenticated ? 'Connected' : 'Disconnected');
          if (status.is_connected && status.is_authenticated) {
            this.startDataStreaming();
          }
        },
        onMessage: (messageType, payload) => {
          this.handleIncomingMessage(messageType, payload);
        },
        onError: (error, details) => {
          console.error('❌ STOQ data provider error:', error, details);
        },
        onConnect: (connectionId) => {
          console.log('✅ STOQ data provider connected:', connectionId);
        },
        onDisconnect: (reason) => {
          console.log('🔌 STOQ data provider disconnected:', reason);
        }
      };

      this.stoqClient = new StoqWasmClient(config, callbacks);
      await this.stoqClient.initialize();
      await this.stoqClient.connect();
      
      this.isInitialized = true;
      console.log('✅ STOQ Data Provider initialized successfully');
      
    } catch (error) {
      console.error('❌ Failed to initialize STOQ Data Provider:', error);
      throw error;
    }
  }

  /**
   * Start requesting data streams from STOQ server
   */
  private async startDataStreaming(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      console.warn('STOQ client not connected, cannot start data streaming');
      return;
    }

    console.log('📊 Starting real-time data streaming via STOQ protocol...');

    try {
      // Request initial data
      await this.requestSystemStatus();
      await this.requestPerformanceMetrics();
      await this.requestAssets();
      await this.requestAllocations();
      await this.requestByzantineDetections();
      await this.requestQUICConnections();

      // Set up periodic updates
      setInterval(() => this.requestSystemStatus(), 5000); // Every 5 seconds
      setInterval(() => this.requestPerformanceMetrics(), 2000); // Every 2 seconds
      setInterval(() => this.requestAssets(), 10000); // Every 10 seconds
      setInterval(() => this.requestAllocations(), 8000); // Every 8 seconds
      setInterval(() => this.requestByzantineDetections(), 15000); // Every 15 seconds
      setInterval(() => this.requestQUICConnections(), 3000); // Every 3 seconds

      console.log('✅ Real-time data streaming started');
    } catch (error) {
      console.error('❌ Failed to start data streaming:', error);
    }
  }

  /**
   * Handle incoming STOQ messages and update data streams
   */
  private handleIncomingMessage(messageType: string, payload: any): void {
    try {
      console.log('📨 Received STOQ message:', messageType);

      switch (messageType) {
        case 'system_status_response':
          this.systemStatusSubject.next(payload.data);
          break;

        case 'performance_metrics_response':
          this.performanceMetricsSubject.next(payload.data);
          break;

        case 'assets_response':
          this.assetsSubject.next(payload.data || []);
          break;

        case 'allocations_response':
          this.allocationsSubject.next(payload.data || []);
          break;

        case 'byzantine_detections_response':
          this.byzantineDetectionsSubject.next(payload.data || []);
          break;

        case 'quic_connections_response':
          this.quicConnectionsSubject.next(payload.data || []);
          break;

        case 'dashboard_update':
          // Handle real-time dashboard updates
          if (payload.type === 'system_status') {
            this.systemStatusSubject.next(payload.data);
          } else if (payload.type === 'performance_metrics') {
            this.performanceMetricsSubject.next(payload.data);
          }
          break;

        default:
          console.log('🔍 Unknown STOQ message type:', messageType);
      }
    } catch (error) {
      console.error('❌ Error handling STOQ message:', error);
    }
  }

  /**
   * Request system status via STOQ protocol
   */
  async requestSystemStatus(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('system_status_request', {
        timestamp: new Date().toISOString(),
        requestId: `status-${Date.now()}`
      });
    } catch (error) {
      console.error('❌ Failed to request system status:', error);
    }
  }

  /**
   * Request performance metrics via STOQ protocol
   */
  async requestPerformanceMetrics(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('performance_metrics_request', {
        timestamp: new Date().toISOString(),
        requestId: `metrics-${Date.now()}`,
        includeHistorical: false
      });
    } catch (error) {
      console.error('❌ Failed to request performance metrics:', error);
    }
  }

  /**
   * Request assets via STOQ protocol
   */
  async requestAssets(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('assets_request', {
        timestamp: new Date().toISOString(),
        requestId: `assets-${Date.now()}`,
        includeConsensusProofs: true
      });
    } catch (error) {
      console.error('❌ Failed to request assets:', error);
    }
  }

  /**
   * Request allocations via STOQ protocol
   */
  async requestAllocations(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('allocations_request', {
        timestamp: new Date().toISOString(),
        requestId: `allocations-${Date.now()}`,
        includeCompleted: false
      });
    } catch (error) {
      console.error('❌ Failed to request allocations:', error);
    }
  }

  /**
   * Request Byzantine detections via STOQ protocol
   */
  async requestByzantineDetections(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('byzantine_detections_request', {
        timestamp: new Date().toISOString(),
        requestId: `byzantine-${Date.now()}`,
        includeResolved: true,
        maxResults: 100
      });
    } catch (error) {
      console.error('❌ Failed to request Byzantine detections:', error);
    }
  }

  /**
   * Request QUIC connections via STOQ protocol
   */
  async requestQUICConnections(): Promise<void> {
    if (!this.stoqClient || !this.stoqClient.isConnected()) {
      return;
    }

    try {
      await this.stoqClient.sendMessage('quic_connections_request', {
        timestamp: new Date().toISOString(),
        requestId: `quic-${Date.now()}`,
        includeInactive: false
      });
    } catch (error) {
      console.error('❌ Failed to request QUIC connections:', error);
    }
  }

  /**
   * Get current system status
   */
  getCurrentSystemStatus(): SystemStatus | null {
    return this.systemStatusSubject.value;
  }

  /**
   * Get current performance metrics
   */
  getCurrentPerformanceMetrics(): PerformanceMetrics | null {
    return this.performanceMetricsSubject.value;
  }

  /**
   * Get current assets
   */
  getCurrentAssets(): Asset[] {
    return this.assetsSubject.value;
  }

  /**
   * Get current allocations
   */
  getCurrentAllocations(): AssetAllocation[] {
    return this.allocationsSubject.value;
  }

  /**
   * Get current Byzantine detections
   */
  getCurrentByzantineDetections(): ByzantineDetection[] {
    return this.byzantineDetectionsSubject.value;
  }

  /**
   * Get current QUIC connections
   */
  getCurrentQUICConnections(): QUICConnection[] {
    return this.quicConnectionsSubject.value;
  }

  /**
   * Check if data provider is connected and streaming
   */
  isConnected(): boolean {
    return this.stoqClient ? this.stoqClient.isConnected() : false;
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
    this.isInitialized = false;
    console.log('🔌 STOQ Data Provider disconnected');
  }
}

// Singleton instance for global use
export const stoqDataProvider = new StoqDataProvider();
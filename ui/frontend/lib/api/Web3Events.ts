// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Web3 Events - Real-time WebSocket system for live metrics and status updates
 * 
 * Provides real-time event streaming from Web3 ecosystem services with:
 * - Certificate-authenticated WebSocket connections
 * - Event subscription management
 * - Automatic reconnection with exponential backoff
 * - Type-safe event handling
 * - Performance monitoring (<500ms latency requirement)
 */

import { Web3APIClient, ServiceType } from './Web3APIClient';

export type EventChannel =
  | 'system.status'
  | 'system.health'
  | 'trustchain.certificates'
  | 'trustchain.rotation'
  | 'trustchain.dns'
  | 'hypermesh.assets'
  | 'hypermesh.stateProof'
  | 'hypermesh.byzantine'
  | 'hypermesh.vm'
  | 'stoq.connections'
  | 'stoq.performance'
  | 'stoq.metrics'
  | 'integration.events';

export interface EventSubscription {
  channel: EventChannel;
  service: ServiceType;
  callback: (data: any) => void;
  active: boolean;
}

export interface WebSocketEvent {
  id: string;
  channel: EventChannel;
  service: ServiceType;
  type: string;
  data: any;
  timestamp: string;
  latency?: number;
}

export interface ConnectionStatus {
  service: ServiceType;
  connected: boolean;
  authenticated: boolean;
  lastPing?: Date;
  latency?: number;
  reconnectAttempts: number;
}

export type EventCallback = (event: WebSocketEvent) => void;
export type ConnectionCallback = (status: ConnectionStatus) => void;

export class Web3Events {
  private connections = new Map<ServiceType, WebSocket>();
  private subscriptions = new Map<string, EventSubscription>();
  private eventListeners = new Map<string, EventCallback[]>();
  private connectionListeners: ConnectionCallback[] = [];
  private reconnectTimers = new Map<ServiceType, number>();
  private connectionStatus = new Map<ServiceType, ConnectionStatus>();
  private apiClient: Web3APIClient | null = null;

  // WebSocket URLs -- not currently functional on the BlockMatrix HTTP API
  private readonly wsUrls: Record<ServiceType, string> = {
    trustchain: 'ws://localhost:9293/ws',
    stoq: 'ws://localhost:9293/ws',
    hypermesh: 'ws://localhost:9293/ws',
    integration: 'ws://localhost:9293/ws'
  };

  /**
   * Set the API client reference to avoid circular dependency
   */
  setApiClient(apiClient: Web3APIClient): void {
    this.apiClient = apiClient;
  }

  /**
   * Connect to WebSocket service with certificate authentication
   */
  async connect(service: ServiceType): Promise<void> {
    if (!this.apiClient || !this.apiClient.isAuthenticated) {
      throw new Error(`API client not authenticated for ${service}`);
    }

    const wsUrl = this.wsUrls[service];
    const certificate = this.apiClient.getCertificateInfo();

    try {
      const ws = new WebSocket(wsUrl, ['web3-protocol']);
      
      // Initialize connection status
      this.connectionStatus.set(service, {
        service,
        connected: false,
        authenticated: false,
        reconnectAttempts: 0
      });

      ws.onopen = () => {
        console.log(`WebSocket connected to ${service}`);
        
        // Send authentication message
        const authMessage = {
          type: 'authenticate',
          certificate: certificate,
          timestamp: new Date().toISOString()
        };
        
        ws.send(JSON.stringify(authMessage));
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          this.handleMessage(service, message);
        } catch (error) {
          console.error(`Failed to parse WebSocket message from ${service}:`, error);
        }
      };

      ws.onclose = (event) => {
        console.log(`WebSocket disconnected from ${service}:`, event.code, event.reason);
        this.updateConnectionStatus(service, { connected: false, authenticated: false });
        
        // Attempt reconnection unless manually closed
        if (event.code !== 1000) {
          this.scheduleReconnection(service);
        }
      };

      ws.onerror = (error) => {
        console.error(`WebSocket error for ${service}:`, error);
        this.updateConnectionStatus(service, { connected: false, authenticated: false });
      };

      this.connections.set(service, ws);

    } catch (error) {
      throw new Error(`Failed to connect to ${service}: ${error}`);
    }
  }

  /**
   * Subscribe to specific event channel
   */
  async subscribe(service: ServiceType, channel: EventChannel, callback: EventCallback): Promise<string> {
    const connection = this.connections.get(service);
    if (!connection || connection.readyState !== WebSocket.OPEN) {
      throw new Error(`No active connection to ${service}`);
    }

    const subscriptionId = `${service}:${channel}:${Date.now()}`;
    
    // Store subscription
    this.subscriptions.set(subscriptionId, {
      channel,
      service,
      callback,
      active: true
    });

    // Add event listener
    const eventKey = `${service}:${channel}`;
    if (!this.eventListeners.has(eventKey)) {
      this.eventListeners.set(eventKey, []);
    }
    this.eventListeners.get(eventKey)!.push(callback);

    // Send subscription message to server
    const subscribeMessage = {
      type: 'subscribe',
      channel,
      subscriptionId,
      timestamp: new Date().toISOString()
    };

    connection.send(JSON.stringify(subscribeMessage));

    console.log(`Subscribed to ${channel} on ${service} with ID ${subscriptionId}`);
    return subscriptionId;
  }

  /**
   * Unsubscribe from event channel
   */
  async unsubscribe(subscriptionId: string): Promise<void> {
    const subscription = this.subscriptions.get(subscriptionId);
    if (!subscription) {
      console.warn(`Subscription ${subscriptionId} not found`);
      return;
    }

    const connection = this.connections.get(subscription.service);
    if (connection && connection.readyState === WebSocket.OPEN) {
      const unsubscribeMessage = {
        type: 'unsubscribe',
        subscriptionId,
        timestamp: new Date().toISOString()
      };
      
      connection.send(JSON.stringify(unsubscribeMessage));
    }

    // Remove from local tracking
    subscription.active = false;
    this.subscriptions.delete(subscriptionId);

    // Remove event listener
    const eventKey = `${subscription.service}:${subscription.channel}`;
    const listeners = this.eventListeners.get(eventKey);
    if (listeners) {
      const index = listeners.indexOf(subscription.callback);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }

    console.log(`Unsubscribed from ${subscriptionId}`);
  }

  /**
   * Disconnect from service
   */
  disconnect(service: ServiceType): void {
    const connection = this.connections.get(service);
    if (connection) {
      connection.close(1000, 'Manual disconnect');
      this.connections.delete(service);
    }

    // Clear reconnection timer
    const timer = this.reconnectTimers.get(service);
    if (timer) {
      clearTimeout(timer);
      this.reconnectTimers.delete(service);
    }

    // Remove connection status
    this.connectionStatus.delete(service);
  }

  /**
   * Disconnect from all services
   */
  disconnectAll(): void {
    for (const service of Object.keys(this.wsUrls) as ServiceType[]) {
      this.disconnect(service);
    }
  }

  /**
   * Get connection status for service
   */
  getConnectionStatus(service: ServiceType): ConnectionStatus | null {
    return this.connectionStatus.get(service) || null;
  }

  /**
   * Get all connection statuses
   */
  getAllConnectionStatuses(): Record<ServiceType, ConnectionStatus | null> {
    const result = {} as Record<ServiceType, ConnectionStatus | null>;
    for (const service of Object.keys(this.wsUrls) as ServiceType[]) {
      result[service] = this.getConnectionStatus(service);
    }
    return result;
  }

  /**
   * Add connection status listener
   */
  onConnectionChange(callback: ConnectionCallback): () => void {
    this.connectionListeners.push(callback);
    
    // Return unsubscribe function
    return () => {
      const index = this.connectionListeners.indexOf(callback);
      if (index > -1) {
        this.connectionListeners.splice(index, 1);
      }
    };
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(service: ServiceType, message: any): void {
    const receiveTime = Date.now();

    switch (message.type) {
      case 'authenticated':
        this.updateConnectionStatus(service, {
          connected: true,
          authenticated: message.success,
          lastPing: new Date()
        });
        break;

      case 'event':
        const event: WebSocketEvent = {
          id: message.id || `${service}-${Date.now()}`,
          channel: message.channel,
          service,
          type: message.eventType || 'data',
          data: message.data,
          timestamp: message.timestamp,
          latency: message.timestamp ? receiveTime - new Date(message.timestamp).getTime() : undefined
        };

        this.emitEvent(event);
        break;

      case 'pong':
        const latency = receiveTime - (message.timestamp ? new Date(message.timestamp).getTime() : receiveTime);
        this.updateConnectionStatus(service, {
          lastPing: new Date(),
          latency
        });
        break;

      case 'error':
        console.error(`WebSocket error from ${service}:`, message.error);
        break;

      default:
        console.log(`Unknown message type from ${service}:`, message.type);
    }
  }

  /**
   * Emit event to subscribers
   */
  private emitEvent(event: WebSocketEvent): void {
    const eventKey = `${event.service}:${event.channel}`;
    const listeners = this.eventListeners.get(eventKey);
    
    if (listeners) {
      listeners.forEach(callback => {
        try {
          callback(event);
        } catch (error) {
          console.error(`Error in event callback for ${eventKey}:`, error);
        }
      });
    }
  }

  /**
   * Update connection status and notify listeners
   */
  private updateConnectionStatus(service: ServiceType, updates: Partial<ConnectionStatus>): void {
    const current = this.connectionStatus.get(service);
    const updated = { ...current, ...updates } as ConnectionStatus;
    
    this.connectionStatus.set(service, updated);
    
    // Notify listeners
    this.connectionListeners.forEach(callback => {
      try {
        callback(updated);
      } catch (error) {
        console.error('Error in connection status callback:', error);
      }
    });
  }

  /**
   * Schedule reconnection with exponential backoff
   */
  private scheduleReconnection(service: ServiceType): void {
    const status = this.connectionStatus.get(service);
    if (!status) return;

    const attempts = status.reconnectAttempts + 1;
    const delay = Math.min(1000 * Math.pow(2, attempts), 30000); // Max 30 seconds

    console.log(`Scheduling reconnection to ${service} in ${delay}ms (attempt ${attempts})`);

    const timer = setTimeout(async () => {
      try {
        this.updateConnectionStatus(service, { reconnectAttempts: attempts });
        await this.connect(service);
        
        // Resubscribe to active subscriptions
        this.resubscribeToService(service);
        
      } catch (error) {
        console.error(`Reconnection to ${service} failed:`, error);
        this.scheduleReconnection(service);
      }
    }, delay);

    this.reconnectTimers.set(service, timer);
  }

  /**
   * Resubscribe to all active subscriptions for a service
   */
  private resubscribeToService(service: ServiceType): void {
    for (const [subscriptionId, subscription] of this.subscriptions) {
      if (subscription.service === service && subscription.active) {
        this.subscribe(service, subscription.channel, subscription.callback);
      }
    }
  }

  /**
   * Send ping to maintain connection
   */
  private sendPing(service: ServiceType): void {
    const connection = this.connections.get(service);
    if (connection && connection.readyState === WebSocket.OPEN) {
      const pingMessage = {
        type: 'ping',
        timestamp: new Date().toISOString()
      };
      
      connection.send(JSON.stringify(pingMessage));
    }
  }

  /**
   * Start ping interval for all connections
   */
  startPingInterval(intervalMs: number = 30000): void {
    setInterval(() => {
      for (const service of this.connections.keys()) {
        this.sendPing(service);
      }
    }, intervalMs);
  }
}

// Note: Singleton instance created in index.ts to avoid circular dependency
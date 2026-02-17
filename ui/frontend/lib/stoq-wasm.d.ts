// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * TypeScript definitions for STOQ WebAssembly client
 * 
 * Provides type-safe access to the STOQ protocol from the browser
 * with TrustChain certificate authentication.
 */

declare module 'stoq-wasm' {
  /**
   * Connection status enumeration
   */
  export enum WasmConnectionStatus {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Authenticating = 3,
    Authenticated = 4,
    Error = 5,
  }

  /**
   * TrustChain certificate for authentication
   */
  export class WasmCertificate {
    constructor(
      pem_data: string,
      fingerprint: string,
      subject: string,
      issuer: string,
      valid_from: string,
      valid_to: string
    );
    
    readonly pem_data: string;
    readonly fingerprint: string;
    readonly subject: string;
    readonly issuer: string;
    readonly valid_from: string;
    readonly valid_to: string;
  }

  /**
   * Connection configuration
   */
  export class WasmConnectionConfig {
    constructor(server_address: string, server_port: number, certificate_pem: string);
    
    set_server_name(server_name: string | null): void;
    set_timeout_ms(timeout_ms: number): void;
  }

  /**
   * STOQ protocol message
   */
  export class WasmStoqMessage {
    constructor(message_type: string, payload: string);
    
    readonly message_type: string;
    readonly payload: string;
    readonly correlation_id: string | null;
    readonly timestamp: string;
    
    set_correlation_id(correlation_id: string | null): void;
  }

  /**
   * Main STOQ WebAssembly client
   */
  export class WasmStoqClient {
    constructor(config: WasmConnectionConfig);

    /**
     * Initialize TrustChain certificates
     */
    initialize_certificates(): Promise<boolean>;

    /**
     * Connect to STOQ server
     */
    connect(): Promise<void>;

    /**
     * Disconnect from STOQ server
     */
    disconnect(): Promise<void>;

    /**
     * Send message through STOQ protocol
     */
    send_message(message: WasmStoqMessage): Promise<void>;

    /**
     * Register message handler for specific message type
     */
    register_message_handler(message_type: string, handler: (message: any) => void): Promise<void>;

    /**
     * Register event callback (status changes, errors, etc.)
     */
    register_event_callback(event_type: string, callback: (event: any) => void): Promise<void>;

    /**
     * Get current connection status
     */
    get_status(): WasmConnectionStatus;

    /**
     * Get connection ID if connected
     */
    get_connection_id(): string | null;

    /**
     * Send dashboard request message
     */
    request_dashboard_data(dashboard_type: string): Promise<void>;

    /**
     * Send system status request
     */
    request_system_status(): Promise<void>;

    /**
     * Send performance metrics request
     */
    request_performance_metrics(time_range: string): Promise<void>;
  }

  /**
   * Helper functions
   */
  export function create_connection_config(
    server_address: string,
    server_port: number,
    certificate_pem: string
  ): WasmConnectionConfig;

  export function create_stoq_message(message_type: string, payload: string): WasmStoqMessage;

  export function get_version(): string;

  export function log_message(message: string): void;

  /**
   * Initialize the WASM module
   */
  export default function init(module?: WebAssembly.Module | Promise<WebAssembly.Module>): Promise<void>;
}

/**
 * JavaScript event types for STOQ client
 */
export interface StoqStatusChangeEvent {
  status: string;
  connectionId: string;
  timestamp: string;
}

export interface StoqMessageEvent {
  messageType: string;
  payload: string;
  timestamp: string;
}

export interface StoqErrorEvent {
  error: string;
  details?: string;
  timestamp: string;
}

/**
 * Dashboard data structures returned by STOQ protocol
 */
export interface StoqDashboardResponse {
  status: 'success' | 'error';
  data?: {
    components: {
      [key: string]: {
        status: string;
        [key: string]: any;
      };
    };
    timestamp: string;
  };
  error?: string;
}

export interface StoqSystemStatusResponse {
  status: 'success' | 'error';
  system?: {
    overall_health: string;
    score: number;
    services: {
      [key: string]: {
        status: string;
        [key: string]: any;
      };
    };
  };
  error?: string;
}

export interface StoqPerformanceMetricsResponse {
  status: 'success' | 'error';
  metrics?: {
    throughput: {
      current: number;
      target: number;
      unit: string;
      efficiency: number;
    };
    latency: {
      average: number;
      p95: number;
      p99: number;
      unit: string;
    };
    connections: {
      active: number;
      total: number;
      failed: number;
    };
  };
  error?: string;
}
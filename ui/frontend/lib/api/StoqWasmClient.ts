// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * STOQ WebAssembly Client Integration
 * 
 * Provides a TypeScript wrapper around the STOQ WASM client that integrates
 * with the existing Web3 API architecture. Handles TrustChain certificate
 * authentication and STOQ protocol communication.
 */

// Types aligned with actual WASM interface
export interface WasmConnectionStatus {
  is_connected: boolean;
  is_authenticated: boolean;
  connection_id: string;
  error_message: string;
  protocol_version: string;
}

export interface WasmStoqClient {
  connect(certificate: WasmCertificate): Promise<void>;
  send_message(message_js: any): Promise<any>;
  disconnect(): Promise<void>;
  readonly status: WasmConnectionStatus;
  free(): void;
}

export interface WasmCertificate {
  validate(): boolean;
  readonly fingerprint: string;
  free(): void;
}

export interface WasmConnectionConfig {
  readonly server_address: string;
  readonly server_port: number;
  readonly use_ipv6: boolean;
  free(): void;
}

export interface StoqDashboardResponse {
  status: 'success' | 'error';
  data?: any;
  error?: string;
}

export interface StoqSystemStatusResponse {
  status: 'success' | 'error';
  system?: any;
  error?: string;
}

export interface StoqPerformanceMetricsResponse {
  status: 'success' | 'error';
  metrics?: any;
  error?: string;
}

/**
 * STOQ WASM client configuration
 */
export interface StoqWasmConfig {
  serverAddress: string;
  serverPort: number;
  certificatePem: string;
  serverName?: string;
  timeoutMs?: number;
  autoReconnect?: boolean;
  reconnectIntervalMs?: number;
  maxReconnectAttempts?: number;
}

/**
 * Connection event callbacks
 */
export interface StoqEventCallbacks {
  onStatusChange?: (status: WasmConnectionStatus, connectionId?: string) => void;
  onMessage?: (messageType: string, payload: any) => void;
  onError?: (error: string, details?: string) => void;
  onConnect?: (connectionId: string) => void;
  onDisconnect?: (reason?: string) => void;
}

/**
 * TypeScript wrapper for STOQ WebAssembly client
 */
export class StoqWasmClient {
  private wasmClient: WasmStoqClient | null = null;
  private wasmModule: any = null;
  private config: StoqWasmConfig;
  private callbacks: StoqEventCallbacks;
  private reconnectTimer: number | null = null;
  private reconnectAttempts = 0;
  private isInitialized = false;
  private messageHandlers = new Map<string, (payload: any) => void>();

  constructor(config: StoqWasmConfig, callbacks: StoqEventCallbacks = {}) {
    this.config = {
      autoReconnect: true,
      reconnectIntervalMs: 5000,
      maxReconnectAttempts: 10,
      timeoutMs: 30000,
      ...config,
    };
    this.callbacks = callbacks;
  }

  /**
   * Initialize the WASM client with certificate authentication
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    try {
      console.log('🚀 Initializing STOQ WASM client...');
      
      // Load WASM module dynamically using script tag approach
      const wasmModule = await this.loadWasmModule();
      
      // Create WASM client instance
      this.wasmClient = new wasmModule.WasmStoqClient(
        this.config.serverAddress,
        this.config.serverPort,
        true // use IPv6
      );
      
      this.wasmModule = wasmModule;
      this.isInitialized = true;
      
      console.log('✅ STOQ WASM client initialized successfully');
      
      // Set up event handlers
      await this.setupEventHandlers();
      await this.setupDefaultMessageHandlers();
    } catch (error) {
      console.error('❌ Failed to initialize STOQ WASM client:', error);
      this.isInitialized = false;
      throw error;
    }
  }

  /**
   * Load WASM module using ES6 dynamic import (Vite-compatible)
   */
  private async loadWasmModule(): Promise<any> {
    try {
      // Check if module is already loaded
      if ((window as any).stoq_wasm) {
        console.log('WASM module already loaded');
        return (window as any).stoq_wasm;
      }

      console.log('Loading WASM module via ES6 dynamic import...');
      
      // Use dynamic import with Vite's URL query parameter for static assets
      const wasmModule = await import('/wasm/stoq_wasm.js?url').then(async (module) => {
        // The module.default is the URL, we need to fetch and eval it
        const wasmUrl = module.default;
        const response = await fetch(wasmUrl);
        const wasmCode = await response.text();
        
        // Create a module script element
        const moduleScript = document.createElement('script');
        moduleScript.type = 'module';
        moduleScript.text = `
          ${wasmCode}
          
          // Initialize and expose WASM module
          window.stoq_wasm_init = async function() {
            if (!window.stoq_wasm) {
              await wasm_bindgen('/wasm/stoq_wasm_bg.wasm');
              window.stoq_wasm = wasm_bindgen;
            }
            return window.stoq_wasm;
          };
        `;
        
        document.head.appendChild(moduleScript);
        
        // Wait for module to load and initialize
        return new Promise((resolve, reject) => {
          setTimeout(async () => {
            try {
              if ((window as any).stoq_wasm_init) {
                const module = await (window as any).stoq_wasm_init();
                resolve(module);
              } else {
                reject(new Error('stoq_wasm_init function not available'));
              }
            } catch (error) {
              reject(error);
            }
          }, 100); // Small delay for script execution
        });
      });
      
      console.log('✅ WASM module loaded and initialized via ES6 import');
      return wasmModule;
      
    } catch (error) {
      console.error('❌ WASM ES6 loading failed, trying fallback approach...', error);
      
      // Fallback: Try direct file fetch approach
      try {
        console.log('Attempting fallback WASM loading...');
        
        const response = await fetch('/wasm/stoq_wasm.js');
        const wasmCode = await response.text();
        
        // Execute WASM code in module context
        const blob = new Blob([wasmCode], { type: 'application/javascript' });
        const moduleUrl = URL.createObjectURL(blob);
        
        const wasmModule = await import(moduleUrl);
        await wasmModule.default('/wasm/stoq_wasm_bg.wasm');
        
        // Store module reference  
        (window as any).stoq_wasm = wasmModule;
        
        URL.revokeObjectURL(moduleUrl);
        
        console.log('✅ WASM module loaded via fallback approach');
        return wasmModule;
        
      } catch (fallbackError) {
        console.error('❌ Fallback WASM loading also failed:', fallbackError);
        throw new Error(`WASM loading failed: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
  }

  /**
   * Connect to the STOQ server
   */
  async connect(): Promise<void> {
    if (!this.isInitialized) {
      throw new Error('STOQ client not initialized');
    }

    if (!this.wasmClient || !this.wasmModule) {
      throw new Error('WASM client not properly initialized');
    }

    try {
      console.log('🔗 Connecting to STOQ server:', this.config.serverAddress);
      
      // Create certificate from PEM data
      const certificate = new this.wasmModule.WasmCertificate(this.config.certificatePem);
      
      // Validate certificate
      if (!certificate.validate()) {
        throw new Error('Invalid TrustChain certificate');
      }
      
      console.log('✅ Certificate validated, fingerprint:', certificate.fingerprint);
      
      // Connect using WASM client
      await this.wasmClient.connect(certificate);
      
      this.reconnectAttempts = 0;
      
      // Get connection status
      const status = this.wasmClient.status;
      if (status.is_connected && status.is_authenticated) {
        console.log('✅ STOQ connection established:', status.connection_id);
        
        if (this.callbacks.onConnect) {
          this.callbacks.onConnect(status.connection_id);
        }
        
        if (this.callbacks.onStatusChange) {
          this.callbacks.onStatusChange(status as any, status.connection_id);
        }
      } else {
        throw new Error(status.error_message || 'Connection failed');
      }
      
      // Clean up certificate
      certificate.free();
    } catch (error) {
      console.error('❌ Failed to connect to STOQ server:', error);
      
      if (this.callbacks.onError) {
        this.callbacks.onError('Connection failed', error instanceof Error ? error.message : String(error));
      }

      // Attempt reconnection if enabled
      if (this.config.autoReconnect && this.reconnectAttempts < (this.config.maxReconnectAttempts || 10)) {
        this.scheduleReconnect();
      }

      throw error;
    }
  }

  /**
   * Disconnect from the STOQ server
   */
  async disconnect(): Promise<void> {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.wasmClient) {
      try {
        console.log('🔌 Disconnecting from STOQ server...');
        await this.wasmClient.disconnect();
        console.log('✅ STOQ disconnected successfully');
      } catch (error) {
        console.error('❌ Error during disconnect:', error);
      }
    }
    
    if (this.callbacks.onDisconnect) {
      this.callbacks.onDisconnect('Manual disconnect');
    }
  }

  /**
   * Get current connection status
   */
  getStatus(): WasmConnectionStatus | null {
    if (!this.wasmClient) {
      return null;
    }
    
    try {
      return this.wasmClient.status;
    } catch (error) {
      console.error('Error getting STOQ status:', error);
      return null;
    }
  }

  /**
   * Get connection ID
   */
  getConnectionId(): string | null {
    const status = this.getStatus();
    return status?.connection_id || null;
  }

  /**
   * Check if client is connected and authenticated
   */
  isConnected(): boolean {
    const status = this.getStatus();
    return status ? status.is_connected && status.is_authenticated : false;
  }

  /**
   * Register a message handler for a specific message type
   */
  registerMessageHandler(messageType: string, handler: (payload: any) => void): void {
    this.messageHandlers.set(messageType, handler);
    
    // TODO: Register with actual WASM client when available
    console.log(`Registered placeholder handler for message type: ${messageType}`);
  }

  /**
   * Send a message through the STOQ protocol
   */
  async sendMessage(messageType: string, payload: any): Promise<void> {
    if (!this.isConnected()) {
      throw new Error('Not connected to STOQ server');
    }

    if (!this.wasmModule || !this.wasmClient) {
      throw new Error('WASM client not properly initialized');
    }

    try {
      console.log('📤 Sending STOQ message:', messageType);
      
      // Create correlation ID for this message
      const correlationId = `msg-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      
      // Create STOQ message using WASM function
      const message = this.wasmModule.create_stoq_message(
        messageType, 
        JSON.stringify(payload),
        correlationId
      );
      
      // Send message via WASM client
      const response = await this.wasmClient.send_message(message);
      
      console.log('✅ STOQ message sent successfully:', messageType);
      
      // Handle response if any
      if (response) {
        console.log('📨 Received response:', response);
        
        // Trigger message callback if registered
        const handler = this.messageHandlers.get(messageType + '_response');
        if (handler) {
          handler(response);
        }
      }
    } catch (error) {
      console.error('❌ Failed to send STOQ message:', error);
      
      if (this.callbacks.onError) {
        this.callbacks.onError('Message send failed', error instanceof Error ? error.message : String(error));
      }
      
      throw error;
    }
  }

  /**
   * Request dashboard data
   */
  async requestDashboardData(dashboardType: string): Promise<void> {
    await this.sendMessage('dashboard_request', {
      dashboard_type: dashboardType,
      timestamp: new Date().toISOString()
    });
  }

  /**
   * Request system status
   */
  async requestSystemStatus(): Promise<void> {
    await this.sendMessage('system_status_request', {
      timestamp: new Date().toISOString()
    });
  }

  /**
   * Request performance metrics
   */
  async requestPerformanceMetrics(timeRange = '1h'): Promise<void> {
    await this.sendMessage('performance_metrics_request', {
      time_range: timeRange,
      timestamp: new Date().toISOString()
    });
  }

  /**
   * Set up event handlers for WASM client
   */
  private async setupEventHandlers(): Promise<void> {
    // TODO: Set up actual WASM event handlers
    console.log('Placeholder event handlers setup');
  }

  /**
   * Set up default message handlers for dashboard integration
   */
  private async setupDefaultMessageHandlers(): Promise<void> {
    // TODO: Set up actual WASM message handlers
    console.log('Placeholder message handlers setup');
  }

  /**
   * Parse connection status from string to status object
   */
  private parseConnectionStatus(status: string): WasmConnectionStatus {
    const statusMap: Record<string, WasmConnectionStatus> = {
      'Disconnected': { is_connected: false, is_authenticated: false, connection_id: '', error_message: '', protocol_version: '' },
      'Connecting': { is_connected: false, is_authenticated: false, connection_id: '', error_message: '', protocol_version: '' },
      'Connected': { is_connected: true, is_authenticated: false, connection_id: '', error_message: '', protocol_version: '' },
      'Authenticating': { is_connected: true, is_authenticated: false, connection_id: '', error_message: '', protocol_version: '' },
      'Authenticated': { is_connected: true, is_authenticated: true, connection_id: '', error_message: '', protocol_version: '' },
      'Error': { is_connected: false, is_authenticated: false, connection_id: '', error_message: 'Connection error', protocol_version: '' },
    };
    return statusMap[status] || statusMap['Disconnected'];
  }

  /**
   * Schedule reconnection attempt
   */
  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }

    this.reconnectAttempts++;
    const delay = Math.min(
      (this.config.reconnectIntervalMs || 5000) * Math.pow(2, this.reconnectAttempts - 1),
      30000 // Max 30 seconds
    );

    console.log(`Scheduling reconnection attempt ${this.reconnectAttempts} in ${delay}ms`);

    this.reconnectTimer = window.setTimeout(async () => {
      try {
        await this.connect();
      } catch (error) {
        console.error('Reconnection attempt failed:', error);
      }
    }, delay);
  }


  /**
   * Clean up resources
   */
  destroy(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Clean up WASM resources
    if (this.wasmClient) {
      try {
        this.wasmClient.free();
      } catch (error) {
        console.error('Error freeing WASM client:', error);
      }
    }

    this.messageHandlers.clear();
    this.wasmClient = null;
    this.wasmModule = null;
    this.isInitialized = false;
    
    console.log('🧹 STOQ WASM client destroyed and resources cleaned up');
  }
}

/**
 * Factory function to create a configured STOQ WASM client
 */
export function createStoqWasmClient(config: StoqWasmConfig, callbacks?: StoqEventCallbacks): StoqWasmClient {
  return new StoqWasmClient(config, callbacks);
}

/**
 * Utility function to validate TrustChain certificate
 */
export function validateCertificate(certificatePem: string): { valid: boolean; error?: string } {
  try {
    // Basic PEM format validation
    if (!certificatePem.includes('-----BEGIN CERTIFICATE-----') || 
        !certificatePem.includes('-----END CERTIFICATE-----')) {
      return { valid: false, error: 'Invalid PEM certificate format' };
    }

    // Additional validation could be added here
    return { valid: true };
  } catch (error) {
    return { valid: false, error: error instanceof Error ? error.message : 'Unknown validation error' };
  }
}
// API Client for Web3 Ecosystem Services

import type { 
	SystemStatus, 
	Certificate, 
	Asset, 
	STOQConnection, 
	STOQMetrics,
	ConsensusMetrics,
	EconomicMetrics,
	ApiResponse 
} from '$lib/types.js';

// API Configuration
const API_BASE_URLS = {
	trustchain: 'https://[2001:db8::1]:8443', // IPv6 TrustChain CA
	stoq: 'https://[2001:db8::2]:8444',      // STOQ Protocol Gateway
	hypermesh: 'https://[2001:db8::3]:8445', // HyperMesh Asset Manager
	caesar: 'https://[2001:db8::4]:8446',    // Caesar Economics
	consensus: 'https://[2001:db8::5]:8447'  // Consensus Monitor
};

// WebSocket endpoints for real-time updates
const WS_BASE_URLS = {
	trustchain: 'wss://[2001:db8::1]:8443/ws',
	stoq: 'wss://[2001:db8::2]:8444/ws',
	hypermesh: 'wss://[2001:db8::3]:8445/ws',
	caesar: 'wss://[2001:db8::4]:8446/ws',
	consensus: 'wss://[2001:db8::5]:8447/ws'
};

class ApiClient {
	private async request<T>(
		service: keyof typeof API_BASE_URLS, 
		endpoint: string, 
		options: RequestInit = {}
	): Promise<ApiResponse<T>> {
		const url = `${API_BASE_URLS[service]}${endpoint}`;
		
		try {
			const response = await fetch(url, {
				...options,
				headers: {
					'Content-Type': 'application/json',
					'Accept': 'application/json',
					...options.headers
				}
			});

			const data = await response.json();
			
			if (!response.ok) {
				throw new Error(data.error || `HTTP ${response.status}`);
			}

			return {
				success: true,
				data,
				timestamp: new Date().toISOString()
			};
		} catch (error) {
			console.error(`API Error (${service}${endpoint}):`, error);
			return {
				success: false,
				error: error instanceof Error ? error.message : 'Unknown error',
				timestamp: new Date().toISOString()
			};
		}
	}

	// TrustChain CA API
	async getTrustChainStatus(): Promise<ApiResponse<SystemStatus>> {
		return this.request('trustchain', '/api/v1/status');
	}

	async getCertificates(): Promise<ApiResponse<Certificate[]>> {
		return this.request('trustchain', '/api/v1/certificates');
	}

	async issueCertificate(request: any): Promise<ApiResponse<Certificate>> {
		return this.request('trustchain', '/api/v1/certificates', {
			method: 'POST',
			body: JSON.stringify(request)
		});
	}

	async revokeCertificate(serialNumber: string): Promise<ApiResponse<void>> {
		return this.request('trustchain', `/api/v1/certificates/${serialNumber}/revoke`, {
			method: 'POST'
		});
	}

	// STOQ Protocol API
	async getSTOQMetrics(): Promise<ApiResponse<STOQMetrics>> {
		return this.request('stoq', '/api/v1/metrics');
	}

	async getSTOQConnections(): Promise<ApiResponse<STOQConnection[]>> {
		return this.request('stoq', '/api/v1/connections');
	}

	async optimizeSTOQPerformance(): Promise<ApiResponse<void>> {
		return this.request('stoq', '/api/v1/optimize', {
			method: 'POST'
		});
	}

	// HyperMesh Asset API
	async getAssets(): Promise<ApiResponse<Asset[]>> {
		return this.request('hypermesh', '/api/v1/assets');
	}

	async getAsset(id: string): Promise<ApiResponse<Asset>> {
		return this.request('hypermesh', `/api/v1/assets/${id}`);
	}

	async createAsset(asset: Partial<Asset>): Promise<ApiResponse<Asset>> {
		return this.request('hypermesh', '/api/v1/assets', {
			method: 'POST',
			body: JSON.stringify(asset)
		});
	}

	async updateAsset(id: string, updates: Partial<Asset>): Promise<ApiResponse<Asset>> {
		return this.request('hypermesh', `/api/v1/assets/${id}`, {
			method: 'PATCH',
			body: JSON.stringify(updates)
		});
	}

	async deleteAsset(id: string): Promise<ApiResponse<void>> {
		return this.request('hypermesh', `/api/v1/assets/${id}`, {
			method: 'DELETE'
		});
	}

	// Caesar Economics API
	async getEconomicMetrics(): Promise<ApiResponse<EconomicMetrics>> {
		return this.request('caesar', '/api/v1/metrics');
	}

	async getUserBalance(): Promise<ApiResponse<any>> {
		return this.request('caesar', '/api/v1/wallet/balance');
	}

	async getRewards(): Promise<ApiResponse<any[]>> {
		return this.request('caesar', '/api/v1/rewards');
	}

	async claimRewards(): Promise<ApiResponse<void>> {
		return this.request('caesar', '/api/v1/rewards/claim', {
			method: 'POST'
		});
	}

	// Consensus API
	async getConsensusMetrics(): Promise<ApiResponse<ConsensusMetrics>> {
		return this.request('consensus', '/api/v1/metrics');
	}

	async getRecentBlocks(): Promise<ApiResponse<any[]>> {
		return this.request('consensus', '/api/v1/blocks');
	}

	async validateConsensus(): Promise<ApiResponse<void>> {
		return this.request('consensus', '/api/v1/validate', {
			method: 'POST'
		});
	}

	// System-wide API
	async getSystemStatus(): Promise<ApiResponse<SystemStatus[]>> {
		const services = Object.keys(API_BASE_URLS) as (keyof typeof API_BASE_URLS)[];
		const statusPromises = services.map(async (service) => {
			const response = await this.request(service, '/api/v1/status');
			return response.data;
		});

		try {
			const statuses = await Promise.all(statusPromises);
			return {
				success: true,
				data: statuses.filter(Boolean) as SystemStatus[],
				timestamp: new Date().toISOString()
			};
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : 'Failed to get system status',
				timestamp: new Date().toISOString()
			};
		}
	}
}

// WebSocket Manager for real-time updates
export class WebSocketManager {
	private connections: Map<string, WebSocket> = new Map();
	private reconnectTimeouts: Map<string, NodeJS.Timeout> = new Map();
	private eventCallbacks: Map<string, Set<(data: any) => void>> = new Map();

	connect(service: keyof typeof WS_BASE_URLS, onMessage?: (data: any) => void): void {
		const url = WS_BASE_URLS[service];
		
		if (this.connections.has(service)) {
			this.disconnect(service);
		}

		try {
			const ws = new WebSocket(url);
			
			ws.onopen = () => {
				console.log(`WebSocket connected to ${service}`);
				this.connections.set(service, ws);
				
				// Clear any reconnect timeout
				const timeout = this.reconnectTimeouts.get(service);
				if (timeout) {
					clearTimeout(timeout);
					this.reconnectTimeouts.delete(service);
				}
			};
			
			ws.onmessage = (event) => {
				try {
					const data = JSON.parse(event.data);
					
					// Trigger specific callback
					if (onMessage) {
						onMessage(data);
					}
					
					// Trigger all registered callbacks for this service
					const callbacks = this.eventCallbacks.get(service);
					if (callbacks) {
						callbacks.forEach(callback => callback(data));
					}
				} catch (error) {
					console.error(`Failed to parse WebSocket message from ${service}:`, error);
				}
			};
			
			ws.onclose = () => {
				console.log(`WebSocket disconnected from ${service}`);
				this.connections.delete(service);
				
				// Auto-reconnect after 5 seconds
				const timeout = setTimeout(() => {
					this.connect(service, onMessage);
				}, 5000);
				this.reconnectTimeouts.set(service, timeout);
			};
			
			ws.onerror = (error) => {
				console.error(`WebSocket error for ${service}:`, error);
			};
			
		} catch (error) {
			console.error(`Failed to connect WebSocket to ${service}:`, error);
		}
	}

	disconnect(service: keyof typeof WS_BASE_URLS): void {
		const ws = this.connections.get(service);
		if (ws) {
			ws.close();
			this.connections.delete(service);
		}
		
		const timeout = this.reconnectTimeouts.get(service);
		if (timeout) {
			clearTimeout(timeout);
			this.reconnectTimeouts.delete(service);
		}
	}

	subscribe(service: keyof typeof WS_BASE_URLS, callback: (data: any) => void): () => void {
		let callbacks = this.eventCallbacks.get(service);
		if (!callbacks) {
			callbacks = new Set();
			this.eventCallbacks.set(service, callbacks);
		}
		
		callbacks.add(callback);
		
		// Return unsubscribe function
		return () => {
			callbacks?.delete(callback);
		};
	}

	send(service: keyof typeof WS_BASE_URLS, data: any): void {
		const ws = this.connections.get(service);
		if (ws && ws.readyState === WebSocket.OPEN) {
			ws.send(JSON.stringify(data));
		} else {
			console.warn(`WebSocket for ${service} is not connected`);
		}
	}

	disconnectAll(): void {
		Object.keys(WS_BASE_URLS).forEach(service => {
			this.disconnect(service as keyof typeof WS_BASE_URLS);
		});
		this.eventCallbacks.clear();
	}
}

// Export singleton instances
export const apiClient = new ApiClient();
export const wsManager = new WebSocketManager();

// Health check utility
export async function checkSystemHealth(): Promise<{[key: string]: boolean}> {
	const services = Object.keys(API_BASE_URLS) as (keyof typeof API_BASE_URLS)[];
	const healthChecks: {[key: string]: boolean} = {};
	
	await Promise.all(
		services.map(async (service) => {
			try {
				const response = await fetch(`${API_BASE_URLS[service]}/health`, {
					method: 'GET',
					signal: AbortSignal.timeout(5000) // 5 second timeout
				});
				healthChecks[service] = response.ok;
			} catch (error) {
				healthChecks[service] = false;
			}
		})
	);
	
	return healthChecks;
}
// Global State Management for Web3 Ecosystem UI

import { writable, derived, get } from 'svelte/store';
import type { 
	SystemStatus, 
	Asset, 
	Certificate, 
	STOQMetrics,
	ConsensusMetrics,
	EconomicMetrics,
	NotificationState,
	DashboardState
} from '$lib/types.js';
import { apiClient, wsManager } from '$lib/api/index.js';

// Dashboard state
export const dashboardState = writable<DashboardState>({
	selectedSystem: null,
	timeRange: '24h',
	autoRefresh: true,
	darkMode: false
});

// System statuses
export const systemStatuses = writable<SystemStatus[]>([]);

// Assets
export const assets = writable<Asset[]>([]);
export const selectedAsset = writable<Asset | null>(null);

// Certificates
export const certificates = writable<Certificate[]>([]);

// STOQ metrics
export const stoqMetrics = writable<STOQMetrics | null>(null);

// Consensus metrics
export const consensusMetrics = writable<ConsensusMetrics | null>(null);

// Economic metrics
export const economicMetrics = writable<EconomicMetrics | null>(null);

// Notifications
export const notifications = writable<NotificationState[]>([]);

// Connection status
export const connectionStatus = writable<{[key: string]: boolean}>({
	trustchain: false,
	stoq: false,
	hypermesh: false,
	caesar: false,
	consensus: false
});

// Loading states
export const loading = writable<{[key: string]: boolean}>({
	systemStatus: false,
	assets: false,
	certificates: false,
	stoqMetrics: false,
	consensusMetrics: false,
	economicMetrics: false
});

// Error states
export const errors = writable<{[key: string]: string | null}>({
	systemStatus: null,
	assets: null,
	certificates: null,
	stoqMetrics: null,
	consensusMetrics: null,
	economicMetrics: null
});

// Derived stores
export const unreadNotifications = derived(
	notifications,
	($notifications) => $notifications.filter(n => !n.read).length
);

export const systemHealth = derived(
	systemStatuses,
	($statuses) => {
		if ($statuses.length === 0) return 'unknown';
		
		const healthyCount = $statuses.filter(s => s.status === 'online').length;
		const totalCount = $statuses.length;
		const healthPercentage = (healthyCount / totalCount) * 100;
		
		if (healthPercentage === 100) return 'healthy';
		if (healthPercentage >= 80) return 'degraded';
		return 'critical';
	}
);

export const activeAssetsCount = derived(
	assets,
	($assets) => $assets.filter(a => a.status.health === 'healthy').length
);

export const totalStakeAmount = derived(
	assets,
	($assets) => $assets
		.filter(a => a.privacy.requireConsensus)
		.reduce((sum, a) => sum + a.allocation.allocated, 0)
);

// Action creators
export const dashboardActions = {
	setSelectedSystem: (system: string | null) => {
		dashboardState.update(state => ({ ...state, selectedSystem: system }));
	},

	setTimeRange: (range: '1h' | '24h' | '7d' | '30d') => {
		dashboardState.update(state => ({ ...state, timeRange: range }));
	},

	toggleAutoRefresh: () => {
		dashboardState.update(state => ({ ...state, autoRefresh: !state.autoRefresh }));
	},

	toggleDarkMode: () => {
		dashboardState.update(state => {
			const newDarkMode = !state.darkMode;
			
			// Apply theme to document
			if (newDarkMode) {
				document.documentElement.classList.add('dark');
				localStorage.setItem('theme', 'dark');
			} else {
				document.documentElement.classList.remove('dark');
				localStorage.setItem('theme', 'light');
			}
			
			return { ...state, darkMode: newDarkMode };
		});
	}
};

export const notificationActions = {
	add: (notification: Omit<NotificationState, 'id' | 'timestamp' | 'read'>) => {
		const newNotification: NotificationState = {
			...notification,
			id: Date.now().toString(),
			timestamp: new Date().toISOString(),
			read: false
		};
		
		notifications.update(items => [newNotification, ...items]);
		
		// Auto-remove after 10 seconds for non-error notifications
		if (notification.type !== 'error') {
			setTimeout(() => {
				notificationActions.remove(newNotification.id);
			}, 10000);
		}
	},

	remove: (id: string) => {
		notifications.update(items => items.filter(item => item.id !== id));
	},

	markAsRead: (id: string) => {
		notifications.update(items =>
			items.map(item =>
				item.id === id ? { ...item, read: true } : item
			)
		);
	},

	markAllAsRead: () => {
		notifications.update(items =>
			items.map(item => ({ ...item, read: true }))
		);
	},

	clear: () => {
		notifications.set([]);
	}
};

export const dataActions = {
	async loadSystemStatuses() {
		loading.update(state => ({ ...state, systemStatus: true }));
		errors.update(state => ({ ...state, systemStatus: null }));
		
		try {
			const response = await apiClient.getSystemStatus();
			if (response.success && response.data) {
				systemStatuses.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load system status');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, systemStatus: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'System Status Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, systemStatus: false }));
		}
	},

	async loadAssets() {
		loading.update(state => ({ ...state, assets: true }));
		errors.update(state => ({ ...state, assets: null }));
		
		try {
			const response = await apiClient.getAssets();
			if (response.success && response.data) {
				assets.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load assets');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, assets: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'Assets Loading Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, assets: false }));
		}
	},

	async loadCertificates() {
		loading.update(state => ({ ...state, certificates: true }));
		errors.update(state => ({ ...state, certificates: null }));
		
		try {
			const response = await apiClient.getCertificates();
			if (response.success && response.data) {
				certificates.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load certificates');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, certificates: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'Certificates Loading Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, certificates: false }));
		}
	},

	async loadSTOQMetrics() {
		loading.update(state => ({ ...state, stoqMetrics: true }));
		errors.update(state => ({ ...state, stoqMetrics: null }));
		
		try {
			const response = await apiClient.getSTOQMetrics();
			if (response.success && response.data) {
				stoqMetrics.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load STOQ metrics');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, stoqMetrics: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'STOQ Metrics Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, stoqMetrics: false }));
		}
	},

	async loadConsensusMetrics() {
		loading.update(state => ({ ...state, consensusMetrics: true }));
		errors.update(state => ({ ...state, consensusMetrics: null }));
		
		try {
			const response = await apiClient.getConsensusMetrics();
			if (response.success && response.data) {
				consensusMetrics.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load consensus metrics');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, consensusMetrics: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'Consensus Metrics Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, consensusMetrics: false }));
		}
	},

	async loadEconomicMetrics() {
		loading.update(state => ({ ...state, economicMetrics: true }));
		errors.update(state => ({ ...state, economicMetrics: null }));
		
		try {
			const response = await apiClient.getEconomicMetrics();
			if (response.success && response.data) {
				economicMetrics.set(response.data);
			} else {
				throw new Error(response.error || 'Failed to load economic metrics');
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			errors.update(state => ({ ...state, economicMetrics: errorMessage }));
			notificationActions.add({
				type: 'error',
				title: 'Economic Metrics Error',
				message: errorMessage
			});
		} finally {
			loading.update(state => ({ ...state, economicMetrics: false }));
		}
	},

	async loadAllData() {
		await Promise.allSettled([
			dataActions.loadSystemStatuses(),
			dataActions.loadAssets(),
			dataActions.loadCertificates(),
			dataActions.loadSTOQMetrics(),
			dataActions.loadConsensusMetrics(),
			dataActions.loadEconomicMetrics()
		]);
	}
};

// WebSocket connection management
export const websocketActions = {
	connectAll() {
		// Connect to all services for real-time updates
		wsManager.connect('trustchain', (data) => {
			if (data.type === 'certificate_issued') {
				dataActions.loadCertificates();
				notificationActions.add({
					type: 'success',
					title: 'Certificate Issued',
					message: `New certificate issued: ${data.subject}`
				});
			}
		});

		wsManager.connect('stoq', (data) => {
			if (data.type === 'metrics_update') {
				stoqMetrics.set(data.metrics);
			}
		});

		wsManager.connect('hypermesh', (data) => {
			if (data.type === 'asset_added') {
				dataActions.loadAssets();
				notificationActions.add({
					type: 'info',
					title: 'New Asset',
					message: `Asset added: ${data.asset.name}`
				});
			}
		});

		wsManager.connect('caesar', (data) => {
			if (data.type === 'reward_earned') {
				notificationActions.add({
					type: 'success',
					title: 'Reward Earned',
					message: `${data.amount} CAESAR earned from ${data.source}`
				});
			}
		});

		wsManager.connect('consensus', (data) => {
			if (data.type === 'new_block') {
				consensusMetrics.update(current => {
					if (current) {
						return { ...current, blockHeight: data.height };
					}
					return current;
				});
			}
		});
	},

	disconnectAll() {
		wsManager.disconnectAll();
	}
};

// Initialize theme from localStorage
export function initializeTheme() {
	const savedTheme = localStorage.getItem('theme');
	const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
	const isDark = savedTheme === 'dark' || (!savedTheme && prefersDark);
	
	dashboardState.update(state => ({ ...state, darkMode: isDark }));
	
	if (isDark) {
		document.documentElement.classList.add('dark');
	}
}

// Auto-refresh data
export function startAutoRefresh() {
	const interval = setInterval(() => {
		const state = get(dashboardState);
		if (state.autoRefresh) {
			dataActions.loadAllData();
		}
	}, 30000); // 30 seconds

	return () => clearInterval(interval);
}
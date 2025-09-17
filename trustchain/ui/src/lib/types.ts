// Web3 Ecosystem Types

export interface SystemStatus {
	name: string;
	status: 'online' | 'offline' | 'degraded' | 'error';
	uptime: number;
	lastChecked: string;
	metrics?: Record<string, any>;
}

// TrustChain Types
export interface Certificate {
	id: string;
	subject: string;
	issuer: string;
	serialNumber: string;
	fingerprint: string;
	validFrom: string;
	validTo: string;
	status: 'valid' | 'expired' | 'revoked' | 'pending';
	keyAlgorithm: string;
	signatureAlgorithm: string;
	extensions: CertificateExtension[];
}

export interface CertificateExtension {
	oid: string;
	critical: boolean;
	value: string;
}

export interface CertificateAuthority {
	id: string;
	name: string;
	rootCertificate: Certificate;
	intermediateCerts: Certificate[];
	status: SystemStatus;
	issuedCertificates: number;
	revokedCertificates: number;
}

// STOQ Protocol Types
export interface STOQConnection {
	id: string;
	remoteAddress: string;
	status: 'connected' | 'connecting' | 'disconnected' | 'failed';
	protocol: 'QUIC' | 'UDP' | 'TCP';
	encryptionLevel: 'none' | 'aes256' | 'quantum-safe';
	bandwidth: {
		upload: number;
		download: number;
		total: number;
	};
	latency: number;
	packetLoss: number;
	connectedAt: string;
}

export interface STOQMetrics {
	totalConnections: number;
	activeConnections: number;
	totalBandwidth: number;
	averageLatency: number;
	packetLoss: number;
	quantumSafeConnections: number;
	falconSigned: number;
}

// HyperMesh Asset Types
export interface Asset {
	id: string;
	type: 'cpu' | 'gpu' | 'memory' | 'storage' | 'network' | 'service';
	name: string;
	description: string;
	owner: string;
	location: AssetLocation;
	specifications: AssetSpecs;
	allocation: AssetAllocation;
	status: AssetStatus;
	privacy: PrivacySettings;
	createdAt: string;
	updatedAt: string;
}

export interface AssetLocation {
	nodeId: string;
	ipv6Address: string;
	region: string;
	zone: string;
	proxyAddress?: string;
}

export interface AssetSpecs {
	[key: string]: any;
	// CPU specific
	cores?: number;
	frequency?: number;
	architecture?: string;
	// GPU specific
	compute?: number;
	memory?: number;
	// Storage specific
	capacity?: number;
	type?: 'ssd' | 'hdd' | 'nvme';
	// Memory specific
	size?: number;
	speed?: number;
}

export interface AssetAllocation {
	allocated: number;
	available: number;
	reserved: number;
	unit: string;
}

export interface AssetStatus {
	health: 'healthy' | 'warning' | 'critical' | 'offline';
	temperature?: number;
	powerUsage?: number;
	utilization: number;
	lastHealthCheck: string;
}

export interface PrivacySettings {
	level: 'private' | 'private-network' | 'p2p' | 'public-network' | 'full-public';
	allowedNetworks?: string[];
	trustedPeers?: string[];
	maxConcurrentUsers: number;
	requireConsensus: boolean;
	proofs: {
		space: boolean;
		stake: boolean;
		work: boolean;
		time: boolean;
	};
}

// Caesar Economics Types
export interface EconomicReward {
	id: string;
	assetId: string;
	amount: number;
	currency: 'CAESAR' | 'BTC' | 'ETH';
	earnedAt: string;
	description: string;
	confirmed: boolean;
}

export interface UserBalance {
	total: number;
	available: number;
	locked: number;
	pending: number;
	currency: string;
}

export interface EconomicMetrics {
	totalRewards: number;
	activeAssets: number;
	averageRewardRate: number;
	stakingParticipation: number;
	networkValue: number;
}

// Consensus Types
export interface ConsensusProof {
	type: 'space' | 'stake' | 'work' | 'time';
	status: 'valid' | 'invalid' | 'pending' | 'expired';
	data: string;
	timestamp: string;
	validatedBy: string[];
}

export interface ConsensusBlock {
	height: number;
	hash: string;
	previousHash: string;
	timestamp: string;
	transactions: number;
	proofs: ConsensusProof[];
	validator: string;
	size: number;
}

export interface ConsensusMetrics {
	blockHeight: number;
	blockTime: number;
	validators: number;
	finalityTime: number;
	tps: number;
	proofCoverage: {
		space: number;
		stake: number;
		work: number;
		time: number;
	};
}

// UI State Types
export interface DashboardState {
	selectedSystem: string | null;
	timeRange: '1h' | '24h' | '7d' | '30d';
	autoRefresh: boolean;
	darkMode: boolean;
}

export interface NotificationState {
	id: string;
	type: 'info' | 'success' | 'warning' | 'error';
	title: string;
	message: string;
	timestamp: string;
	read: boolean;
	actions?: NotificationAction[];
}

export interface NotificationAction {
	label: string;
	action: () => void;
	style?: 'primary' | 'secondary' | 'destructive';
}

// API Response Types
export interface ApiResponse<T> {
	success: boolean;
	data?: T;
	error?: string;
	timestamp: string;
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	pageSize: number;
	hasNext: boolean;
	hasPrev: boolean;
}
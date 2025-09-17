import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number, decimals = 2): string {
	if (bytes === 0) return '0 Bytes';
	
	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
	
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	
	return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

export function formatSpeed(bps: number): string {
	if (bps === 0) return '0 bps';
	
	const k = 1000; // Use 1000 for network speeds (not 1024)
	const sizes = ['bps', 'Kbps', 'Mbps', 'Gbps', 'Tbps'];
	
	const i = Math.floor(Math.log(bps) / Math.log(k));
	
	return parseFloat((bps / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatDuration(ms: number): string {
	if (ms < 1000) return `${ms}ms`;
	if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
	if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
	return `${(ms / 3600000).toFixed(1)}h`;
}

export function formatNumber(num: number): string {
	if (num >= 1e9) return (num / 1e9).toFixed(1) + 'B';
	if (num >= 1e6) return (num / 1e6).toFixed(1) + 'M';
	if (num >= 1e3) return (num / 1e3).toFixed(1) + 'K';
	return num.toString();
}

export function getStatusColor(status: string): string {
	switch (status.toLowerCase()) {
		case 'online':
		case 'active':
		case 'connected':
		case 'healthy':
			return 'text-green-500';
		case 'warning':
		case 'degraded':
			return 'text-yellow-500';
		case 'offline':
		case 'error':
		case 'failed':
		case 'unhealthy':
			return 'text-red-500';
		case 'pending':
		case 'loading':
			return 'text-blue-500';
		default:
			return 'text-gray-500';
	}
}

export function copyToClipboard(text: string): Promise<void> {
	return navigator.clipboard.writeText(text);
}

export function debounce<T extends (...args: any[]) => any>(
	func: T,
	delay: number
): (...args: Parameters<T>) => void {
	let timeoutId: NodeJS.Timeout;
	return (...args: Parameters<T>) => {
		clearTimeout(timeoutId);
		timeoutId = setTimeout(() => func(...args), delay);
	};
}

export function throttle<T extends (...args: any[]) => any>(
	func: T,
	limit: number
): (...args: Parameters<T>) => void {
	let inThrottle: boolean;
	return (...args: Parameters<T>) => {
		if (!inThrottle) {
			func(...args);
			inThrottle = true;
			setTimeout(() => (inThrottle = false), limit);
		}
	};
}

// IPv6 validation for Web3 ecosystem
export function isValidIPv6(ip: string): boolean {
	const ipv6Regex = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^::1$|^::$/;
	return ipv6Regex.test(ip);
}

// Asset ID validation for HyperMesh
export function isValidAssetId(id: string): boolean {
	// HyperMesh Asset IDs are 64-character hex strings
	const assetIdRegex = /^[0-9a-fA-F]{64}$/;
	return assetIdRegex.test(id);
}

// Certificate fingerprint validation for TrustChain
export function isValidCertFingerprint(fingerprint: string): boolean {
	// SHA-256 fingerprints are 64-character hex strings
	const fingerprintRegex = /^[0-9a-fA-F]{64}$/;
	return fingerprintRegex.test(fingerprint);
}
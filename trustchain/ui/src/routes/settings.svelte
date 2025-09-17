<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Separator from '$lib/components/ui/separator.svelte';
	import { Settings, Network, Shield, Globe, Save, RefreshCw } from 'lucide-svelte';

	let nodeSettings = $state({
		nodeId: 'node-001',
		ipv6Address: '2001:db8::1001',
		region: 'us-west-2',
		zone: 'us-west-2a',
		proxyEnabled: true,
		autoDiscovery: true,
		maxConnections: 1000,
		bandwidth: {
			upload: 1000, // Mbps
			download: 1000 // Mbps
		}
	});

	let securitySettings = $state({
		quantumSafe: true,
		falconSigning: true,
		kyberKeyExchange: true,
		tlsVersion: '1.3',
		certificateValidation: 'strict',
		ocspStapling: true,
		hsts: true
	});

	let consensusSettings = $state({
		participateInConsensus: true,
		requiredProofs: {
			space: true,
			stake: true,
			work: true,
			time: true
		},
		validatorMode: false,
		stakingAmount: 0,
		slashingProtection: true
	});

	let privacySettings = $state({
		defaultPrivacyLevel: 'public-network',
		allowAnonymousConnections: false,
		logRetention: 30, // days
		metricsCollection: true,
		shareUsageStats: true,
		trustedNetworks: ['2001:db8::/32']
	});

	let performanceSettings = $state({
		quicOptimization: true,
		compressionLevel: 6,
		cacheSize: 1024, // MB
		workerThreads: 8,
		asyncIO: true,
		zeroCopyEnabled: true
	});

	let saving = $state(false);
	let lastSaved = $state<Date | null>(null);

	function saveSettings() {
		saving = true;
		
		// Simulate saving settings
		setTimeout(() => {
			saving = false;
			lastSaved = new Date();
			console.log('Settings saved:', {
				nodeSettings,
				securitySettings,
				consensusSettings,
				privacySettings,
				performanceSettings
			});
		}, 1500);
	}

	function resetToDefaults() {
		// Reset all settings to defaults
		nodeSettings.maxConnections = 1000;
		nodeSettings.autoDiscovery = true;
		securitySettings.quantumSafe = true;
		consensusSettings.participateInConsensus = true;
		privacySettings.defaultPrivacyLevel = 'public-network';
		performanceSettings.quicOptimization = true;
	}

	function testConfiguration() {
		console.log('Testing configuration...');
	}

	onMount(() => {
		// Load saved settings from localStorage or API
		const savedSettings = localStorage.getItem('web3-settings');
		if (savedSettings) {
			try {
				const parsed = JSON.parse(savedSettings);
				if (parsed.lastSaved) {
					lastSaved = new Date(parsed.lastSaved);
				}
			} catch (e) {
				console.warn('Failed to parse saved settings');
			}
		}
	});
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<Settings class="h-8 w-8 text-primary mr-3" />
				System Settings
			</h1>
			<p class="text-muted-foreground mt-2">
				Configure your Web3 ecosystem node and services
			</p>
		</div>
		<div class="flex items-center space-x-4">
			{#if lastSaved}
				<div class="text-sm text-muted-foreground">
					Last saved: {lastSaved.toLocaleTimeString()}
				</div>
			{/if}
			<Button variant="outline" onclick={testConfiguration}>
				<RefreshCw class="h-4 w-4 mr-2" />
				Test
			</Button>
			<Button onclick={saveSettings} disabled={saving}>
				<Save class="h-4 w-4 mr-2" />
				{saving ? 'Saving...' : 'Save Settings'}
			</Button>
		</div>
	</div>

	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
		<!-- Node Configuration -->
		<Card class="p-6">
			<div class="flex items-center space-x-2 mb-4">
				<Network class="h-5 w-5 text-blue-600" />
				<h3 class="font-semibold text-lg">Node Configuration</h3>
			</div>
			
			<div class="space-y-4">
				<div>
					<label class="text-sm font-medium">Node ID</label>
					<input
						type="text"
						bind:value={nodeSettings.nodeId}
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						readonly
					/>
				</div>
				
				<div>
					<label class="text-sm font-medium">IPv6 Address</label>
					<input
						type="text"
						bind:value={nodeSettings.ipv6Address}
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					/>
				</div>
				
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label class="text-sm font-medium">Region</label>
						<select
							bind:value={nodeSettings.region}
							class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						>
							<option value="us-west-2">US West 2</option>
							<option value="us-east-1">US East 1</option>
							<option value="eu-central-1">EU Central 1</option>
							<option value="ap-southeast-1">AP Southeast 1</option>
						</select>
					</div>
					<div>
						<label class="text-sm font-medium">Max Connections</label>
						<input
							type="number"
							bind:value={nodeSettings.maxConnections}
							min="100"
							max="10000"
							class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						/>
					</div>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="proxy-enabled"
						bind:checked={nodeSettings.proxyEnabled}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="proxy-enabled" class="text-sm font-medium">Enable NAT-like Proxy</label>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="auto-discovery"
						bind:checked={nodeSettings.autoDiscovery}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="auto-discovery" class="text-sm font-medium">Auto-discovery</label>
				</div>
			</div>
		</Card>

		<!-- Security Settings -->
		<Card class="p-6">
			<div class="flex items-center space-x-2 mb-4">
				<Shield class="h-5 w-5 text-quantum-600" />
				<h3 class="font-semibold text-lg">Security Settings</h3>
			</div>
			
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<div>
						<label class="text-sm font-medium">Quantum-Safe Cryptography</label>
						<p class="text-xs text-muted-foreground">FALCON-1024 signatures</p>
					</div>
					<input
						type="checkbox"
						bind:checked={securitySettings.quantumSafe}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
				</div>
				
				<div class="flex items-center justify-between">
					<div>
						<label class="text-sm font-medium">FALCON Signing</label>
						<p class="text-xs text-muted-foreground">Post-quantum signatures</p>
					</div>
					<input
						type="checkbox"
						bind:checked={securitySettings.falconSigning}
						disabled={!securitySettings.quantumSafe}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
				</div>
				
				<div class="flex items-center justify-between">
					<div>
						<label class="text-sm font-medium">Kyber Key Exchange</label>
						<p class="text-xs text-muted-foreground">Quantum-resistant KEM</p>
					</div>
					<input
						type="checkbox"
						bind:checked={securitySettings.kyberKeyExchange}
						disabled={!securitySettings.quantumSafe}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
				</div>
				
				<div>
					<label class="text-sm font-medium">Certificate Validation</label>
					<select
						bind:value={securitySettings.certificateValidation}
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					>
						<option value="strict">Strict</option>
						<option value="moderate">Moderate</option>
						<option value="permissive">Permissive</option>
					</select>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="ocsp-stapling"
						bind:checked={securitySettings.ocspStapling}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="ocsp-stapling" class="text-sm font-medium">OCSP Stapling</label>
				</div>
			</div>
		</Card>

		<!-- Consensus Settings -->
		<Card class="p-6">
			<div class="flex items-center space-x-2 mb-4">
				<Shield class="h-5 w-5 text-green-600" />
				<h3 class="font-semibold text-lg">Consensus Participation</h3>
			</div>
			
			<div class="space-y-4">
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="participate-consensus"
						bind:checked={consensusSettings.participateInConsensus}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="participate-consensus" class="text-sm font-medium">Participate in Consensus</label>
				</div>
				
				<div class="space-y-2">
					<label class="text-sm font-medium">Required Proofs</label>
					<div class="grid grid-cols-2 gap-2">
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="proof-space"
								bind:checked={consensusSettings.requiredProofs.space}
								class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
							/>
							<label for="proof-space" class="text-xs">PoSpace</label>
						</div>
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="proof-stake"
								bind:checked={consensusSettings.requiredProofs.stake}
								class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
							/>
							<label for="proof-stake" class="text-xs">PoStake</label>
						</div>
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="proof-work"
								bind:checked={consensusSettings.requiredProofs.work}
								class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
							/>
							<label for="proof-work" class="text-xs">PoWork</label>
						</div>
						<div class="flex items-center space-x-2">
							<input
								type="checkbox"
								id="proof-time"
								bind:checked={consensusSettings.requiredProofs.time}
								class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
							/>
							<label for="proof-time" class="text-xs">PoTime</label>
						</div>
					</div>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="validator-mode"
						bind:checked={consensusSettings.validatorMode}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="validator-mode" class="text-sm font-medium">Validator Mode</label>
				</div>
				
				{#if consensusSettings.validatorMode}
					<div>
						<label class="text-sm font-medium">Staking Amount (CAESAR)</label>
						<input
							type="number"
							bind:value={consensusSettings.stakingAmount}
							min="10000"
							class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						/>
					</div>
				{/if}
			</div>
		</Card>

		<!-- Privacy Settings -->
		<Card class="p-6">
			<div class="flex items-center space-x-2 mb-4">
				<Globe class="h-5 w-5 text-purple-600" />
				<h3 class="font-semibold text-lg">Privacy Settings</h3>
			</div>
			
			<div class="space-y-4">
				<div>
					<label class="text-sm font-medium">Default Privacy Level</label>
					<select
						bind:value={privacySettings.defaultPrivacyLevel}
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					>
						<option value="private">Private</option>
						<option value="private-network">Private Network</option>
						<option value="p2p">P2P</option>
						<option value="public-network">Public Network</option>
						<option value="full-public">Full Public</option>
					</select>
				</div>
				
				<div>
					<label class="text-sm font-medium">Log Retention (Days)</label>
					<input
						type="number"
						bind:value={privacySettings.logRetention}
						min="1"
						max="365"
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					/>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="anonymous-connections"
						bind:checked={privacySettings.allowAnonymousConnections}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="anonymous-connections" class="text-sm font-medium">Allow Anonymous Connections</label>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="metrics-collection"
						bind:checked={privacySettings.metricsCollection}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="metrics-collection" class="text-sm font-medium">Collect Performance Metrics</label>
				</div>
				
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="share-stats"
						bind:checked={privacySettings.shareUsageStats}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="share-stats" class="text-sm font-medium">Share Usage Statistics</label>
				</div>
			</div>
		</Card>
	</div>

	<!-- Performance Tuning -->
	<Card class="p-6 mt-8">
		<h3 class="font-semibold text-lg mb-4">Performance Optimization</h3>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="space-y-4">
				<h4 class="font-medium">STOQ Protocol</h4>
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="quic-optimization"
						bind:checked={performanceSettings.quicOptimization}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="quic-optimization" class="text-sm">QUIC Optimization</label>
				</div>
				<div>
					<label class="text-sm font-medium">Compression Level</label>
					<input
						type="range"
						min="1"
						max="9"
						bind:value={performanceSettings.compressionLevel}
						class="w-full"
					/>
					<div class="text-xs text-muted-foreground">Level: {performanceSettings.compressionLevel}</div>
				</div>
			</div>
			
			<div class="space-y-4">
				<h4 class="font-medium">System Resources</h4>
				<div>
					<label class="text-sm font-medium">Cache Size (MB)</label>
					<input
						type="number"
						bind:value={performanceSettings.cacheSize}
						min="128"
						max="8192"
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					/>
				</div>
				<div>
					<label class="text-sm font-medium">Worker Threads</label>
					<input
						type="number"
						bind:value={performanceSettings.workerThreads}
						min="1"
						max="32"
						class="w-full mt-1 px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					/>
				</div>
			</div>
			
			<div class="space-y-4">
				<h4 class="font-medium">Advanced</h4>
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="async-io"
						bind:checked={performanceSettings.asyncIO}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="async-io" class="text-sm">Async I/O</label>
				</div>
				<div class="flex items-center space-x-2">
					<input
						type="checkbox"
						id="zero-copy"
						bind:checked={performanceSettings.zeroCopyEnabled}
						class="w-4 h-4 text-primary border-gray-300 rounded focus:ring-primary"
					/>
					<label for="zero-copy" class="text-sm">Zero-Copy Networking</label>
				</div>
			</div>
		</div>
	</Card>

	<!-- Actions -->
	<div class="flex justify-between items-center mt-8">
		<Button variant="outline" onclick={resetToDefaults}>
			Reset to Defaults
		</Button>
		<div class="flex space-x-3">
			<Button variant="outline" onclick={testConfiguration}>
				Test Configuration
			</Button>
			<Button onclick={saveSettings} disabled={saving}>
				{saving ? 'Saving...' : 'Save All Settings'}
			</Button>
		</div>
	</div>
</Layout>
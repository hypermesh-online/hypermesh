<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Progress from '$lib/components/ui/progress.svelte';
	import { Network, Shield, Zap, Activity, Settings, RefreshCw } from 'lucide-svelte';
	import type { STOQConnection, STOQMetrics } from '$lib/types.js';
	import { formatSpeed, formatDuration } from '$lib/utils.js';

	// Mock data
	let stoqMetrics: STOQMetrics = $state({
		totalConnections: 1247,
		activeConnections: 445,
		totalBandwidth: 2.95e9, // 2.95 Gbps in bps
		averageLatency: 23.5,
		packetLoss: 0.01,
		quantumSafeConnections: 445,
		falconSigned: 892
	});

	let connections: STOQConnection[] = $state([
		{
			id: 'conn-001',
			remoteAddress: '2001:db8::1001',
			status: 'connected',
			protocol: 'QUIC',
			encryptionLevel: 'quantum-safe',
			bandwidth: {
				upload: 125000000, // 1 Gbps
				download: 250000000, // 2 Gbps
				total: 375000000
			},
			latency: 18.5,
			packetLoss: 0.001,
			connectedAt: '2024-09-17T08:30:00Z'
		},
		{
			id: 'conn-002',
			remoteAddress: '2001:db8::2001',
			status: 'connected',
			protocol: 'QUIC',
			encryptionLevel: 'quantum-safe',
			bandwidth: {
				upload: 62500000, // 500 Mbps
				download: 125000000, // 1 Gbps
				total: 187500000
			},
			latency: 45.2,
			packetLoss: 0.005,
			connectedAt: '2024-09-17T07:15:00Z'
		},
		{
			id: 'conn-003',
			remoteAddress: '2001:db8::3001',
			status: 'connecting',
			protocol: 'QUIC',
			encryptionLevel: 'quantum-safe',
			bandwidth: {
				upload: 0,
				download: 0,
				total: 0
			},
			latency: 0,
			packetLoss: 0,
			connectedAt: new Date().toISOString()
		}
	]);

	let performanceTarget = {
		targetBandwidth: 40e9, // 40 Gbps
		currentBandwidth: 2.95e9, // 2.95 Gbps
		bottleneck: 'QUIC Implementation',
		optimization: 'In Progress'
	};

	let loading = $state(false);
	let lastRefresh = $state(new Date());

	onMount(() => {
		// Set up auto-refresh every 10 seconds for real-time metrics
		const interval = setInterval(() => {
			refreshMetrics();
		}, 10000);

		return () => clearInterval(interval);
	});

	function refreshMetrics() {
		lastRefresh = new Date();
		
		// Simulate real-time updates
		stoqMetrics.totalBandwidth += (Math.random() - 0.5) * 1e8; // ±100 Mbps variation
		stoqMetrics.averageLatency += (Math.random() - 0.5) * 5; // ±2.5ms variation
		stoqMetrics.activeConnections += Math.floor((Math.random() - 0.5) * 6); // ±3 connections
		
		// Keep values in reasonable ranges
		stoqMetrics.totalBandwidth = Math.max(2e9, Math.min(4e9, stoqMetrics.totalBandwidth));
		stoqMetrics.averageLatency = Math.max(15, Math.min(50, stoqMetrics.averageLatency));
		stoqMetrics.activeConnections = Math.max(400, Math.min(500, stoqMetrics.activeConnections));
	}

	function optimizePerformance() {
		loading = true;
		setTimeout(() => {
			loading = false;
			// Simulate slight performance improvement
			stoqMetrics.totalBandwidth *= 1.05;
			performanceTarget.currentBandwidth = stoqMetrics.totalBandwidth;
		}, 2000);
	}

	function configureProtocol() {
		console.log('Configure STOQ protocol settings');
	}

	$: performancePercentage = (performanceTarget.currentBandwidth / performanceTarget.targetBandwidth) * 100;
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<Network class="h-8 w-8 text-stoq-600 mr-3" />
				STOQ Protocol Dashboard
			</h1>
			<p class="text-muted-foreground mt-2">
				Quantum-resistant transport layer with QUIC optimization
			</p>
		</div>
		<div class="flex items-center space-x-4">
			<div class="text-sm text-muted-foreground">
				Last updated: {lastRefresh.toLocaleTimeString()}
			</div>
			<Button variant="outline" onclick={configureProtocol}>
				<Settings class="h-4 w-4 mr-2" />
				Configure
			</Button>
			<Button onclick={optimizePerformance} disabled={loading}>
				<Zap class="h-4 w-4 mr-2" />
				{loading ? 'Optimizing...' : 'Optimize'}
			</Button>
		</div>
	</div>

	<!-- Key Metrics -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Total Bandwidth"
			value={formatSpeed(stoqMetrics.totalBandwidth)}
			change={-26.2}
			changeLabel="vs. 40 Gbps target"
			icon={Network}
			color="text-stoq-600"
		/>
		<MetricsCard
			title="Active Connections"
			value={stoqMetrics.activeConnections}
			change={2.1}
			changeLabel="this hour"
			icon={Activity}
			color="text-green-600"
		/>
		<MetricsCard
			title="Average Latency"
			value={stoqMetrics.averageLatency.toFixed(1)}
			unit="ms"
			change={-5.3}
			changeLabel="improvement"
			icon={Zap}
			color="text-blue-600"
		/>
		<MetricsCard
			title="Quantum Safe"
			value={((stoqMetrics.quantumSafeConnections / stoqMetrics.activeConnections) * 100).toFixed(1)}
			unit="%"
			icon={Shield}
			color="text-quantum-600"
		/>
	</div>

	<!-- Performance Status Banner -->
	<Card class="p-6 mb-8 bg-gradient-to-r from-stoq-50 to-blue-50 border-stoq-200">
		<div class="flex items-center justify-between">
			<div class="flex items-center space-x-4">
				<div class="p-3 bg-stoq-100 rounded-lg">
					<Network class="h-6 w-6 text-stoq-600" />
				</div>
				<div>
					<h3 class="font-semibold text-lg">Performance Optimization Status</h3>
					<p class="text-muted-foreground">Current bottleneck: {performanceTarget.bottleneck}</p>
				</div>
			</div>
			<div class="flex items-center space-x-4">
				<div class="text-right">
					<div class="font-medium">{formatSpeed(performanceTarget.currentBandwidth)} / {formatSpeed(performanceTarget.targetBandwidth)}</div>
					<div class="text-sm text-muted-foreground">{performancePercentage.toFixed(1)}% of target</div>
				</div>
				<Badge variant={performanceTarget.optimization === 'In Progress' ? 'secondary' : 'default'}>
					{performanceTarget.optimization}
				</Badge>
			</div>
		</div>
		<div class="mt-4">
			<Progress value={performancePercentage} class="h-3" />
		</div>
	</Card>

	<!-- Protocol Details -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
		<!-- QUIC Configuration -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">QUIC Protocol Configuration</h3>
			<div class="space-y-4">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Protocol Version:</span>
					<span class="font-medium">QUIC v2 (RFC 9369)</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Congestion Control:</span>
					<span class="font-medium">BBR v3</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Max Streams:</span>
					<span class="font-medium">1,000 bidirectional</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Initial RTT:</span>
					<span class="font-medium">100ms</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Idle Timeout:</span>
					<span class="font-medium">30 seconds</span>
				</div>
				<div class="pt-3 border-t">
					<div class="text-sm text-muted-foreground mb-2">Performance Issues:</div>
					<div class="text-xs space-y-1">
						<div class="flex items-center space-x-2">
							<div class="w-2 h-2 bg-red-500 rounded-full"></div>
							<span>Stream multiplexing bottleneck</span>
						</div>
						<div class="flex items-center space-x-2">
							<div class="w-2 h-2 bg-yellow-500 rounded-full"></div>
							<span>Connection migration overhead</span>
						</div>
						<div class="flex items-center space-x-2">
							<div class="w-2 h-2 bg-yellow-500 rounded-full"></div>
							<span>0-RTT handshake optimization needed</span>
						</div>
					</div>
				</div>
			</div>
		</Card>

		<!-- Quantum Cryptography -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">FALCON-1024 Quantum Security</h3>
			<div class="space-y-4">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Signature Algorithm:</span>
					<Badge class="quantum-glow">FALCON-1024</Badge>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Key Exchange:</span>
					<span class="font-medium">Kyber-1024</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Hash Function:</span>
					<span class="font-medium">SHAKE-256</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Signatures Generated:</span>
					<span class="font-medium">{stoqMetrics.falconSigned.toLocaleString()}</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Verification Rate:</span>
					<span class="font-medium">99.99%</span>
				</div>
				<div class="pt-3 border-t">
					<div class="text-sm text-muted-foreground mb-2">Security Status:</div>
					<div class="space-y-1">
						<div class="flex justify-between text-xs">
							<span>Quantum Resistance:</span>
							<span class="text-green-600 font-medium">NIST Level 5</span>
						</div>
						<div class="flex justify-between text-xs">
							<span>Classical Security:</span>
							<span class="text-green-600 font-medium">256-bit equivalent</span>
						</div>
						<div class="flex justify-between text-xs">
							<span>Performance Impact:</span>
							<span class="text-yellow-600 font-medium">~15% overhead</span>
						</div>
					</div>
				</div>
			</div>
		</Card>
	</div>

	<!-- Active Connections -->
	<Card class="p-6 mb-8">
		<div class="flex items-center justify-between mb-6">
			<h3 class="font-semibold text-lg">Active Connections</h3>
			<Badge variant="outline">{connections.length} shown of {stoqMetrics.activeConnections}</Badge>
		</div>
		
		<div class="space-y-4">
			{#each connections as connection}
				<div class="border rounded-lg p-4">
					<div class="flex items-center justify-between mb-3">
						<div class="flex items-center space-x-3">
							<div class={`w-3 h-3 rounded-full ${
								connection.status === 'connected' ? 'bg-green-500' :
								connection.status === 'connecting' ? 'bg-yellow-500 animate-pulse' :
								'bg-red-500'
							}`}></div>
							<div>
								<code class="text-sm font-mono">{connection.remoteAddress}</code>
								<div class="text-xs text-muted-foreground">
									Connected {formatDuration(Date.now() - new Date(connection.connectedAt).getTime())} ago
								</div>
							</div>
						</div>
						<div class="flex items-center space-x-2">
							<Badge variant="outline">{connection.protocol}</Badge>
							<Badge class="quantum-glow">
								{connection.encryptionLevel === 'quantum-safe' ? 'FALCON-1024' : connection.encryptionLevel}
							</Badge>
						</div>
					</div>
					
					{#if connection.status === 'connected'}
						<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
							<div>
								<span class="text-muted-foreground">Upload:</span>
								<div class="font-medium">{formatSpeed(connection.bandwidth.upload)}</div>
							</div>
							<div>
								<span class="text-muted-foreground">Download:</span>
								<div class="font-medium">{formatSpeed(connection.bandwidth.download)}</div>
							</div>
							<div>
								<span class="text-muted-foreground">Latency:</span>
								<div class="font-medium">{connection.latency.toFixed(1)}ms</div>
							</div>
							<div>
								<span class="text-muted-foreground">Packet Loss:</span>
								<div class="font-medium">{(connection.packetLoss * 100).toFixed(3)}%</div>
							</div>
						</div>
					{:else}
						<div class="text-sm text-muted-foreground">
							{connection.status === 'connecting' ? 'Establishing connection...' : 'Connection failed'}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</Card>

	<!-- Optimization Recommendations -->
	<Card class="p-6">
		<h3 class="font-semibold text-lg mb-4">Performance Optimization</h3>
		<div class="space-y-4">
			<div class="border-l-4 border-red-500 pl-4">
				<h4 class="font-medium text-red-700">Critical: QUIC Stream Multiplexing</h4>
				<p class="text-sm text-muted-foreground mt-1">
					Current implementation shows bottlenecks in stream handling. 
					Recommend upgrading to optimized QUIC stack with better stream scheduling.
				</p>
			</div>
			<div class="border-l-4 border-yellow-500 pl-4">
				<h4 class="font-medium text-yellow-700">Medium: Connection Migration</h4>
				<p class="text-sm text-muted-foreground mt-1">
					Frequent connection migrations causing overhead. 
					Consider implementing path validation caching.
				</p>
			</div>
			<div class="border-l-4 border-blue-500 pl-4">
				<h4 class="font-medium text-blue-700">Low: 0-RTT Optimization</h4>
				<p class="text-sm text-muted-foreground mt-1">
					Enable 0-RTT for repeat connections to reduce handshake time.
				</p>
			</div>
		</div>
	</Card>
</Layout>
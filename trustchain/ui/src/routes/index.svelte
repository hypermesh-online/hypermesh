<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import SystemStatusCard from '$lib/components/web3/SystemStatusCard.svelte';
	import AssetCard from '$lib/components/web3/AssetCard.svelte';
	import CertificateCard from '$lib/components/web3/CertificateCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Progress from '$lib/components/ui/progress.svelte';
	import { 
		Shield, 
		Network, 
		HardDrive, 
		Coins, 
		Activity, 
		Zap,
		Lock,
		Globe,
		RefreshCw
	} from 'lucide-svelte';
	import type { SystemStatus, Asset, Certificate } from '$lib/types.js';

	// Mock data - in real app this would come from API
	let ecosystemMetrics = $state({
		totalAssets: 1247,
		activeCertificates: 892,
		networkThroughput: 2.95,
		consensusBlocks: 15234,
		quantumConnections: 445,
		economicRewards: 12847.32
	});

	let systemStatuses: SystemStatus[] = $state([
		{
			name: 'TrustChain CA',
			status: 'online',
			uptime: 2592000000, // 30 days
			lastChecked: new Date().toISOString(),
			metrics: {
				'Certificates Issued': '892',
				'Root CAs': '3',
				'Revoked Certs': '12'
			}
		},
		{
			name: 'STOQ Protocol',
			status: 'online',
			uptime: 2505600000, // 29 days
			lastChecked: new Date().toISOString(),
			metrics: {
				'Current Throughput': '2.95 Gbps',
				'Active Connections': '445',
				'Quantum Safe': '100%'
			}
		},
		{
			name: 'HyperMesh Network',
			status: 'online',
			uptime: 2419200000, // 28 days
			lastChecked: new Date().toISOString(),
			metrics: {
				'Total Assets': '1,247',
				'Active Nodes': '156',
				'Asset Utilization': '67%'
			}
		},
		{
			name: 'Caesar Economics',
			status: 'online',
			uptime: 2332800000, // 27 days
			lastChecked: new Date().toISOString(),
			metrics: {
				'Total Rewards': '12,847.32 CAESAR',
				'Staking Rate': '34%',
				'Network Value': '$2.4M'
			}
		},
		{
			name: 'Four-Proof Consensus',
			status: 'online',
			uptime: 2246400000, // 26 days
			lastChecked: new Date().toISOString(),
			metrics: {
				'Block Height': '15,234',
				'Validators': '67',
				'Finality Time': '2.3s'
			}
		}
	]);

	let loading = $state(true);
	let lastRefresh = $state(new Date());

	onMount(() => {
		// Simulate loading
		setTimeout(() => {
			loading = false;
		}, 1000);

		// Set up auto-refresh every 30 seconds
		const interval = setInterval(() => {
			refreshData();
		}, 30000);

		return () => clearInterval(interval);
	});

	function refreshData() {
		loading = true;
		lastRefresh = new Date();
		
		// Simulate API call
		setTimeout(() => {
			// Update metrics with slight variations
			ecosystemMetrics.totalAssets += Math.floor(Math.random() * 5);
			ecosystemMetrics.networkThroughput = 2.95 + (Math.random() - 0.5) * 0.1;
			ecosystemMetrics.consensusBlocks += Math.floor(Math.random() * 3) + 1;
			
			loading = false;
		}, 500);
	}
</script>

<Layout>
	<!-- Dashboard Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight">Web3 Ecosystem Dashboard</h1>
			<p class="text-muted-foreground mt-2">
				Quantum-secure, user-sovereign internet infrastructure
			</p>
		</div>
		<div class="flex items-center space-x-4">
			<div class="text-sm text-muted-foreground">
				Last updated: {lastRefresh.toLocaleTimeString()}
			</div>
			<Button variant="outline" size="sm" onclick={refreshData} disabled={loading}>
				<RefreshCw class={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
				Refresh
			</Button>
		</div>
	</div>

	<!-- Key Metrics Grid -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Total Assets"
			value={ecosystemMetrics.totalAssets}
			change={2.4}
			changeLabel="from last week"
			icon={HardDrive}
			color="text-hypermesh-600"
		/>
		<MetricsCard
			title="Active Certificates"
			value={ecosystemMetrics.activeCertificates}
			change={1.2}
			changeLabel="from last week"
			icon={Shield}
			color="text-trustchain-600"
		/>
		<MetricsCard
			title="Network Throughput"
			value={ecosystemMetrics.networkThroughput.toFixed(2)}
			unit="Gbps"
			change={-0.3}
			changeLabel="from target"
			icon={Network}
			color="text-stoq-600"
		/>
		<MetricsCard
			title="Economic Rewards"
			value={ecosystemMetrics.economicRewards.toFixed(2)}
			unit="CAESAR"
			change={12.8}
			changeLabel="this month"
			icon={Coins}
			color="text-caesar-600"
		/>
	</div>

	<!-- Quantum Security Status Banner -->
	<Card class="p-6 mb-8 bg-gradient-to-r from-quantum-50 to-trustchain-50 border-quantum-200">
		<div class="flex items-center justify-between">
			<div class="flex items-center space-x-4">
				<div class="p-3 bg-quantum-100 rounded-lg">
					<Lock class="h-6 w-6 text-quantum-600" />
				</div>
				<div>
					<h3 class="font-semibold text-lg">Quantum Security Status</h3>
					<p class="text-muted-foreground">FALCON-1024 post-quantum cryptography active</p>
				</div>
			</div>
			<div class="flex items-center space-x-4">
				<Badge class="consensus-badge">4-Proof Consensus</Badge>
				<Badge variant="outline" class="border-green-500 text-green-700">IPv6 Ready</Badge>
				<div class="flex items-center space-x-2 text-green-600">
					<div class="status-indicator status-online"></div>
					<span class="font-medium">All Systems Operational</span>
				</div>
			</div>
		</div>
	</Card>

	<!-- System Status Grid -->
	<div class="mb-8">
		<h2 class="text-2xl font-semibold mb-6">System Status</h2>
		<div class="ecosystem-grid">
			{#each systemStatuses as system}
				<SystemStatusCard {system} showDetails={true} />
			{/each}
		</div>
	</div>

	<!-- Performance Overview -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
		<!-- STOQ Protocol Performance -->
		<Card class="p-6">
			<div class="flex items-center justify-between mb-4">
				<h3 class="font-semibold text-lg">STOQ Protocol Performance</h3>
				<Badge variant="outline">Target: 40 Gbps</Badge>
			</div>
			<div class="space-y-4">
				<div>
					<div class="flex justify-between text-sm mb-2">
						<span>Current Throughput</span>
						<span class="font-medium">{ecosystemMetrics.networkThroughput.toFixed(2)} Gbps</span>
					</div>
					<Progress value={(ecosystemMetrics.networkThroughput / 40) * 100} class="h-2" />
				</div>
				<div class="text-sm text-muted-foreground">
					Performance bottleneck identified in QUIC implementation. 
					Optimization in progress for Phase 1 deployment.
				</div>
			</div>
		</Card>

		<!-- Consensus Validation -->
		<Card class="p-6">
			<div class="flex items-center justify-between mb-4">
				<h3 class="font-semibold text-lg">Four-Proof Consensus</h3>
				<Badge class="consensus-badge">NKrypt Protocol</Badge>
			</div>
			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span>PoSpace</span>
						<span class="text-green-600 font-medium">✓ Active</span>
					</div>
					<div class="flex justify-between text-sm">
						<span>PoStake</span>
						<span class="text-green-600 font-medium">✓ Active</span>
					</div>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span>PoWork</span>
						<span class="text-green-600 font-medium">✓ Active</span>
					</div>
					<div class="flex justify-between text-sm">
						<span>PoTime</span>
						<span class="text-green-600 font-medium">✓ Active</span>
					</div>
				</div>
			</div>
		</Card>
	</div>

	<!-- Quick Actions -->
	<Card class="p-6">
		<h3 class="font-semibold text-lg mb-4">Quick Actions</h3>
		<div class="flex flex-wrap gap-3">
			<Button variant="outline">
				<Shield class="h-4 w-4 mr-2" />
				Issue Certificate
			</Button>
			<Button variant="outline">
				<HardDrive class="h-4 w-4 mr-2" />
				Add Asset
			</Button>
			<Button variant="outline">
				<Network class="h-4 w-4 mr-2" />
				Monitor Network
			</Button>
			<Button variant="outline">
				<Coins class="h-4 w-4 mr-2" />
				View Rewards
			</Button>
			<Button variant="outline">
				<Globe class="h-4 w-4 mr-2" />
				Network Settings
			</Button>
		</div>
	</Card>
</Layout>

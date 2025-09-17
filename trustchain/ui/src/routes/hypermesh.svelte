<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import AssetCard from '$lib/components/web3/AssetCard.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Progress from '$lib/components/ui/progress.svelte';
	import { HardDrive, Plus, Settings, Search, Filter, Cpu, Gpu, MemoryStick, Network } from 'lucide-svelte';
	import type { Asset } from '$lib/types.js';

	// Mock data
	let assets: Asset[] = $state([
		{
			id: 'asset-cpu-001',
			type: 'cpu',
			name: 'Intel Xeon E5-2690',
			description: '12-core server processor with hyperthreading',
			owner: 'node-001',
			location: {
				nodeId: 'node-001',
				ipv6Address: '2001:db8::1',
				region: 'us-west-2',
				zone: 'us-west-2a',
				proxyAddress: 'proxy://hypermesh.node.001'
			},
			specifications: {
				cores: 24,
				frequency: 2600,
				architecture: 'x86_64',
				cache: '30MB'
			},
			allocation: {
				allocated: 18,
				available: 6,
				reserved: 0,
				unit: 'cores'
			},
			status: {
				health: 'healthy',
				temperature: 62,
				powerUsage: 135,
				utilization: 75,
				lastHealthCheck: new Date().toISOString()
			},
			privacy: {
				level: 'public-network',
				maxConcurrentUsers: 3,
				requireConsensus: true,
				proofs: {
					space: true,
					stake: true,
					work: true,
					time: true
				}
			},
			createdAt: '2024-01-15T10:30:00Z',
			updatedAt: new Date().toISOString()
		},
		{
			id: 'asset-gpu-001',
			type: 'gpu',
			name: 'NVIDIA RTX 4090',
			description: 'High-performance GPU for compute workloads',
			owner: 'node-002',
			location: {
				nodeId: 'node-002',
				ipv6Address: '2001:db8::2',
				region: 'eu-central-1',
				zone: 'eu-central-1a'
			},
			specifications: {
				compute: 16384,
				memory: 24576,
				bandwidth: 1008,
				architecture: 'Ada Lovelace'
			},
			allocation: {
				allocated: 12288,
				available: 12288,
				reserved: 0,
				unit: 'MB VRAM'
			},
			status: {
				health: 'healthy',
				temperature: 68,
				powerUsage: 425,
				utilization: 45,
				lastHealthCheck: new Date().toISOString()
			},
			privacy: {
				level: 'p2p',
				trustedPeers: ['peer-abc123', 'peer-def456'],
				maxConcurrentUsers: 1,
				requireConsensus: false,
				proofs: {
					space: true,
					stake: false,
					work: true,
					time: false
				}
			},
			createdAt: '2024-02-01T14:20:00Z',
			updatedAt: new Date().toISOString()
		},
		{
			id: 'asset-storage-001',
			type: 'storage',
			name: 'Samsung 990 PRO NVMe',
			description: '2TB high-speed NVMe SSD storage',
			owner: 'node-003',
			location: {
				nodeId: 'node-003',
				ipv6Address: '2001:db8::3',
				region: 'ap-southeast-1',
				zone: 'ap-southeast-1b'
			},
			specifications: {
				capacity: 2048,
				type: 'nvme',
				readSpeed: 7000,
				writeSpeed: 6900,
				endurance: 1200
			},
			allocation: {
				allocated: 1536,
				available: 512,
				reserved: 0,
				unit: 'GB'
			},
			status: {
				health: 'warning',
				temperature: 72,
				utilization: 85,
				lastHealthCheck: new Date().toISOString()
			},
			privacy: {
				level: 'private',
				maxConcurrentUsers: 0,
				requireConsensus: true,
				proofs: {
					space: true,
					stake: true,
					work: false,
					time: true
				}
			},
			createdAt: '2024-01-20T09:15:00Z',
			updatedAt: new Date().toISOString()
		}
	]);

	let meshMetrics = $state({
		totalAssets: 1247,
		activeNodes: 156,
		totalCapacity: {
			cpu: 4892,
			gpu: 892,
			memory: 15680,
			storage: 2450000
		},
		utilization: {
			cpu: 67,
			gpu: 43,
			memory: 78,
			storage: 62
		},
		networkThroughput: 12.8,
		proxyConnections: 445
	});

	let searchTerm = $state('');
	let typeFilter = $state('all');
	let privacyFilter = $state('all');
	let healthFilter = $state('all');

	$: filteredAssets = assets.filter(asset => {
		const matchesSearch = searchTerm === '' || 
			asset.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
			asset.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
			asset.location.nodeId.toLowerCase().includes(searchTerm.toLowerCase());
		
		const matchesType = typeFilter === 'all' || asset.type === typeFilter;
		const matchesPrivacy = privacyFilter === 'all' || asset.privacy.level === privacyFilter;
		const matchesHealth = healthFilter === 'all' || asset.status.health === healthFilter;
		
		return matchesSearch && matchesType && matchesPrivacy && matchesHealth;
	});

	function addAsset() {
		// Navigate to asset creation form
		console.log('Add asset clicked');
	}

	function configureNode() {
		// Navigate to node configuration
		console.log('Configure node clicked');
	}
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<HardDrive class="h-8 w-8 text-hypermesh-600 mr-3" />
				HyperMesh Asset Network
			</h1>
			<p class="text-muted-foreground mt-2">
				Decentralized resource sharing with NAT-like proxy addressing
			</p>
		</div>
		<div class="flex space-x-3">
			<Button variant="outline" onclick={configureNode}>
				<Settings class="h-4 w-4 mr-2" />
				Configure Node
			</Button>
			<Button onclick={addAsset}>
				<Plus class="h-4 w-4 mr-2" />
				Add Asset
			</Button>
		</div>
	</div>

	<!-- Network Overview Metrics -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Total Assets"
			value={meshMetrics.totalAssets}
			change={3.2}
			changeLabel="this week"
			icon={HardDrive}
			color="text-hypermesh-600"
		/>
		<MetricsCard
			title="Active Nodes"
			value={meshMetrics.activeNodes}
			change={1.8}
			changeLabel="this week"
			icon={Network}
			color="text-blue-600"
		/>
		<MetricsCard
			title="Network Throughput"
			value={meshMetrics.networkThroughput}
			unit="Gbps"
			change={5.4}
			changeLabel="this hour"
			icon={Network}
			color="text-green-600"
		/>
		<MetricsCard
			title="Proxy Connections"
			value={meshMetrics.proxyConnections}
			icon={Network}
			color="text-purple-600"
		/>
	</div>

	<!-- Resource Utilization Overview -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">Resource Utilization</h3>
			<div class="space-y-4">
				<div>
					<div class="flex justify-between text-sm mb-2">
						<div class="flex items-center space-x-2">
							<Cpu class="h-4 w-4 text-blue-600" />
							<span>CPU Cores</span>
						</div>
						<span class="font-medium">{meshMetrics.utilization.cpu}%</span>
					</div>
					<Progress value={meshMetrics.utilization.cpu} class="h-2" />
				</div>
				<div>
					<div class="flex justify-between text-sm mb-2">
						<div class="flex items-center space-x-2">
							<Gpu class="h-4 w-4 text-green-600" />
							<span>GPU Memory</span>
						</div>
						<span class="font-medium">{meshMetrics.utilization.gpu}%</span>
					</div>
					<Progress value={meshMetrics.utilization.gpu} class="h-2" />
				</div>
				<div>
					<div class="flex justify-between text-sm mb-2">
						<div class="flex items-center space-x-2">
							<MemoryStick class="h-4 w-4 text-yellow-600" />
							<span>System Memory</span>
						</div>
						<span class="font-medium">{meshMetrics.utilization.memory}%</span>
					</div>
					<Progress value={meshMetrics.utilization.memory} class="h-2" />
				</div>
				<div>
					<div class="flex justify-between text-sm mb-2">
						<div class="flex items-center space-x-2">
							<HardDrive class="h-4 w-4 text-purple-600" />
							<span>Storage</span>
						</div>
						<span class="font-medium">{meshMetrics.utilization.storage}%</span>
					</div>
					<Progress value={meshMetrics.utilization.storage} class="h-2" />
				</div>
			</div>
		</Card>

		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">NAT-like Proxy System</h3>
			<div class="space-y-4">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Total Proxy Addresses:</span>
					<span class="font-medium">1,247</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Active Connections:</span>
					<span class="font-medium">445</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Trust-based Routing:</span>
					<Badge variant="default">Enabled</Badge>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Federated Resolution:</span>
					<Badge variant="default">Active</Badge>
				</div>
				<div class="pt-3 border-t">
					<div class="text-sm text-muted-foreground mb-2">Privacy Distribution:</div>
					<div class="space-y-1 text-xs">
						<div class="flex justify-between">
							<span>Private:</span>
							<span>34%</span>
						</div>
						<div class="flex justify-between">
							<span>P2P:</span>
							<span>28%</span>
						</div>
						<div class="flex justify-between">
							<span>Public Network:</span>
							<span>38%</span>
						</div>
					</div>
				</div>
			</div>
		</Card>
	</div>

	<!-- Search and Filters -->
	<Card class="p-6 mb-6">
		<div class="flex flex-col sm:flex-row gap-4">
			<div class="flex-1">
				<div class="relative">
					<Search class="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
					<input
						type="text"
						placeholder="Search assets by name, description, or node..."
						class="w-full pl-10 pr-4 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						bind:value={searchTerm}
					/>
				</div>
			</div>
			<div class="flex items-center space-x-2">
				<Filter class="h-4 w-4 text-muted-foreground" />
				<select
					class="px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					bind:value={typeFilter}
				>
					<option value="all">All Types</option>
					<option value="cpu">CPU</option>
					<option value="gpu">GPU</option>
					<option value="memory">Memory</option>
					<option value="storage">Storage</option>
					<option value="network">Network</option>
				</select>
				<select
					class="px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					bind:value={privacyFilter}
				>
					<option value="all">All Privacy</option>
					<option value="private">Private</option>
					<option value="p2p">P2P</option>
					<option value="public-network">Public Network</option>
					<option value="full-public">Full Public</option>
				</select>
				<select
					class="px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					bind:value={healthFilter}
				>
					<option value="all">All Health</option>
					<option value="healthy">Healthy</option>
					<option value="warning">Warning</option>
					<option value="critical">Critical</option>
					<option value="offline">Offline</option>
				</select>
			</div>
		</div>
	</Card>

	<!-- Assets Grid -->
	<div class="space-y-6">
		<div class="flex items-center justify-between">
			<h2 class="text-2xl font-semibold">
				Assets ({filteredAssets.length})
			</h2>
			{#if searchTerm || typeFilter !== 'all' || privacyFilter !== 'all' || healthFilter !== 'all'}
				<Button 
					variant="ghost" 
					size="sm" 
					onclick={() => { 
						searchTerm = ''; 
						typeFilter = 'all'; 
						privacyFilter = 'all'; 
						healthFilter = 'all'; 
					}}
				>
					Clear Filters
				</Button>
			{/if}
		</div>

		{#if filteredAssets.length === 0}
			<Card class="p-12 text-center">
				<HardDrive class="h-12 w-12 text-muted-foreground mx-auto mb-4" />
				<h3 class="text-lg font-medium mb-2">No assets found</h3>
				<p class="text-muted-foreground">
					{searchTerm || typeFilter !== 'all' || privacyFilter !== 'all' || healthFilter !== 'all'
						? 'Try adjusting your search criteria.' 
						: 'Add your first asset to get started.'}
				</p>
			</Card>
		{:else}
			<div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
				{#each filteredAssets as asset}
					<AssetCard {asset} showAllocation={true} />
				{/each}
			</div>
		{/if}
	</div>
</Layout>
<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Progress from '$lib/components/ui/progress.svelte';
	import { Shield, Clock, Zap, Database, Activity, CheckCircle } from 'lucide-svelte';
	import type { ConsensusBlock, ConsensusMetrics, ConsensusProof } from '$lib/types.js';

	// Mock data
	let consensusMetrics: ConsensusMetrics = $state({
		blockHeight: 15234,
		blockTime: 2.3, // seconds
		validators: 67,
		finalityTime: 4.8, // seconds
		tps: 847, // transactions per second
		proofCoverage: {
			space: 98.5,
			stake: 96.2,
			work: 99.1,
			time: 97.8
		}
	});

	let recentBlocks: ConsensusBlock[] = $state([
		{
			height: 15234,
			hash: 'a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890',
			previousHash: 'b2c3d4e5f6789012345678901234567890123456789012345678901234567890a1',
			timestamp: new Date(Date.now() - 1000).toISOString(),
			transactions: 247,
			proofs: [
				{ type: 'space', status: 'valid', data: 'proof_data_space', timestamp: new Date().toISOString(), validatedBy: ['validator-001'] },
				{ type: 'stake', status: 'valid', data: 'proof_data_stake', timestamp: new Date().toISOString(), validatedBy: ['validator-002'] },
				{ type: 'work', status: 'valid', data: 'proof_data_work', timestamp: new Date().toISOString(), validatedBy: ['validator-003'] },
				{ type: 'time', status: 'valid', data: 'proof_data_time', timestamp: new Date().toISOString(), validatedBy: ['validator-004'] }
			],
			validator: 'validator-001',
			size: 1024576 // 1MB
		},
		{
			height: 15233,
			hash: 'b2c3d4e5f6789012345678901234567890123456789012345678901234567890a1',
			previousHash: 'c3d4e5f6789012345678901234567890123456789012345678901234567890a1b2',
			timestamp: new Date(Date.now() - 3300).toISOString(),
			transactions: 189,
			proofs: [
				{ type: 'space', status: 'valid', data: 'proof_data_space', timestamp: new Date().toISOString(), validatedBy: ['validator-002'] },
				{ type: 'stake', status: 'valid', data: 'proof_data_stake', timestamp: new Date().toISOString(), validatedBy: ['validator-003'] },
				{ type: 'work', status: 'valid', data: 'proof_data_work', timestamp: new Date().toISOString(), validatedBy: ['validator-004'] },
				{ type: 'time', status: 'valid', data: 'proof_data_time', timestamp: new Date().toISOString(), validatedBy: ['validator-001'] }
			],
			validator: 'validator-002',
			size: 876432
		}
	]);

	let validators = $state([
		{
			id: 'validator-001',
			stake: 15000,
			uptime: 99.7,
			blocksProduced: 892,
			status: 'active'
		},
		{
			id: 'validator-002',
			stake: 12500,
			uptime: 98.9,
			blocksProduced: 734,
			status: 'active'
		},
		{
			id: 'validator-003',
			stake: 18000,
			uptime: 99.9,
			blocksProduced: 1023,
			status: 'active'
		}
	]);

	let proofTypes = {
		space: {
			name: 'Proof of Space (PoSp)',
			description: 'WHERE - Storage location and physical/network location validation',
			color: 'text-blue-600',
			bgColor: 'bg-blue-50',
			coverage: consensusMetrics.proofCoverage.space
		},
		stake: {
			name: 'Proof of Stake (PoSt)',
			description: 'WHO - Ownership, access rights, and economic stake validation',
			color: 'text-green-600',
			bgColor: 'bg-green-50',
			coverage: consensusMetrics.proofCoverage.stake
		},
		work: {
			name: 'Proof of Work (PoWk)',
			description: 'WHAT/HOW - Computational resources and processing validation',
			color: 'text-purple-600',
			bgColor: 'bg-purple-50',
			coverage: consensusMetrics.proofCoverage.work
		},
		time: {
			name: 'Proof of Time (PoTm)',
			description: 'WHEN - Temporal ordering and timestamp validation',
			color: 'text-yellow-600',
			bgColor: 'bg-yellow-50',
			coverage: consensusMetrics.proofCoverage.time
		}
	};

	let loading = $state(false);
	let lastRefresh = $state(new Date());

	onMount(() => {
		// Set up auto-refresh every 5 seconds for real-time consensus data
		const interval = setInterval(() => {
			refreshConsensusData();
		}, 5000);

		return () => clearInterval(interval);
	});

	function refreshConsensusData() {
		lastRefresh = new Date();
		
		// Simulate new block
		if (Math.random() > 0.7) { // 30% chance of new block
			const newBlock: ConsensusBlock = {
				height: consensusMetrics.blockHeight + 1,
				hash: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join(''),
				previousHash: recentBlocks[0].hash,
				timestamp: new Date().toISOString(),
				transactions: Math.floor(Math.random() * 300) + 50,
				proofs: [
					{ type: 'space', status: 'valid', data: 'proof_data_space', timestamp: new Date().toISOString(), validatedBy: [`validator-${String(Math.floor(Math.random() * 3) + 1).padStart(3, '0')}`] },
					{ type: 'stake', status: 'valid', data: 'proof_data_stake', timestamp: new Date().toISOString(), validatedBy: [`validator-${String(Math.floor(Math.random() * 3) + 1).padStart(3, '0')}`] },
					{ type: 'work', status: 'valid', data: 'proof_data_work', timestamp: new Date().toISOString(), validatedBy: [`validator-${String(Math.floor(Math.random() * 3) + 1).padStart(3, '0')}`] },
					{ type: 'time', status: 'valid', data: 'proof_data_time', timestamp: new Date().toISOString(), validatedBy: [`validator-${String(Math.floor(Math.random() * 3) + 1).padStart(3, '0')}`] }
				],
				validator: `validator-${String(Math.floor(Math.random() * 3) + 1).padStart(3, '0')}`,
				size: Math.floor(Math.random() * 500000) + 500000
			};
			
			recentBlocks = [newBlock, ...recentBlocks.slice(0, 9)];
			consensusMetrics.blockHeight++;
		}
		
		// Update TPS and other metrics with slight variations
		consensusMetrics.tps += Math.floor((Math.random() - 0.5) * 50);
		consensusMetrics.tps = Math.max(700, Math.min(1000, consensusMetrics.tps));
	}

	function validateConsensus() {
		loading = true;
		setTimeout(() => {
			loading = false;
			// Simulate validation result
			console.log('Consensus validation completed');
		}, 2000);
	}

	$: averageProofCoverage = Object.values(consensusMetrics.proofCoverage).reduce((a, b) => a + b, 0) / 4;
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<Shield class="h-8 w-8 text-quantum-600 mr-3" />
				Four-Proof Consensus System
			</h1>
			<p class="text-muted-foreground mt-2">
				NKrypt protocol with unified WHERE/WHO/WHAT/WHEN validation
			</p>
		</div>
		<div class="flex items-center space-x-4">
			<div class="text-sm text-muted-foreground">
				Last updated: {lastRefresh.toLocaleTimeString()}
			</div>
			<Button onclick={validateConsensus} disabled={loading}>
				<CheckCircle class="h-4 w-4 mr-2" />
				{loading ? 'Validating...' : 'Validate'}
			</Button>
		</div>
	</div>

	<!-- Consensus Metrics -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Block Height"
			value={consensusMetrics.blockHeight}
			change={0.8}
			changeLabel="blocks/min"
			icon={Database}
			color="text-blue-600"
		/>
		<MetricsCard
			title="Block Time"
			value={consensusMetrics.blockTime}
			unit="seconds"
			change={-12.3}
			changeLabel="optimization"
			icon={Clock}
			color="text-green-600"
		/>
		<MetricsCard
			title="Transactions/sec"
			value={consensusMetrics.tps}
			change={5.2}
			changeLabel="this hour"
			icon={Zap}
			color="text-purple-600"
		/>
		<MetricsCard
			title="Active Validators"
			value={consensusMetrics.validators}
			change={1.5}
			changeLabel="this week"
			icon={Shield}
			color="text-quantum-600"
		/>
	</div>

	<!-- Four-Proof System Overview -->
	<Card class="p-6 mb-8 bg-gradient-to-r from-quantum-50 to-purple-50 border-quantum-200">
		<div class="flex items-center justify-between mb-6">
			<div>
				<h3 class="font-semibold text-lg">NKrypt Four-Proof System</h3>
				<p class="text-muted-foreground">Every asset requires ALL FOUR proofs for consensus validation</p>
			</div>
			<div class="text-right">
				<div class="text-2xl font-bold text-quantum-600">{averageProofCoverage.toFixed(1)}%</div>
				<div class="text-sm text-muted-foreground">Average Coverage</div>
			</div>
		</div>
		
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
			{#each Object.entries(proofTypes) as [key, proof]}
				<div class={`p-4 rounded-lg border-2 ${proof.bgColor}`}>
					<div class="flex items-center justify-between mb-2">
						<Badge class={`${proof.color} bg-white`}>
							{key.toUpperCase()}
						</Badge>
						<span class={`font-bold ${proof.color}`}>
							{proof.coverage.toFixed(1)}%
						</span>
					</div>
					<h4 class="font-medium text-sm mb-1">{proof.name}</h4>
					<p class="text-xs text-muted-foreground">{proof.description}</p>
					<div class="mt-2">
						<Progress value={proof.coverage} class="h-1" />
					</div>
				</div>
			{/each}
		</div>
	</Card>

	<!-- Consensus Performance -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
		<!-- Finality and Performance -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">Consensus Performance</h3>
			<div class="space-y-4">
				<div class="flex justify-between">
					<span class="text-muted-foreground">Finality Time:</span>
					<span class="font-medium">{consensusMetrics.finalityTime}s</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Block Production Rate:</span>
					<span class="font-medium">{(60 / consensusMetrics.blockTime).toFixed(1)} blocks/min</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Average Block Size:</span>
					<span class="font-medium">
						{(recentBlocks.reduce((sum, block) => sum + block.size, 0) / recentBlocks.length / 1024).toFixed(0)} KB
					</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Network Participation:</span>
					<span class="font-medium">{((validators.filter(v => v.status === 'active').length / validators.length) * 100).toFixed(1)}%</span>
				</div>
				
				<div class="pt-4 border-t">
					<div class="text-sm font-medium mb-2">Consensus Health</div>
					<div class="space-y-2">
						<div class="flex justify-between text-xs">
							<span>Proof Validation Rate:</span>
							<span class="text-green-600">{averageProofCoverage.toFixed(1)}%</span>
						</div>
						<div class="flex justify-between text-xs">
							<span>Byzantine Fault Tolerance:</span>
							<span class="text-green-600">33% threshold</span>
						</div>
						<div class="flex justify-between text-xs">
							<span>Chain Integrity:</span>
							<span class="text-green-600">100% verified</span>
						</div>
					</div>
				</div>
			</div>
		</Card>

		<!-- Validator Status -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">Top Validators</h3>
			<div class="space-y-4">
				{#each validators.slice(0, 5) as validator}
					<div class="flex items-center justify-between p-3 border rounded-lg">
						<div class="flex items-center space-x-3">
							<div class={`w-3 h-3 rounded-full ${
								validator.status === 'active' ? 'bg-green-500' : 'bg-gray-500'
							}`}></div>
							<div>
								<code class="text-sm font-mono">{validator.id}</code>
								<div class="text-xs text-muted-foreground">
									{validator.blocksProduced} blocks produced
								</div>
							</div>
						</div>
						<div class="text-right">
							<div class="font-medium">{validator.stake.toLocaleString()} CAESAR</div>
							<div class="text-xs text-muted-foreground">
								{validator.uptime}% uptime
							</div>
						</div>
					</div>
				{/each}
			</div>
			
			<div class="mt-4 pt-4 border-t">
				<div class="text-sm text-muted-foreground">
					Total Staked: {validators.reduce((sum, v) => sum + v.stake, 0).toLocaleString()} CAESAR
				</div>
			</div>
		</Card>
	</div>

	<!-- Recent Blocks -->
	<Card class="p-6">
		<div class="flex items-center justify-between mb-6">
			<h3 class="font-semibold text-lg">Recent Blocks</h3>
			<Badge variant="outline">Latest {recentBlocks.length} blocks</Badge>
		</div>
		
		<div class="space-y-4">
			{#each recentBlocks as block}
				<div class="border rounded-lg p-4">
					<div class="flex items-center justify-between mb-3">
						<div class="flex items-center space-x-3">
							<Badge variant="outline">#{block.height}</Badge>
							<div>
								<code class="text-xs font-mono">{block.hash.substring(0, 16)}...</code>
								<div class="text-xs text-muted-foreground">
									{new Date(block.timestamp).toLocaleTimeString()} • {block.transactions} transactions
								</div>
							</div>
						</div>
						<div class="flex items-center space-x-2">
							<Badge variant="secondary" class="text-xs">
								{(block.size / 1024).toFixed(0)} KB
							</Badge>
							<code class="text-xs text-muted-foreground">{block.validator}</code>
						</div>
					</div>
					
					<!-- Four Proofs Status -->
					<div class="grid grid-cols-2 md:grid-cols-4 gap-2">
						{#each block.proofs as proof}
							<div class="flex items-center space-x-2 text-xs">
								<div class={`w-2 h-2 rounded-full ${
									proof.status === 'valid' ? 'bg-green-500' :
									proof.status === 'pending' ? 'bg-yellow-500' :
									'bg-red-500'
								}`}></div>
								<span class="font-medium">{proof.type.toUpperCase()}</span>
								<span class={`${
									proof.status === 'valid' ? 'text-green-600' :
									proof.status === 'pending' ? 'text-yellow-600' :
									'text-red-600'
								}`}>
									{proof.status}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
		
		{#if recentBlocks.length === 0}
			<div class="text-center py-8">
				<Database class="h-12 w-12 text-muted-foreground mx-auto mb-4" />
				<h4 class="text-lg font-medium mb-2">No blocks yet</h4>
				<p class="text-muted-foreground">Waiting for consensus validation</p>
			</div>
		{/if}
	</Card>
</Layout>
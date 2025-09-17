<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import Progress from '$lib/components/ui/progress.svelte';
	import { Coins, TrendingUp, Wallet, Users, Gift, Send, Download } from 'lucide-svelte';
	import type { EconomicReward, UserBalance, EconomicMetrics } from '$lib/types.js';

	// Mock data
	let userBalance: UserBalance = $state({
		total: 1247.89,
		available: 1089.23,
		locked: 158.66,
		pending: 45.32,
		currency: 'CAESAR'
	});

	let economicMetrics: EconomicMetrics = $state({
		totalRewards: 2847291.45,
		activeAssets: 1247,
		averageRewardRate: 0.125, // 12.5% APY
		stakingParticipation: 67.8, // 67.8% of supply staked
		networkValue: 2400000 // $2.4M USD
	});

	let recentRewards: EconomicReward[] = $state([
		{
			id: 'reward-001',
			assetId: 'asset-cpu-001',
			amount: 12.45,
			currency: 'CAESAR',
			earnedAt: '2024-09-17T10:30:00Z',
			description: 'CPU sharing reward - Intel Xeon E5-2690',
			confirmed: true
		},
		{
			id: 'reward-002',
			assetId: 'asset-gpu-001',
			amount: 34.67,
			currency: 'CAESAR',
			earnedAt: '2024-09-17T09:15:00Z',
			description: 'GPU compute reward - NVIDIA RTX 4090',
			confirmed: true
		},
		{
			id: 'reward-003',
			assetId: 'asset-storage-001',
			amount: 8.92,
			currency: 'CAESAR',
			earnedAt: '2024-09-17T08:45:00Z',
			description: 'Storage hosting reward - Samsung 990 PRO NVMe',
			confirmed: false
		},
		{
			id: 'reward-004',
			assetId: 'asset-network-001',
			amount: 15.78,
			currency: 'CAESAR',
			earnedAt: '2024-09-17T07:20:00Z',
			description: 'Network bandwidth reward - Gigabit connection',
			confirmed: true
		}
	]);

	let stakingInfo = $state({
		stakedAmount: 850.00,
		stakingReward: 106.25,
		stakingPeriod: 180, // days
		annualReward: 0.125, // 12.5% APY
		nextRewardDate: '2024-09-24T00:00:00Z'
	});

	let totalEarnings = $state(0);
	let weeklyTrend = $state(0);

	onMount(() => {
		// Calculate total earnings from recent rewards
		totalEarnings = recentRewards
			.filter(r => r.confirmed)
			.reduce((sum, reward) => sum + reward.amount, 0);
		
		// Simulate weekly trend
		weeklyTrend = 15.6;
	});

	function claimRewards() {
		// Simulate claiming pending rewards
		const pendingRewards = recentRewards.filter(r => !r.confirmed);
		const claimAmount = pendingRewards.reduce((sum, r) => sum + r.amount, 0);
		
		if (claimAmount > 0) {
			// Mark as confirmed
			recentRewards = recentRewards.map(r => ({ ...r, confirmed: true }));
			
			// Update balance
			userBalance.available += claimAmount;
			userBalance.pending -= claimAmount;
			userBalance.total += claimAmount;
		}
	}

	function stakeTokens() {
		console.log('Stake tokens dialog');
	}

	function withdrawTokens() {
		console.log('Withdraw tokens dialog');
	}

	function sendTokens() {
		console.log('Send tokens dialog');
	}

	function exportRewards() {
		const data = JSON.stringify(recentRewards, null, 2);
		const blob = new Blob([data], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'caesar-rewards.json';
		a.click();
		URL.revokeObjectURL(url);
	}

	$: pendingRewardsCount = recentRewards.filter(r => !r.confirmed).length;
	$: pendingRewardsAmount = recentRewards
		.filter(r => !r.confirmed)
		.reduce((sum, r) => sum + r.amount, 0);
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<Coins class="h-8 w-8 text-caesar-600 mr-3" />
				Caesar Economics Platform
			</h1>
			<p class="text-muted-foreground mt-2">
				Decentralized economic incentives for resource sharing
			</p>
		</div>
		<div class="flex space-x-3">
			<Button variant="outline" onclick={exportRewards}>
				<Download class="h-4 w-4 mr-2" />
				Export
			</Button>
			<Button variant="outline" onclick={sendTokens}>
				<Send class="h-4 w-4 mr-2" />
				Send
			</Button>
			<Button onclick={claimRewards} disabled={pendingRewardsCount === 0}>
				<Gift class="h-4 w-4 mr-2" />
				Claim {pendingRewardsCount > 0 ? `(${pendingRewardsCount})` : 'Rewards'}
			</Button>
		</div>
	</div>

	<!-- Wallet Overview -->
	<Card class="p-6 mb-8 bg-gradient-to-r from-caesar-50 to-yellow-50 border-caesar-200">
		<div class="flex items-center justify-between">
			<div class="flex items-center space-x-4">
				<div class="p-3 bg-caesar-100 rounded-lg">
					<Wallet class="h-6 w-6 text-caesar-600" />
				</div>
				<div>
					<h3 class="font-semibold text-lg">Wallet Balance</h3>
					<div class="flex items-baseline space-x-2">
						<span class="text-3xl font-bold text-caesar-700">{userBalance.total.toFixed(2)}</span>
						<span class="text-lg text-muted-foreground">CAESAR</span>
					</div>
				</div>
			</div>
			<div class="text-right space-y-1">
				<div class="text-sm text-muted-foreground">Available: {userBalance.available.toFixed(2)} CAESAR</div>
				<div class="text-sm text-muted-foreground">Staked: {userBalance.locked.toFixed(2)} CAESAR</div>
				<div class="text-sm text-muted-foreground">Pending: {userBalance.pending.toFixed(2)} CAESAR</div>
			</div>
		</div>
	</Card>

	<!-- Economic Metrics -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Total Network Rewards"
			value={economicMetrics.totalRewards.toFixed(0)}
			unit="CAESAR"
			change={8.7}
			changeLabel="this month"
			icon={Coins}
			color="text-caesar-600"
		/>
		<MetricsCard
			title="Active Assets"
			value={economicMetrics.activeAssets}
			change={3.2}
			changeLabel="this week"
			icon={TrendingUp}
			color="text-green-600"
		/>
		<MetricsCard
			title="Staking Participation"
			value={economicMetrics.stakingParticipation.toFixed(1)}
			unit="%"
			change={2.1}
			changeLabel="this month"
			icon={Users}
			color="text-blue-600"
		/>
		<MetricsCard
			title="Network Value"
			value={`$${(economicMetrics.networkValue / 1000000).toFixed(1)}M`}
			change={12.5}
			changeLabel="this quarter"
			icon={TrendingUp}
			color="text-purple-600"
		/>
	</div>

	<!-- Staking & Rewards Overview -->
	<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
		<!-- Staking Information -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">Staking Overview</h3>
			<div class="space-y-4">
				<div class="flex justify-between items-center">
					<span class="text-muted-foreground">Staked Amount</span>
					<span class="font-bold text-lg">{stakingInfo.stakedAmount.toFixed(2)} CAESAR</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Annual Reward Rate</span>
					<span class="font-medium text-green-600">{(stakingInfo.annualReward * 100).toFixed(1)}% APY</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Earned Rewards</span>
					<span class="font-medium">{stakingInfo.stakingReward.toFixed(2)} CAESAR</span>
				</div>
				<div class="flex justify-between">
					<span class="text-muted-foreground">Next Reward</span>
					<span class="font-medium">{new Date(stakingInfo.nextRewardDate).toLocaleDateString()}</span>
				</div>
				
				<div class="pt-4 border-t">
					<div class="flex justify-between text-sm mb-2">
						<span>Staking Progress</span>
						<span>{stakingInfo.stakingPeriod} days</span>
					</div>
					<Progress value={75} class="h-2" />
					<div class="text-xs text-muted-foreground mt-1">
						135 days remaining until unlock
					</div>
				</div>
				
				<div class="flex space-x-2 pt-4">
					<Button variant="outline" size="sm" onclick={stakeTokens}>
						Stake More
					</Button>
					<Button variant="outline" size="sm" onclick={withdrawTokens}>
						Withdraw
					</Button>
				</div>
			</div>
		</Card>

		<!-- Reward Distribution -->
		<Card class="p-6">
			<h3 class="font-semibold text-lg mb-4">Reward Distribution</h3>
			<div class="space-y-4">
				<div class="text-center">
					<div class="text-2xl font-bold text-caesar-600">{totalEarnings.toFixed(2)} CAESAR</div>
					<div class="text-sm text-muted-foreground">Total Today</div>
				</div>
				
				<div class="grid grid-cols-2 gap-4 text-sm">
					<div class="text-center p-3 bg-muted rounded-lg">
						<div class="font-medium">CPU Sharing</div>
						<div class="text-xs text-muted-foreground">12.45 CAESAR</div>
					</div>
					<div class="text-center p-3 bg-muted rounded-lg">
						<div class="font-medium">GPU Compute</div>
						<div class="text-xs text-muted-foreground">34.67 CAESAR</div>
					</div>
					<div class="text-center p-3 bg-muted rounded-lg">
						<div class="font-medium">Storage</div>
						<div class="text-xs text-muted-foreground">8.92 CAESAR</div>
					</div>
					<div class="text-center p-3 bg-muted rounded-lg">
						<div class="font-medium">Network</div>
						<div class="text-xs text-muted-foreground">15.78 CAESAR</div>
					</div>
				</div>

				<div class="pt-4 border-t">
					<div class="flex items-center justify-between text-sm mb-2">
						<span>Weekly Trend</span>
						<div class="flex items-center space-x-1 text-green-600">
							<TrendingUp class="h-3 w-3" />
							<span>+{weeklyTrend}%</span>
						</div>
					</div>
					<div class="text-xs text-muted-foreground">
						Earnings increased by {weeklyTrend}% compared to last week
					</div>
				</div>
			</div>
		</Card>
	</div>

	<!-- Recent Rewards -->
	<Card class="p-6 mb-8">
		<div class="flex items-center justify-between mb-6">
			<h3 class="font-semibold text-lg">Recent Rewards</h3>
			{#if pendingRewardsCount > 0}
				<Badge variant="secondary">
					{pendingRewardsAmount.toFixed(2)} CAESAR pending
				</Badge>
			{/if}
		</div>
		
		<div class="space-y-4">
			{#each recentRewards as reward}
				<div class="flex items-center justify-between p-4 border rounded-lg">
					<div class="flex items-center space-x-3">
						<div class={`w-3 h-3 rounded-full ${reward.confirmed ? 'bg-green-500' : 'bg-yellow-500'}`}></div>
						<div>
							<div class="font-medium">{reward.description}</div>
							<div class="text-sm text-muted-foreground">
								{new Date(reward.earnedAt).toLocaleString()}
							</div>
						</div>
					</div>
					<div class="text-right">
						<div class="font-bold text-caesar-600">+{reward.amount.toFixed(2)} CAESAR</div>
						<div class="text-xs text-muted-foreground">
							{reward.confirmed ? 'Confirmed' : 'Pending'}
						</div>
					</div>
				</div>
			{/each}
		</div>
		
		{#if recentRewards.length === 0}
			<div class="text-center py-8">
				<Coins class="h-12 w-12 text-muted-foreground mx-auto mb-4" />
				<h4 class="text-lg font-medium mb-2">No rewards yet</h4>
				<p class="text-muted-foreground">Share your assets to start earning CAESAR tokens</p>
			</div>
		{/if}
	</Card>

	<!-- Economic Model Information -->
	<Card class="p-6">
		<h3 class="font-semibold text-lg mb-4">Economic Model</h3>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="space-y-2">
				<h4 class="font-medium">Reward Calculation</h4>
				<div class="text-sm text-muted-foreground space-y-1">
					<div>• Based on asset utilization</div>
					<div>• Performance multipliers</div>
					<div>• Network contribution score</div>
					<div>• Staking bonus (up to 25%)</div>
				</div>
			</div>
			<div class="space-y-2">
				<h4 class="font-medium">Token Distribution</h4>
				<div class="text-sm text-muted-foreground space-y-1">
					<div>• 60% - Resource providers</div>
					<div>• 20% - Network validators</div>
					<div>• 15% - Development fund</div>
					<div>• 5% - Community treasury</div>
				</div>
			</div>
			<div class="space-y-2">
				<h4 class="font-medium">Privacy Incentives</h4>
				<div class="text-sm text-muted-foreground space-y-1">
					<div>• Private: Base rate</div>
					<div>• P2P: +10% bonus</div>
					<div>• Public Network: +20% bonus</div>
					<div>• Full Public: +50% bonus</div>
				</div>
			</div>
		</div>
	</Card>
</Layout>
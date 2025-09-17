<script lang="ts">
	import { cn, formatBytes, getStatusColor } from '$lib/utils.js';
	import type { Asset } from '$lib/types.js';
	import Card from '../ui/card.svelte';
	import Badge from '../ui/badge.svelte';
	import Progress from '../ui/progress.svelte';
	import { Cpu, HardDrive, MemoryStick, Gpu, Network, Server } from 'lucide-svelte';

	interface $$Props {
		asset: Asset;
		showAllocation?: boolean;
		class?: string;
	}

	let { asset, showAllocation = true, class: className }: $$Props = $props();

	$: typeIcon = asset.type === 'cpu' ? Cpu
		: asset.type === 'gpu' ? Gpu
		: asset.type === 'memory' ? MemoryStick
		: asset.type === 'storage' ? HardDrive
		: asset.type === 'network' ? Network
		: Server;

	$: utilizationPercentage = (asset.allocation.allocated / (asset.allocation.allocated + asset.allocation.available)) * 100;

	$: privacyColor = asset.privacy.level === 'private' ? 'bg-red-100 text-red-800'
		: asset.privacy.level === 'full-public' ? 'bg-green-100 text-green-800'
		: 'bg-yellow-100 text-yellow-800';
</script>

<Card class={cn('p-6', className)}>
	<div class="flex items-start justify-between">
		<div class="flex items-center space-x-3">
			<div class="p-2 rounded-lg bg-primary/10">
				<svelte:component this={typeIcon} class="h-5 w-5 text-primary" />
			</div>
			<div>
				<h3 class="font-semibold">{asset.name}</h3>
				<p class="text-sm text-muted-foreground">{asset.description}</p>
			</div>
		</div>
		<div class="flex flex-col items-end space-y-2">
			<Badge variant="outline" class="capitalize">{asset.type}</Badge>
			<div class={cn('px-2 py-1 rounded text-xs font-medium', privacyColor)}>
				{asset.privacy.level.replace('-', ' ')}
			</div>
		</div>
	</div>

	<div class="mt-4 space-y-3">
		<!-- Health Status -->
		<div class="flex items-center justify-between">
			<span class="text-sm text-muted-foreground">Health:</span>
			<span class={cn('text-sm font-medium', getStatusColor(asset.status.health))}>
				{asset.status.health}
			</span>
		</div>

		<!-- Utilization -->
		<div class="flex items-center justify-between">
			<span class="text-sm text-muted-foreground">Utilization:</span>
			<span class="text-sm font-medium">{asset.status.utilization.toFixed(1)}%</span>
		</div>

		{#if showAllocation}
			<!-- Allocation Progress -->
			<div class="space-y-2">
				<div class="flex justify-between text-sm">
					<span class="text-muted-foreground">Allocation:</span>
					<span class="font-medium">
						{asset.allocation.allocated} / {asset.allocation.allocated + asset.allocation.available} {asset.allocation.unit}
					</span>
				</div>
				<Progress value={utilizationPercentage} class="h-2" />
			</div>
		{/if}

		<!-- Location -->
		<div class="flex items-center justify-between">
			<span class="text-sm text-muted-foreground">Location:</span>
			<span class="text-sm font-medium">{asset.location.region}</span>
		</div>

		<!-- Consensus Requirements -->
		{#if asset.privacy.requireConsensus}
			<div class="pt-2 border-t">
				<span class="text-xs font-medium text-muted-foreground">Required Proofs:</span>
				<div class="flex space-x-1 mt-1">
					{#if asset.privacy.proofs.space}
						<Badge variant="secondary" class="text-xs">PoSp</Badge>
					{/if}
					{#if asset.privacy.proofs.stake}
						<Badge variant="secondary" class="text-xs">PoSt</Badge>
					{/if}
					{#if asset.privacy.proofs.work}
						<Badge variant="secondary" class="text-xs">PoWk</Badge>
					{/if}
					{#if asset.privacy.proofs.time}
						<Badge variant="secondary" class="text-xs">PoTm</Badge>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</Card>
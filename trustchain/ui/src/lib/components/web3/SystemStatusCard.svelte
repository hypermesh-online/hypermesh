<script lang="ts">
	import { cn, getStatusColor, formatDuration } from '$lib/utils.js';
	import type { SystemStatus } from '$lib/types.js';
	import Card from '../ui/card.svelte';
	import Badge from '../ui/badge.svelte';
	import { Activity, AlertTriangle, CheckCircle, XCircle } from 'lucide-svelte';

	interface $$Props {
		system: SystemStatus;
		showDetails?: boolean;
		class?: string;
	}

	let { system, showDetails = false, class: className }: $$Props = $props();

	$: statusIcon = system.status === 'online' ? CheckCircle 
		: system.status === 'degraded' ? AlertTriangle 
		: system.status === 'offline' ? XCircle 
		: Activity;

	$: statusVariant = system.status === 'online' ? 'default'
		: system.status === 'degraded' ? 'secondary'
		: system.status === 'offline' ? 'destructive'
		: 'outline';
</script>

<Card class={cn('p-6', className)}>
	<div class="flex items-center justify-between">
		<div class="flex items-center space-x-3">
			<svelte:component this={statusIcon} class={cn('h-5 w-5', getStatusColor(system.status))} />
			<div>
				<h3 class="font-semibold text-lg">{system.name}</h3>
				{#if showDetails && system.lastChecked}
					<p class="text-sm text-muted-foreground">
						Last checked: {new Date(system.lastChecked).toLocaleTimeString()}
					</p>
				{/if}
			</div>
		</div>
		<Badge variant={statusVariant} class="capitalize">
			{system.status}
		</Badge>
	</div>

	{#if showDetails}
		<div class="mt-4 space-y-2">
			<div class="flex justify-between text-sm">
				<span class="text-muted-foreground">Uptime:</span>
				<span class="font-medium">{formatDuration(system.uptime)}</span>
			</div>

			{#if system.metrics}
				{#each Object.entries(system.metrics) as [key, value]}
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground capitalize">{key.replace(/([A-Z])/g, ' $1')}:</span>
						<span class="font-medium">{value}</span>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</Card>
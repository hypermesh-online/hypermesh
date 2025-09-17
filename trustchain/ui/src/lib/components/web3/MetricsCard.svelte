<script lang="ts">
	import { cn, formatNumber } from '$lib/utils.js';
	import Card from '../ui/card.svelte';
	import { TrendingUp, TrendingDown, Minus } from 'lucide-svelte';

	interface $$Props {
		title: string;
		value: string | number;
		unit?: string;
		change?: number;
		changeLabel?: string;
		icon?: any;
		color?: string;
		class?: string;
	}

	let { 
		title, 
		value, 
		unit = '', 
		change, 
		changeLabel,
		icon,
		color = 'text-primary',
		class: className 
	}: $$Props = $props();

	$: formattedValue = typeof value === 'number' ? formatNumber(value) : value;
	$: trendIcon = change && change > 0 ? TrendingUp 
		: change && change < 0 ? TrendingDown 
		: Minus;
	$: trendColor = change && change > 0 ? 'text-green-500' 
		: change && change < 0 ? 'text-red-500' 
		: 'text-muted-foreground';
</script>

<Card class={cn('p-6', className)}>
	<div class="flex items-center justify-between">
		<div class="space-y-2">
			<p class="text-sm font-medium text-muted-foreground">{title}</p>
			<div class="flex items-baseline space-x-1">
				<span class="text-2xl font-bold">{formattedValue}</span>
				{#if unit}
					<span class="text-sm text-muted-foreground">{unit}</span>
				{/if}
			</div>
			{#if change !== undefined}
				<div class="flex items-center space-x-1 text-xs">
					<svelte:component this={trendIcon} class={cn('h-3 w-3', trendColor)} />
					<span class={trendColor}>
						{change > 0 ? '+' : ''}{change.toFixed(1)}%
					</span>
					{#if changeLabel}
						<span class="text-muted-foreground">{changeLabel}</span>
					{/if}
				</div>
			{/if}
		</div>
		{#if icon}
			<div class="p-3 rounded-lg bg-primary/10">
				<svelte:component this={icon} class={cn('h-6 w-6', color)} />
			</div>
		{/if}
	</div>
</Card>
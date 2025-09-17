<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	interface $$Props extends HTMLAttributes<HTMLDivElement> {
		value?: number;
		max?: number;
		class?: string;
	}

	let { value = 0, max = 100, class: className, ...restProps }: $$Props = $props();

	$: percentage = Math.min(Math.max((value / max) * 100, 0), 100);
</script>

<div
	class={cn('relative h-4 w-full overflow-hidden rounded-full bg-secondary', className)}
	{...restProps}
>
	<div
		class="h-full w-full flex-1 bg-primary transition-all duration-300 ease-in-out"
		style="transform: translateX(-{100 - percentage}%)"
	/>
</div>
<script lang="ts">
	import { onMount } from 'svelte';
	import Navigation from './web3/Navigation.svelte';
	import { cn } from '$lib/utils.js';

	interface $$Props {
		class?: string;
	}

	let { class: className }: $$Props = $props();
	let darkMode = $state(false);
	let sidebarCollapsed = $state(false);

	onMount(() => {
		// Check for saved theme preference or default to system preference
		const savedTheme = localStorage.getItem('theme');
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		
		darkMode = savedTheme === 'dark' || (!savedTheme && prefersDark);
		
		// Apply theme to document
		updateTheme();
	});

	function updateTheme() {
		if (darkMode) {
			document.documentElement.classList.add('dark');
			localStorage.setItem('theme', 'dark');
		} else {
			document.documentElement.classList.remove('dark');
			localStorage.setItem('theme', 'light');
		}
	}

	function toggleTheme() {
		darkMode = !darkMode;
		updateTheme();
	}

	function toggleSidebar() {
		sidebarCollapsed = !sidebarCollapsed;
	}
</script>

<div class="flex h-screen bg-background">
	<!-- Navigation Sidebar -->
	<Navigation 
		{darkMode} 
		collapsed={sidebarCollapsed}
		on:toggleTheme={toggleTheme}
		on:toggleSidebar={toggleSidebar}
	/>

	<!-- Main Content Area -->
	<main class={cn(
		'flex-1 overflow-auto transition-all duration-300',
		sidebarCollapsed ? 'lg:ml-16' : 'lg:ml-64',
		className
	)}>
		<div class="container mx-auto p-6 lg:p-8">
			<slot />
		</div>
	</main>
</div>

<!-- Global styles for the layout -->
<style>
	:global(html) {
		scroll-behavior: smooth;
	}
	
	:global(body) {
		overflow: hidden;
	}
</style>
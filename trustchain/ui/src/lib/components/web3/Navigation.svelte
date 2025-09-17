<script lang="ts">
	import { cn } from '$lib/utils.js';
	import Button from '../ui/button.svelte';
	import Badge from '../ui/badge.svelte';
	import { url } from '@roxi/routify';
	import { 
		LayoutDashboard, 
		Shield, 
		Network, 
		HardDrive, 
		Coins, 
		Settings, 
		Moon, 
		Sun,
		Menu,
		X
	} from 'lucide-svelte';

	interface NavItem {
		label: string;
		path: string;
		icon: any;
		badge?: string;
		color?: string;
	}

	interface $$Props {
		darkMode?: boolean;
		collapsed?: boolean;
		class?: string;
	}

	let { darkMode = false, collapsed = false, class: className }: $$Props = $props();
	let mobileMenuOpen = $state(false);

	const navItems: NavItem[] = [
		{
			label: 'Dashboard',
			path: '/',
			icon: LayoutDashboard
		},
		{
			label: 'TrustChain',
			path: '/trustchain',
			icon: Shield,
			color: 'text-trustchain-600',
			badge: 'CA'
		},
		{
			label: 'STOQ Protocol',
			path: '/stoq',
			icon: Network,
			color: 'text-stoq-600',
			badge: '2.95 Gbps'
		},
		{
			label: 'HyperMesh',
			path: '/hypermesh',
			icon: HardDrive,
			color: 'text-hypermesh-600'
		},
		{
			label: 'Caesar',
			path: '/caesar',
			icon: Coins,
			color: 'text-caesar-600'
		},
		{
			label: 'Consensus',
			path: '/consensus',
			icon: Shield,
			color: 'text-quantum-600',
			badge: '4-Proof'
		},
		{
			label: 'Settings',
			path: '/settings',
			icon: Settings
		}
	];

	function toggleMobileMenu() {
		mobileMenuOpen = !mobileMenuOpen;
	}

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}
</script>

<!-- Mobile menu button -->
<div class="lg:hidden fixed top-4 left-4 z-50">
	<Button variant="ghost" size="icon" onclick={toggleMobileMenu}>
		{#if mobileMenuOpen}
			<X class="h-5 w-5" />
		{:else}
			<Menu class="h-5 w-5" />
		{/if}
	</Button>
</div>

<!-- Mobile backdrop -->
{#if mobileMenuOpen}
	<div 
		class="lg:hidden fixed inset-0 bg-black/50 z-40" 
		onclick={closeMobileMenu}
	/>
{/if}

<!-- Navigation Sidebar -->
<nav class={cn(
	'fixed top-0 left-0 z-40 h-screen bg-background border-r transition-all duration-300',
	collapsed ? 'w-16' : 'w-64',
	mobileMenuOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0',
	className
)}>
	<div class="flex flex-col h-full">
		<!-- Header -->
		<div class="p-6 border-b">
			<div class="flex items-center space-x-3">
				<div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
					<Shield class="h-5 w-5 text-primary-foreground" />
				</div>
				{#if !collapsed}
					<div>
						<h1 class="font-bold text-lg">Web3 Ecosystem</h1>
						<p class="text-xs text-muted-foreground">Quantum-Secure Infrastructure</p>
					</div>
				{/if}
			</div>
		</div>

		<!-- Navigation Items -->
		<div class="flex-1 overflow-y-auto py-4">
			<div class="space-y-1 px-3">
				{#each navItems as item}
					<a 
						href={$url(item.path)}
						class={cn(
							'flex items-center space-x-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
							'hover:bg-accent hover:text-accent-foreground',
							$url().path === item.path ? 'bg-accent text-accent-foreground' : 'text-muted-foreground'
						)}
						onclick={closeMobileMenu}
					>
						<svelte:component 
							this={item.icon} 
							class={cn('h-5 w-5', item.color || 'text-current')} 
						/>
						{#if !collapsed}
							<span class="flex-1">{item.label}</span>
							{#if item.badge}
								<Badge variant="outline" class="text-xs">
									{item.badge}
								</Badge>
							{/if}
						{/if}
					</a>
				{/each}
			</div>
		</div>

		<!-- Footer -->
		<div class="p-4 border-t">
			{#if !collapsed}
				<div class="space-y-3">
					<!-- Dark Mode Toggle -->
					<Button variant="ghost" class="w-full justify-start" size="sm">
						{#if darkMode}
							<Sun class="h-4 w-4 mr-2" />
							Light Mode
						{:else}
							<Moon class="h-4 w-4 mr-2" />
							Dark Mode
						{/if}
					</Button>

					<!-- System Status Summary -->
					<div class="text-xs text-muted-foreground space-y-1">
						<div class="flex justify-between">
							<span>FALCON-1024:</span>
							<span class="text-green-500 font-medium">Active</span>
						</div>
						<div class="flex justify-between">
							<span>IPv6 Network:</span>
							<span class="text-green-500 font-medium">Connected</span>
						</div>
						<div class="flex justify-between">
							<span>Consensus:</span>
							<span class="text-green-500 font-medium">4-Proof</span>
						</div>
					</div>
				</div>
			{:else}
				<Button variant="ghost" size="icon" class="w-full">
					{#if darkMode}
						<Sun class="h-4 w-4" />
					{:else}
						<Moon class="h-4 w-4" />
					{/if}
				</Button>
			{/if}
		</div>
	</div>
</nav>
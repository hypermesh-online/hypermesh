<script lang="ts">
	import { cn, getStatusColor, copyToClipboard } from '$lib/utils.js';
	import type { Certificate } from '$lib/types.js';
	import Card from '../ui/card.svelte';
	import Badge from '../ui/badge.svelte';
	import Button from '../ui/button.svelte';
	import { Shield, Copy, Calendar, Key } from 'lucide-svelte';

	interface $$Props {
		certificate: Certificate;
		showDetails?: boolean;
		class?: string;
	}

	let { certificate, showDetails = false, class: className }: $$Props = $props();

	$: statusVariant = certificate.status === 'valid' ? 'default'
		: certificate.status === 'expired' ? 'secondary'
		: certificate.status === 'revoked' ? 'destructive'
		: 'outline';

	$: isExpiringSoon = certificate.status === 'valid' && 
		new Date(certificate.validTo).getTime() - Date.now() < 30 * 24 * 60 * 60 * 1000; // 30 days

	async function handleCopyFingerprint() {
		try {
			await copyToClipboard(certificate.fingerprint);
			// Could add toast notification here
		} catch (error) {
			console.error('Failed to copy fingerprint:', error);
		}
	}
</script>

<Card class={cn('p-6', className)}>
	<div class="flex items-start justify-between">
		<div class="flex items-center space-x-3">
			<div class="p-2 rounded-lg bg-trustchain-50">
				<Shield class="h-5 w-5 text-trustchain-600" />
			</div>
			<div>
				<h3 class="font-semibold">{certificate.subject}</h3>
				<p class="text-sm text-muted-foreground">Serial: {certificate.serialNumber}</p>
			</div>
		</div>
		<div class="flex flex-col items-end space-y-2">
			<Badge variant={statusVariant} class="capitalize">
				{certificate.status}
			</Badge>
			{#if isExpiringSoon}
				<Badge variant="destructive" class="text-xs">
					Expires Soon
				</Badge>
			{/if}
		</div>
	</div>

	<div class="mt-4 space-y-3">
		<!-- Issuer -->
		<div class="flex items-center justify-between">
			<span class="text-sm text-muted-foreground">Issuer:</span>
			<span class="text-sm font-medium">{certificate.issuer}</span>
		</div>

		<!-- Validity Period -->
		<div class="space-y-1">
			<div class="flex items-center justify-between">
				<span class="text-sm text-muted-foreground">Valid From:</span>
				<span class="text-sm font-medium">{new Date(certificate.validFrom).toLocaleDateString()}</span>
			</div>
			<div class="flex items-center justify-between">
				<span class="text-sm text-muted-foreground">Valid To:</span>
				<span class="text-sm font-medium">{new Date(certificate.validTo).toLocaleDateString()}</span>
			</div>
		</div>

		{#if showDetails}
			<!-- Algorithms -->
			<div class="space-y-1 pt-2 border-t">
				<div class="flex items-center justify-between">
					<span class="text-sm text-muted-foreground">Key Algorithm:</span>
					<span class="text-sm font-medium">{certificate.keyAlgorithm}</span>
				</div>
				<div class="flex items-center justify-between">
					<span class="text-sm text-muted-foreground">Signature:</span>
					<span class="text-sm font-medium">{certificate.signatureAlgorithm}</span>
				</div>
			</div>

			<!-- Fingerprint -->
			<div class="space-y-2 pt-2 border-t">
				<span class="text-sm font-medium text-muted-foreground">SHA-256 Fingerprint:</span>
				<div class="flex items-center space-x-2">
					<code class="flex-1 text-xs bg-muted p-2 rounded font-mono break-all">
						{certificate.fingerprint}
					</code>
					<Button variant="ghost" size="icon" onclick={handleCopyFingerprint}>
						<Copy class="h-4 w-4" />
					</Button>
				</div>
			</div>

			<!-- Extensions -->
			{#if certificate.extensions.length > 0}
				<div class="space-y-2 pt-2 border-t">
					<span class="text-sm font-medium text-muted-foreground">Extensions:</span>
					<div class="space-y-1">
						{#each certificate.extensions as ext}
							<div class="flex items-center justify-between text-xs">
								<span class="font-mono">{ext.oid}</span>
								<div class="flex space-x-1">
									{#if ext.critical}
										<Badge variant="destructive" class="text-xs">Critical</Badge>
									{/if}
									<span class="text-muted-foreground">
										{ext.value.length > 20 ? ext.value.substring(0, 20) + '...' : ext.value}
									</span>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{/if}
	</div>
</Card>
<script lang="ts">
	import { onMount } from 'svelte';
	import Layout from '$lib/components/Layout.svelte';
	import CertificateCard from '$lib/components/web3/CertificateCard.svelte';
	import MetricsCard from '$lib/components/web3/MetricsCard.svelte';
	import Card from '$lib/components/ui/card.svelte';
	import Button from '$lib/components/ui/button.svelte';
	import Badge from '$lib/components/ui/badge.svelte';
	import { Shield, Plus, Download, Search, Filter } from 'lucide-svelte';
	import type { Certificate, CertificateAuthority } from '$lib/types.js';

	// Mock data
	let certificates: Certificate[] = $state([
		{
			id: 'cert-001',
			subject: 'CN=hypermesh.node.001,O=HyperMesh Network',
			issuer: 'CN=TrustChain Root CA,O=Web3 Ecosystem',
			serialNumber: '1A2B3C4D5E6F',
			fingerprint: 'A1B2C3D4E5F6789012345678901234567890123456789012345678901234567890',
			validFrom: '2024-01-01T00:00:00Z',
			validTo: '2025-12-31T23:59:59Z',
			status: 'valid',
			keyAlgorithm: 'FALCON-1024',
			signatureAlgorithm: 'FALCON-1024-SHA256',
			extensions: [
				{ oid: '2.5.29.15', critical: true, value: 'Digital Signature, Key Encipherment' },
				{ oid: '2.5.29.37', critical: false, value: 'TLS Web Server Authentication' }
			]
		},
		{
			id: 'cert-002',
			subject: 'CN=stoq.protocol.gateway,O=STOQ Network',
			issuer: 'CN=TrustChain Root CA,O=Web3 Ecosystem',
			serialNumber: '2B3C4D5E6F7A',
			fingerprint: 'B2C3D4E5F6789012345678901234567890123456789012345678901234567890A1',
			validFrom: '2024-02-01T00:00:00Z',
			validTo: '2025-01-31T23:59:59Z',
			status: 'valid',
			keyAlgorithm: 'FALCON-1024',
			signatureAlgorithm: 'FALCON-1024-SHA256',
			extensions: [
				{ oid: '2.5.29.15', critical: true, value: 'Digital Signature, Key Agreement' },
				{ oid: '2.5.29.17', critical: false, value: 'DNS:stoq.protocol.gateway' }
			]
		},
		{
			id: 'cert-003',
			subject: 'CN=caesar.economics.node,O=Caesar Network',
			issuer: 'CN=TrustChain Intermediate CA,O=Web3 Ecosystem',
			serialNumber: '3C4D5E6F7A8B',
			fingerprint: 'C3D4E5F6789012345678901234567890123456789012345678901234567890A1B2',
			validFrom: '2024-03-01T00:00:00Z',
			validTo: '2024-12-31T23:59:59Z',
			status: 'expired',
			keyAlgorithm: 'FALCON-1024',
			signatureAlgorithm: 'FALCON-1024-SHA256',
			extensions: [
				{ oid: '2.5.29.15', critical: true, value: 'Digital Signature' },
				{ oid: '2.5.29.37', critical: false, value: 'Code Signing' }
			]
		}
	]);

	let caMetrics = $state({
		totalCertificates: 892,
		validCertificates: 856,
		expiredCertificates: 24,
		revokedCertificates: 12,
		issuanceRate: 2.3, // per day
		rootCAs: 3,
		intermediateCAs: 7
	});

	let searchTerm = $state('');
	let statusFilter = $state('all');
	let loading = $state(false);

	$: filteredCertificates = certificates.filter(cert => {
		const matchesSearch = searchTerm === '' || 
			cert.subject.toLowerCase().includes(searchTerm.toLowerCase()) ||
			cert.issuer.toLowerCase().includes(searchTerm.toLowerCase()) ||
			cert.serialNumber.toLowerCase().includes(searchTerm.toLowerCase());
		
		const matchesStatus = statusFilter === 'all' || cert.status === statusFilter;
		
		return matchesSearch && matchesStatus;
	});

	function issueCertificate() {
		// Simulate certificate issuance
		loading = true;
		setTimeout(() => {
			loading = false;
			// Add mock certificate
			const newCert: Certificate = {
				id: `cert-${String(certificates.length + 1).padStart(3, '0')}`,
				subject: 'CN=new.node.001,O=HyperMesh Network',
				issuer: 'CN=TrustChain Root CA,O=Web3 Ecosystem',
				serialNumber: Math.random().toString(16).substring(2, 14).toUpperCase(),
				fingerprint: Array.from({length: 64}, () => Math.floor(Math.random() * 16).toString(16)).join('').toUpperCase(),
				validFrom: new Date().toISOString(),
				validTo: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
				status: 'valid',
				keyAlgorithm: 'FALCON-1024',
				signatureAlgorithm: 'FALCON-1024-SHA256',
				extensions: []
			};
			certificates = [newCert, ...certificates];
			caMetrics.totalCertificates++;
			caMetrics.validCertificates++;
		}, 2000);
	}

	function exportCertificates() {
		// Simulate export
		const data = JSON.stringify(filteredCertificates, null, 2);
		const blob = new Blob([data], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'trustchain-certificates.json';
		a.click();
		URL.revokeObjectURL(url);
	}
</script>

<Layout>
	<!-- Header -->
	<div class="flex items-center justify-between mb-8">
		<div>
			<h1 class="text-3xl font-bold tracking-tight flex items-center">
				<Shield class="h-8 w-8 text-trustchain-600 mr-3" />
				TrustChain Certificate Authority
			</h1>
			<p class="text-muted-foreground mt-2">
				Quantum-resistant certificate management with FALCON-1024
			</p>
		</div>
		<div class="flex space-x-3">
			<Button variant="outline" onclick={exportCertificates}>
				<Download class="h-4 w-4 mr-2" />
				Export
			</Button>
			<Button onclick={issueCertificate} disabled={loading}>
				<Plus class="h-4 w-4 mr-2" />
				{loading ? 'Issuing...' : 'Issue Certificate'}
			</Button>
		</div>
	</div>

	<!-- Metrics Grid -->
	<div class="metrics-grid mb-8">
		<MetricsCard
			title="Total Certificates"
			value={caMetrics.totalCertificates}
			icon={Shield}
			color="text-trustchain-600"
		/>
		<MetricsCard
			title="Valid Certificates"
			value={caMetrics.validCertificates}
			change={1.2}
			changeLabel="this week"
			icon={Shield}
			color="text-green-600"
		/>
		<MetricsCard
			title="Expired"
			value={caMetrics.expiredCertificates}
			icon={Shield}
			color="text-yellow-600"
		/>
		<MetricsCard
			title="Issuance Rate"
			value={caMetrics.issuanceRate}
			unit="per day"
			change={0.8}
			changeLabel="this month"
			icon={Shield}
			color="text-blue-600"
		/>
	</div>

	<!-- CA Infrastructure Status -->
	<Card class="p-6 mb-8">
		<h3 class="font-semibold text-lg mb-4">Certificate Authority Infrastructure</h3>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<div class="space-y-2">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Root CAs:</span>
					<span class="font-medium">{caMetrics.rootCAs}</span>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Intermediate CAs:</span>
					<span class="font-medium">{caMetrics.intermediateCAs}</span>
				</div>
			</div>
			<div class="space-y-2">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Cryptography:</span>
					<Badge class="quantum-glow">FALCON-1024</Badge>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">Hash Algorithm:</span>
					<span class="font-medium">SHA-256</span>
				</div>
			</div>
			<div class="space-y-2">
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">OCSP Responder:</span>
					<Badge variant="default">Online</Badge>
				</div>
				<div class="flex justify-between">
					<span class="text-sm text-muted-foreground">CRL Distribution:</span>
					<Badge variant="default">Active</Badge>
				</div>
			</div>
		</div>
	</Card>

	<!-- Search and Filter -->
	<Card class="p-6 mb-6">
		<div class="flex flex-col sm:flex-row gap-4">
			<div class="flex-1">
				<div class="relative">
					<Search class="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
					<input
						type="text"
						placeholder="Search certificates by subject, issuer, or serial number..."
						class="w-full pl-10 pr-4 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
						bind:value={searchTerm}
					/>
				</div>
			</div>
			<div class="flex items-center space-x-2">
				<Filter class="h-4 w-4 text-muted-foreground" />
				<select
					class="px-3 py-2 border border-input rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
					bind:value={statusFilter}
				>
					<option value="all">All Statuses</option>
					<option value="valid">Valid</option>
					<option value="expired">Expired</option>
					<option value="revoked">Revoked</option>
					<option value="pending">Pending</option>
				</select>
			</div>
		</div>
	</Card>

	<!-- Certificates Grid -->
	<div class="space-y-6">
		<div class="flex items-center justify-between">
			<h2 class="text-2xl font-semibold">
				Certificates ({filteredCertificates.length})
			</h2>
			{#if searchTerm || statusFilter !== 'all'}
				<Button 
					variant="ghost" 
					size="sm" 
					onclick={() => { searchTerm = ''; statusFilter = 'all'; }}
				>
					Clear Filters
				</Button>
			{/if}
		</div>

		{#if filteredCertificates.length === 0}
			<Card class="p-12 text-center">
				<Shield class="h-12 w-12 text-muted-foreground mx-auto mb-4" />
				<h3 class="text-lg font-medium mb-2">No certificates found</h3>
				<p class="text-muted-foreground">
					{searchTerm || statusFilter !== 'all' 
						? 'Try adjusting your search criteria.' 
						: 'Issue your first certificate to get started.'}
				</p>
			</Card>
		{:else}
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				{#each filteredCertificates as certificate}
					<CertificateCard {certificate} showDetails={true} />
				{/each}
			</div>
		{/if}
	</div>
</Layout>
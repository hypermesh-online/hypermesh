<script>
  import { onMount } from 'svelte';
  import { 
    asset_registry, 
    assetStore,
    asset_manager_connected,
    ASSET_TYPES,
    PRIVACY_LEVELS
  } from '../../lib/stores/assets.js';
  import { 
    Home, 
    Car, 
    Cpu, 
    HardDrive, 
    Coins,
    FileText,
    Plus,
    Filter,
    Search
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';
  import Input from '../../lib/components/ui/Input.svelte';

  let searchQuery = '';
  let selectedCategory = 'all';
  let selectedPrivacyLevel = 'all';
  let showAddAssetModal = false;

  // Filter assets based on search and category
  $: filteredAssets = $asset_registry ? Array.from($asset_registry.values()).filter(asset => {
    const matchesSearch = asset.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         asset.type.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = selectedCategory === 'all' || getCategoryForAsset(asset.type) === selectedCategory;
    const matchesPrivacy = selectedPrivacyLevel === 'all' || asset.privacy_level === selectedPrivacyLevel;
    
    return matchesSearch && matchesCategory && matchesPrivacy;
  }) : [];

  function getCategoryForAsset(assetType) {
    if ([ASSET_TYPES.REAL_ESTATE, ASSET_TYPES.VEHICLE, ASSET_TYPES.EQUIPMENT, ASSET_TYPES.INVENTORY].includes(assetType)) {
      return 'physical';
    } else if ([ASSET_TYPES.TOKEN, ASSET_TYPES.CONTRACT, ASSET_TYPES.SERVICE, ASSET_TYPES.DATA].includes(assetType)) {
      return 'digital';
    } else if ([ASSET_TYPES.CPU, ASSET_TYPES.GPU, ASSET_TYPES.MEMORY, ASSET_TYPES.STORAGE, ASSET_TYPES.NETWORK].includes(assetType)) {
      return 'infrastructure';
    }
    return 'other';
  }

  function getAssetIcon(assetType) {
    switch(assetType) {
      case ASSET_TYPES.REAL_ESTATE: return Home;
      case ASSET_TYPES.VEHICLE: return Car;
      case ASSET_TYPES.CPU: return Cpu;
      case ASSET_TYPES.STORAGE: return HardDrive;
      case ASSET_TYPES.TOKEN: return Coins;
      case ASSET_TYPES.CONTRACT: return FileText;
      default: return FileText;
    }
  }

  function formatCurrency(value, currency) {
    if (currency === 'USD') {
      return `$${parseFloat(value).toLocaleString()}`;
    } else if (currency === 'CAESAR') {
      return `${parseFloat(value).toFixed(2)} CAESAR`;
    }
    return `${value} ${currency}`;
  }

  function getPrivacyBadgeColor(privacyLevel) {
    switch(privacyLevel) {
      case PRIVACY_LEVELS.PRIVATE: return 'bg-red-500/20 text-red-400';
      case PRIVACY_LEVELS.PRIVATE_NETWORK: return 'bg-orange-500/20 text-orange-400';
      case PRIVACY_LEVELS.P2P: return 'bg-yellow-500/20 text-yellow-400';
      case PRIVACY_LEVELS.PUBLIC_NETWORK: return 'bg-blue-500/20 text-blue-400';
      case PRIVACY_LEVELS.FULL_PUBLIC: return 'bg-green-500/20 text-green-400';
      default: return 'bg-gray-500/20 text-gray-400';
    }
  }

  onMount(() => {
    if ($asset_manager_connected) {
      assetStore.loadUniversalAssets();
    }
  });
</script>

<svelte:head>
  <title>Universal Assets - HyperMesh Asset Manager</title>
  <meta name="description" content="View and manage all your real-world and digital assets through HyperMesh" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Universal Assets</h1>
      <p class="text-gray-400 mt-2">
        Manage physical property, digital tokens, and HyperMesh infrastructure resources
      </p>
    </div>
    
    <Button on:click={() => showAddAssetModal = true}>
      <Plus class="w-4 h-4 mr-2" />
      Add Asset
    </Button>
  </div>

  <!-- Search and Filters -->
  <Card class="p-6">
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <!-- Search -->
      <div class="relative">
        <Search class="w-4 h-4 absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
        <Input
          bind:value={searchQuery}
          placeholder="Search assets..."
          class="pl-10"
        />
      </div>

      <!-- Category Filter -->
      <select 
        bind:value={selectedCategory}
        class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
      >
        <option value="all">All Categories</option>
        <option value="physical">Physical Assets</option>
        <option value="digital">Digital Assets</option>
        <option value="infrastructure">Infrastructure</option>
      </select>

      <!-- Privacy Filter -->
      <select 
        bind:value={selectedPrivacyLevel}
        class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
      >
        <option value="all">All Privacy Levels</option>
        <option value={PRIVACY_LEVELS.PRIVATE}>Private</option>
        <option value={PRIVACY_LEVELS.PRIVATE_NETWORK}>Private Network</option>
        <option value={PRIVACY_LEVELS.P2P}>P2P</option>
        <option value={PRIVACY_LEVELS.PUBLIC_NETWORK}>Public Network</option>
        <option value={PRIVACY_LEVELS.FULL_PUBLIC}>Full Public</option>
      </select>
    </div>
  </Card>

  <!-- Asset Grid -->
  {#if !$asset_manager_connected}
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Home class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to view and manage your assets.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else if filteredAssets.length === 0}
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Search class="w-16 h-16 text-gray-400 mx-auto" />
        <h2 class="text-xl font-semibold text-gray-300">No Assets Found</h2>
        <p class="text-gray-400">
          {searchQuery ? `No assets match "${searchQuery}"` : 'No assets match your current filters'}
        </p>
        <Button on:click={() => { searchQuery = ''; selectedCategory = 'all'; selectedPrivacyLevel = 'all'; }}>
          Clear Filters
        </Button>
      </div>
    </Card>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {#each filteredAssets as asset (asset.id)}
        <Card class="p-6 hover:border-yellow-400/50 transition-colors cursor-pointer">
          <div class="flex items-start justify-between mb-4">
            <div class="flex items-center space-x-3">
              <div class="w-12 h-12 bg-gray-800 rounded-lg flex items-center justify-center">
                <svelte:component this={getAssetIcon(asset.type)} class="w-6 h-6 text-yellow-400" />
              </div>
              <div>
                <h3 class="font-semibold text-white text-lg">{asset.name}</h3>
                <p class="text-sm text-gray-400 capitalize">
                  {asset.type.replace('_', ' ')} • {getCategoryForAsset(asset.type)}
                </p>
              </div>
            </div>
            
            <!-- Privacy Level Badge -->
            <span class="px-2 py-1 text-xs rounded-full {getPrivacyBadgeColor(asset.privacy_level)}">
              {asset.privacy_level.replace('_', ' ').toUpperCase()}
            </span>
          </div>

          <!-- Asset Value -->
          <div class="mb-4">
            <p class="text-2xl font-bold text-white mb-1">
              {formatCurrency(asset.value, asset.currency)}
            </p>
            
            <!-- Demurrage for CAESAR tokens -->
            {#if asset.currency === 'CAESAR' && asset.demurrage}
              <p class="text-sm text-orange-400">
                Effective: {formatCurrency(asset.demurrage.effective_balance, 'CAESAR')}
              </p>
              <p class="text-xs text-gray-500">
                Demurrage: -{asset.demurrage.accumulated.toFixed(2)} CAESAR
              </p>
            {/if}
          </div>

          <!-- Consensus Proofs Status -->
          <div class="mb-4">
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm text-gray-400">Consensus Status</span>
              <span class="text-xs text-gray-500">
                {Object.values(asset.consensus_proofs).filter(p => p === true).length}/4 proofs
              </span>
            </div>
            
            <div class="w-full bg-gray-700 rounded-full h-2">
              <div 
                class="bg-gradient-to-r from-yellow-400 to-yellow-600 h-2 rounded-full transition-all duration-300"
                style="width: {(Object.values(asset.consensus_proofs).filter(p => p === true).length / 4) * 100}%"
              ></div>
            </div>
          </div>

          <!-- Asset-Specific Details -->
          {#if asset.metadata}
            <div class="bg-gray-800/50 rounded-lg p-3 mb-4">
              <p class="text-sm text-gray-400 mb-2">Details</p>
              {#if asset.type === ASSET_TYPES.REAL_ESTATE}
                <div class="grid grid-cols-2 gap-2 text-sm text-gray-300">
                  <span>{asset.metadata.bedrooms} bed</span>
                  <span>{asset.metadata.bathrooms} bath</span>
                  <span>{asset.metadata.square_feet} sq ft</span>
                  <span class="text-orange-400">${asset.metadata.monthly_payment}/mo</span>
                </div>
              {:else if asset.type === ASSET_TYPES.VEHICLE}
                <div class="grid grid-cols-2 gap-2 text-sm text-gray-300">
                  <span>{asset.metadata.mileage.toLocaleString()} miles</span>
                  <span>VIN: {asset.metadata.vin.slice(-6)}</span>
                </div>
              {:else if asset.type === ASSET_TYPES.CONTRACT}
                <div class="text-sm text-gray-300">
                  <p>Employer: {asset.metadata.employer}</p>
                  <p class="text-green-400">${asset.metadata.annual_salary.toLocaleString()}/year</p>
                </div>
              {/if}
            </div>
          {/if}

          <!-- Sharing/Allocation Info -->
          {#if asset.sharing_config}
            <div class="bg-blue-500/10 rounded-lg p-3 mb-4">
              <p class="text-sm text-blue-400 mb-1">Sharing Active</p>
              <p class="text-sm text-gray-300">
                {asset.sharing_config.hourly_rate} CAESAR/hour • {asset.sharing_config.available_hours}h/day
              </p>
            </div>
          {/if}

          {#if asset.allocation}
            <div class="bg-green-500/10 rounded-lg p-3 mb-4">
              <p class="text-sm text-green-400 mb-1">HyperMesh Resource</p>
              {#if asset.allocation.total_cores}
                <p class="text-sm text-gray-300">
                  {asset.allocation.shared_cores}/{asset.allocation.total_cores} cores shared
                </p>
              {:else if asset.allocation.total_space}
                <p class="text-sm text-gray-300">
                  {((asset.allocation.shared_space - asset.allocation.used_space) / 1024).toFixed(1)}TB available
                </p>
              {/if}
            </div>
          {/if}

          <!-- Action Buttons -->
          <div class="flex space-x-2">
            <Button size="sm" variant="outline" class="flex-1">
              View Details
            </Button>
            {#if asset.type === ASSET_TYPES.CONTRACT}
              <Button size="sm" class="flex-1">
                Execute
              </Button>
            {:else if asset.sharing_config || asset.allocation}
              <Button size="sm" variant="outline" class="flex-1">
                Configure
              </Button>
            {/if}
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>

<!-- Add Asset Modal (placeholder) -->
{#if showAddAssetModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <Card class="p-6 max-w-md w-full mx-4">
      <h2 class="text-xl font-semibold mb-4">Add New Asset</h2>
      <p class="text-gray-400 mb-4">
        Asset registration will be implemented with HyperMesh integration.
      </p>
      <div class="flex space-x-2">
        <Button variant="outline" class="flex-1" on:click={() => showAddAssetModal = false}>
          Cancel
        </Button>
        <Button class="flex-1" on:click={() => showAddAssetModal = false}>
          Coming Soon
        </Button>
      </div>
    </Card>
  </div>
{/if}

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
</style>
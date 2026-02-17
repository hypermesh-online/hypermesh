<script>
  import { onMount } from 'svelte';
  import { 
    assetStore, 
    asset_registry, 
    asset_manager_connected,
    caesar_economy,
    hypermesh_allocations,
    consensus_status,
    total_asset_value,
    shared_resources_earnings,
    real_economy_activity,
    ASSET_TYPES,
    PRIVACY_LEVELS
  } from '../../stores/assets.js';
  import { 
    Home, 
    Car, 
    Cpu, 
    HardDrive, 
    Zap, 
    Shield, 
    TrendingUp,
    AlertCircle,
    CheckCircle,
    DollarSign,
    Activity
  } from 'lucide-svelte';
  import Card from '../ui/Card.svelte';
  import Button from '../ui/Button.svelte';

  let selectedAssetType = 'all';
  let assetTypeFilter = 'all';
  
  // Reactive asset filtering
  $: filteredAssets = $asset_registry ? Array.from($asset_registry.values()).filter(asset => {
    if (assetTypeFilter === 'all') return true;
    return asset.type === assetTypeFilter;
  }) : [];

  // Asset type counts
  $: assetTypeCounts = $asset_registry ? {
    physical: Array.from($asset_registry.values()).filter(a => 
      [ASSET_TYPES.REAL_ESTATE, ASSET_TYPES.VEHICLE, ASSET_TYPES.EQUIPMENT].includes(a.type)
    ).length,
    digital: Array.from($asset_registry.values()).filter(a => 
      [ASSET_TYPES.TOKEN, ASSET_TYPES.CONTRACT, ASSET_TYPES.SERVICE].includes(a.type)
    ).length,
    infrastructure: Array.from($asset_registry.values()).filter(a => 
      [ASSET_TYPES.CPU, ASSET_TYPES.GPU, ASSET_TYPES.MEMORY, ASSET_TYPES.STORAGE].includes(a.type)
    ).length
  } : { physical: 0, digital: 0, infrastructure: 0 };

  function getAssetIcon(assetType) {
    switch(assetType) {
      case ASSET_TYPES.REAL_ESTATE: return Home;
      case ASSET_TYPES.VEHICLE: return Car;
      case ASSET_TYPES.CPU: return Cpu;
      case ASSET_TYPES.STORAGE: return HardDrive;
      case ASSET_TYPES.CONTRACT: return Shield;
      default: return Activity;
    }
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

  function formatCurrency(value, currency) {
    if (currency === 'USD') {
      return `$${parseFloat(value).toLocaleString()}`;
    } else if (currency === 'CAESAR') {
      return `${parseFloat(value).toFixed(2)} CAESAR`;
    }
    return `${value} ${currency}`;
  }

  function getConsensusProofStatus(proofs) {
    const validProofs = Object.values(proofs).filter(p => p === true).length;
    return {
      count: validProofs,
      total: 4,
      percentage: (validProofs / 4) * 100,
      status: validProofs === 4 ? 'complete' : validProofs >= 2 ? 'partial' : 'low'
    };
  }

  onMount(() => {
    if ($asset_manager_connected) {
      assetStore.loadUniversalAssets();
    }
  });
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Universal Asset Manager</h1>
      <p class="text-gray-400 mt-2">Manage your real-world economy through HyperMesh</p>
    </div>
    
    {#if $asset_manager_connected}
      <div class="flex items-center space-x-2">
        <CheckCircle class="w-5 h-5 text-green-400" />
        <span class="text-sm text-green-400">HyperMesh Connected</span>
      </div>
    {:else}
      <Button on:click={() => assetStore.connectAssetManager()}>
        Connect HyperMesh
      </Button>
    {/if}
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Shield class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to manage your real-world assets, execute contracts, 
          and participate in the Caesar economy.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Dashboard Content -->
    
    <!-- Overview Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <!-- Total Asset Value -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Total Asset Value</p>
            <p class="text-2xl font-bold text-white">${$total_asset_value}</p>
          </div>
          <DollarSign class="w-8 h-8 text-yellow-400" />
        </div>
        <div class="mt-4 flex items-center text-sm">
          <TrendingUp class="w-4 h-4 text-green-400 mr-1" />
          <span class="text-green-400">+5.2% from last month</span>
        </div>
      </Card>

      <!-- HyperMesh Earnings -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">HyperMesh Earnings</p>
            <p class="text-2xl font-bold text-white">{$shared_resources_earnings}</p>
          </div>
          <Zap class="w-8 h-8 text-yellow-400" />
        </div>
        <div class="mt-4 flex items-center text-sm">
          <Activity class="w-4 h-4 text-blue-400 mr-1" />
          <span class="text-blue-400">
            {$hypermesh_allocations.cpu_shared + $hypermesh_allocations.storage_shared}% resources shared
          </span>
        </div>
      </Card>

      <!-- Active Contracts -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Active Contracts</p>
            <p class="text-2xl font-bold text-white">{$real_economy_activity}</p>
          </div>
          <Shield class="w-8 h-8 text-yellow-400" />
        </div>
        <div class="mt-4 flex items-center text-sm">
          <CheckCircle class="w-4 h-4 text-green-400 mr-1" />
          <span class="text-green-400">All contracts validated</span>
        </div>
      </Card>

      <!-- Consensus Health -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Consensus Health</p>
            <p class="text-2xl font-bold text-white">{$consensus_status.consensus_score}%</p>
          </div>
          {#if $consensus_status.consensus_score >= 75}
            <CheckCircle class="w-8 h-8 text-green-400" />
          {:else}
            <AlertCircle class="w-8 h-8 text-red-400" />
          {/if}
        </div>
        <div class="mt-4 flex items-center text-sm">
          <span class="text-gray-400">
            {Object.values($consensus_status).filter(v => v === true).length}/4 proofs validated
          </span>
        </div>
      </Card>
    </div>

    <!-- Asset Type Filter -->
    <div class="flex space-x-2">
      <Button 
        variant={assetTypeFilter === 'all' ? 'default' : 'outline'} 
        size="sm"
        on:click={() => assetTypeFilter = 'all'}
      >
        All Assets ({Array.from($asset_registry.values()).length})
      </Button>
      <Button 
        variant={assetTypeFilter === 'physical' ? 'default' : 'outline'} 
        size="sm"
        on:click={() => assetTypeFilter = 'physical'}
      >
        Physical ({assetTypeCounts.physical})
      </Button>
      <Button 
        variant={assetTypeFilter === 'digital' ? 'default' : 'outline'} 
        size="sm"
        on:click={() => assetTypeFilter = 'digital'}
      >
        Digital ({assetTypeCounts.digital})
      </Button>
      <Button 
        variant={assetTypeFilter === 'infrastructure' ? 'default' : 'outline'} 
        size="sm"
        on:click={() => assetTypeFilter = 'infrastructure'}
      >
        Infrastructure ({assetTypeCounts.infrastructure})
      </Button>
    </div>

    <!-- Asset Registry -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      {#each filteredAssets as asset (asset.id)}
        <Card class="p-6">
          <div class="flex items-start justify-between mb-4">
            <div class="flex items-center space-x-3">
              <div class="w-10 h-10 bg-gray-800 rounded-lg flex items-center justify-center">
                <svelte:component this={getAssetIcon(asset.type)} class="w-5 h-5 text-yellow-400" />
              </div>
              <div>
                <h3 class="font-semibold text-white">{asset.name}</h3>
                <p class="text-sm text-gray-400 capitalize">{asset.type.replace('_', ' ')}</p>
              </div>
            </div>
            
            <!-- Privacy Level Badge -->
            <span class="px-2 py-1 text-xs rounded-full {getPrivacyBadgeColor(asset.privacy_level)}">
              {asset.privacy_level.replace('_', ' ').toUpperCase()}
            </span>
          </div>

          <!-- Asset Value -->
          <div class="mb-4">
            <p class="text-2xl font-bold text-white">
              {formatCurrency(asset.value, asset.currency)}
            </p>
            
            <!-- Demurrage for CAESAR tokens -->
            {#if asset.currency === 'CAESAR' && asset.demurrage}
              <p class="text-sm text-orange-400">
                Effective: {formatCurrency(asset.demurrage.effective_balance, 'CAESAR')} 
                (-{asset.demurrage.accumulated.toFixed(2)} demurrage)
              </p>
            {/if}
          </div>

          <!-- Consensus Proofs -->
          <div class="mb-4">
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm text-gray-400">Consensus Proofs</span>
              {#if getConsensusProofStatus(asset.consensus_proofs).status === 'complete'}
                <CheckCircle class="w-4 h-4 text-green-400" />
              {:else}
                <AlertCircle class="w-4 h-4 text-orange-400" />
              {/if}
            </div>
            
            <div class="grid grid-cols-4 gap-1">
              <div class="text-center">
                <div class="w-2 h-2 rounded-full mx-auto {asset.consensus_proofs.pos ? 'bg-green-400' : 'bg-gray-600'}"></div>
                <span class="text-xs text-gray-400">PoS</span>
              </div>
              <div class="text-center">
                <div class="w-2 h-2 rounded-full mx-auto {asset.consensus_proofs.post ? 'bg-green-400' : 'bg-gray-600'}"></div>
                <span class="text-xs text-gray-400">PoSt</span>
              </div>
              <div class="text-center">
                <div class="w-2 h-2 rounded-full mx-auto {asset.consensus_proofs.powk ? 'bg-green-400' : 'bg-gray-600'}"></div>
                <span class="text-xs text-gray-400">PoWk</span>
              </div>
              <div class="text-center">
                <div class="w-2 h-2 rounded-full mx-auto {asset.consensus_proofs.potm ? 'bg-green-400' : 'bg-gray-600'}"></div>
                <span class="text-xs text-gray-400">PoTm</span>
              </div>
            </div>
          </div>

          <!-- Asset-Specific Information -->
          {#if asset.sharing_config}
            <!-- Vehicle Sharing Info -->
            <div class="bg-gray-800/50 rounded-lg p-3 mb-4">
              <p class="text-sm text-gray-400 mb-1">Sharing Configuration</p>
              <div class="grid grid-cols-2 gap-2 text-sm">
                <span class="text-gray-300">Available: {asset.sharing_config.available_hours}h/day</span>
                <span class="text-gray-300">Rate: {asset.sharing_config.hourly_rate} CAESAR/h</span>
              </div>
            </div>
          {/if}

          {#if asset.allocation}
            <!-- Infrastructure Resource Allocation -->
            <div class="bg-gray-800/50 rounded-lg p-3 mb-4">
              <p class="text-sm text-gray-400 mb-1">Resource Allocation</p>
              {#if asset.allocation.total_cores}
                <div class="text-sm">
                  <span class="text-gray-300">
                    Shared: {asset.allocation.shared_cores}/{asset.allocation.total_cores} cores
                  </span>
                  <span class="text-yellow-400 ml-2">
                    ({asset.allocation.earnings_per_hour} CAESAR/h)
                  </span>
                </div>
              {:else if asset.allocation.total_space}
                <div class="text-sm">
                  <span class="text-gray-300">
                    Shared: {asset.allocation.shared_space - asset.allocation.used_space}/{asset.allocation.shared_space} GB
                  </span>
                  <span class="text-yellow-400 ml-2">
                    ({asset.allocation.earnings_per_gb_hour} CAESAR/GB/h)
                  </span>
                </div>
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
              <Button size="sm" class="flex-1">
                Configure
              </Button>
            {/if}
          </div>
        </Card>
      {/each}
    </div>

    {#if filteredAssets.length === 0}
      <Card class="text-center py-12">
        <Activity class="w-16 h-16 text-gray-400 mx-auto mb-4" />
        <h2 class="text-xl font-semibold text-gray-300">No Assets Found</h2>
        <p class="text-gray-400 mt-2">
          {assetTypeFilter === 'all' ? 'Connect your assets to get started' : `No ${assetTypeFilter} assets found`}
        </p>
      </Card>
    {/if}
  {/if}
</div>

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
</style>
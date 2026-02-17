<script>
  import { onMount } from 'svelte';
  import { 
    hypermesh_allocations,
    consensus_status,
    privacy_config,
    assetStore,
    asset_manager_connected,
    loading_states,
    PRIVACY_LEVELS
  } from '../../lib/stores/assets.js';
  import { 
    Cpu,
    HardDrive,
    Zap,
    Shield,
    Settings,
    TrendingUp,
    Eye,
    EyeOff,
    Users,
    Globe,
    Lock
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';

  let showAdvancedSettings = false;

  // Resource allocation sliders
  let cpuAllocation = 0;
  let memoryAllocation = 0;
  let storageAllocation = 0;

  // Initialize sliders with current values
  $: if ($hypermesh_allocations && cpuAllocation === 0) {
    cpuAllocation = $hypermesh_allocations.cpu_shared;
    memoryAllocation = $hypermesh_allocations.memory_shared;
    storageAllocation = $hypermesh_allocations.storage_shared;
  }

  function getPrivacyLevelIcon(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: return Lock;
      case PRIVACY_LEVELS.PRIVATE_NETWORK: return Shield;
      case PRIVACY_LEVELS.P2P: return Users;
      case PRIVACY_LEVELS.PUBLIC_NETWORK: return Eye;
      case PRIVACY_LEVELS.FULL_PUBLIC: return Globe;
      default: return Shield;
    }
  }

  function getPrivacyLevelColor(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: return 'text-red-400';
      case PRIVACY_LEVELS.PRIVATE_NETWORK: return 'text-orange-400';
      case PRIVACY_LEVELS.P2P: return 'text-yellow-400';
      case PRIVACY_LEVELS.PUBLIC_NETWORK: return 'text-blue-400';
      case PRIVACY_LEVELS.FULL_PUBLIC: return 'text-green-400';
      default: return 'text-gray-400';
    }
  }

  function getPrivacyDescription(level) {
    switch(level) {
      case PRIVACY_LEVELS.PRIVATE: 
        return 'Internal network only, no external access';
      case PRIVACY_LEVELS.PRIVATE_NETWORK: 
        return 'Specific networks/groups only';
      case PRIVACY_LEVELS.P2P: 
        return 'Trusted peer sharing';
      case PRIVACY_LEVELS.PUBLIC_NETWORK: 
        return 'Specific public networks';
      case PRIVACY_LEVELS.FULL_PUBLIC: 
        return 'Maximum CAESAR rewards, full HyperMesh node';
      default: 
        return 'Privacy level not set';
    }
  }

  async function updateResourceAllocation(resourceType, percentage) {
    try {
      await assetStore.updateResourceAllocation(resourceType, percentage);
    } catch (error) {
      console.error(`Failed to update ${resourceType} allocation:`, error);
    }
  }

  function handleSliderChange(resourceType, value) {
    switch(resourceType) {
      case 'cpu':
        cpuAllocation = value;
        updateResourceAllocation('cpu', value);
        break;
      case 'memory':
        memoryAllocation = value;
        updateResourceAllocation('memory', value);
        break;
      case 'storage':
        storageAllocation = value;
        updateResourceAllocation('storage', value);
        break;
    }
  }

  onMount(() => {
    if ($asset_manager_connected) {
      assetStore.loadHyperMeshAllocations();
    }
  });
</script>

<svelte:head>
  <title>HyperMesh Configuration - Resource Sharing & Privacy</title>
  <meta name="description" content="Configure your HyperMesh resource sharing, privacy levels, and consensus settings" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">HyperMesh Configuration</h1>
      <p class="text-gray-400 mt-2">
        Configure resource sharing, privacy levels, and earn CAESAR tokens
      </p>
    </div>
    
    <Button on:click={() => showAdvancedSettings = !showAdvancedSettings}>
      <Settings class="w-4 h-4 mr-2" />
      {showAdvancedSettings ? 'Hide' : 'Show'} Advanced
    </Button>
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Shield class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to configure your resource sharing and privacy settings.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Consensus Status -->
    <Card class="p-6">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-xl font-semibold">Consensus Health</h2>
        <div class="flex items-center space-x-2">
          {#if $consensus_status.consensus_score >= 75}
            <Shield class="w-5 h-5 text-green-400" />
            <span class="text-green-400">Healthy</span>
          {:else}
            <Shield class="w-5 h-5 text-orange-400" />
            <span class="text-orange-400">Degraded</span>
          {/if}
        </div>
      </div>

      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        <!-- Proof of Space -->
        <div class="text-center">
          <div class="w-12 h-12 mx-auto mb-2 {$consensus_status.pos_validated ? 'bg-green-500/20' : 'bg-red-500/20'} rounded-lg flex items-center justify-center">
            <Shield class="w-6 h-6 {$consensus_status.pos_validated ? 'text-green-400' : 'text-red-400'}" />
          </div>
          <p class="text-sm font-medium">Proof of Space</p>
          <p class="text-xs text-gray-400">Storage allocation</p>
        </div>

        <!-- Proof of Stake -->
        <div class="text-center">
          <div class="w-12 h-12 mx-auto mb-2 {$consensus_status.post_validated ? 'bg-green-500/20' : 'bg-red-500/20'} rounded-lg flex items-center justify-center">
            <Users class="w-6 h-6 {$consensus_status.post_validated ? 'text-green-400' : 'text-red-400'}" />
          </div>
          <p class="text-sm font-medium">Proof of Stake</p>
          <p class="text-xs text-gray-400">Ownership rights</p>
        </div>

        <!-- Proof of Work -->
        <div class="text-center">
          <div class="w-12 h-12 mx-auto mb-2 {$consensus_status.powk_validated ? 'bg-green-500/20' : 'bg-red-500/20'} rounded-lg flex items-center justify-center">
            <Cpu class="w-6 h-6 {$consensus_status.powk_validated ? 'text-green-400' : 'text-red-400'}" />
          </div>
          <p class="text-sm font-medium">Proof of Work</p>
          <p class="text-xs text-gray-400">Computational resources</p>
        </div>

        <!-- Proof of Time -->
        <div class="text-center">
          <div class="w-12 h-12 mx-auto mb-2 {$consensus_status.potm_validated ? 'bg-green-500/20' : 'bg-red-500/20'} rounded-lg flex items-center justify-center">
            <Zap class="w-6 h-6 {$consensus_status.potm_validated ? 'text-green-400' : 'text-red-400'}" />
          </div>
          <p class="text-sm font-medium">Proof of Time</p>
          <p class="text-xs text-gray-400">Temporal ordering</p>
        </div>
      </div>

      <div class="w-full bg-gray-700 rounded-full h-3">
        <div 
          class="bg-gradient-to-r from-yellow-400 to-yellow-600 h-3 rounded-full transition-all duration-300"
          style="width: {$consensus_status.consensus_score}%"
        ></div>
      </div>
      <p class="text-center text-sm text-gray-400 mt-2">
        Consensus Score: {$consensus_status.consensus_score}%
      </p>
    </Card>

    <!-- Resource Allocation -->
    <Card class="p-6">
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-xl font-semibold">Resource Allocation</h2>
        <div class="text-right">
          <p class="text-2xl font-bold text-yellow-400">{$hypermesh_allocations.earnings_rate} CAESAR/hour</p>
          <p class="text-sm text-gray-400">Current earnings rate</p>
        </div>
      </div>

      <div class="space-y-6">
        <!-- CPU Allocation -->
        <div>
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center space-x-2">
              <Cpu class="w-5 h-5 text-yellow-400" />
              <span class="font-medium">CPU Cores</span>
            </div>
            <span class="text-lg font-bold">{cpuAllocation}%</span>
          </div>
          
          <input
            type="range"
            min="0"
            max="100"
            bind:value={cpuAllocation}
            on:change={() => handleSliderChange('cpu', cpuAllocation)}
            class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer slider"
            disabled={$loading_states.updating_allocation}
          />
          
          <div class="flex justify-between text-xs text-gray-400 mt-1">
            <span>Private (0%)</span>
            <span>Full Sharing (100%)</span>
          </div>
          
          <p class="text-sm text-gray-400 mt-2">
            Sharing {cpuAllocation}% of CPU cores • Estimated: {(cpuAllocation * 0.0075).toFixed(3)} CAESAR/hour
          </p>
        </div>

        <!-- Memory Allocation -->
        <div>
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center space-x-2">
              <HardDrive class="w-5 h-5 text-yellow-400" />
              <span class="font-medium">Memory (RAM)</span>
            </div>
            <span class="text-lg font-bold">{memoryAllocation}%</span>
          </div>
          
          <input
            type="range"
            min="0"
            max="100"
            bind:value={memoryAllocation}
            on:change={() => handleSliderChange('memory', memoryAllocation)}
            class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer slider"
            disabled={$loading_states.updating_allocation}
          />
          
          <div class="flex justify-between text-xs text-gray-400 mt-1">
            <span>Private (0%)</span>
            <span>Full Sharing (100%)</span>
          </div>
          
          <p class="text-sm text-gray-400 mt-2">
            Sharing {memoryAllocation}% of RAM • Estimated: {(memoryAllocation * 0.0025).toFixed(3)} CAESAR/hour
          </p>
        </div>

        <!-- Storage Allocation -->
        <div>
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center space-x-2">
              <HardDrive class="w-5 h-5 text-yellow-400" />
              <span class="font-medium">Storage Space</span>
            </div>
            <span class="text-lg font-bold">{storageAllocation}%</span>
          </div>
          
          <input
            type="range"
            min="0"
            max="100"
            bind:value={storageAllocation}
            on:change={() => handleSliderChange('storage', storageAllocation)}
            class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer slider"
            disabled={$loading_states.updating_allocation}
          />
          
          <div class="flex justify-between text-xs text-gray-400 mt-1">
            <span>Private (0%)</span>
            <span>Full Sharing (100%)</span>
          </div>
          
          <p class="text-sm text-gray-400 mt-2">
            Sharing {storageAllocation}% of storage • Estimated: {(storageAllocation * 0.0085).toFixed(3)} CAESAR/hour
          </p>
        </div>
      </div>
    </Card>

    <!-- Privacy Configuration -->
    <Card class="p-6">
      <h2 class="text-xl font-semibold mb-4">Privacy & Sharing Settings</h2>
      
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- Default Privacy Level -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Default Privacy Level
          </label>
          <div class="space-y-2">
            {#each Object.values(PRIVACY_LEVELS) as level}
              <div class="flex items-center space-x-3 p-3 border border-gray-700 rounded-lg hover:border-gray-600 transition-colors">
                <input
                  type="radio"
                  name="default_privacy"
                  value={level}
                  bind:group={$privacy_config.default_level}
                  class="text-yellow-400 focus:ring-yellow-400"
                />
                <div class="flex items-center space-x-2 flex-1">
                  <svelte:component this={getPrivacyLevelIcon(level)} class="w-4 h-4 {getPrivacyLevelColor(level)}" />
                  <div>
                    <p class="font-medium capitalize">{level.replace('_', ' ')}</p>
                    <p class="text-xs text-gray-400">{getPrivacyDescription(level)}</p>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Sharing Limits -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Sharing Limits
          </label>
          
          <div class="space-y-4">
            <div>
              <label class="block text-sm text-gray-400 mb-1">
                Maximum Concurrent Users
              </label>
              <input
                type="number"
                min="1"
                max="100"
                bind:value={$privacy_config.max_concurrent_users}
                class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
              />
            </div>

            <div>
              <label class="block text-sm text-gray-400 mb-1">
                Consent Duration (hours)
              </label>
              <input
                type="number"
                min="1"
                max="168"
                bind:value={$privacy_config.consent_duration}
                class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-yellow-400 focus:border-transparent"
              />
            </div>

            <div class="flex items-center space-x-2">
              <input
                type="checkbox"
                bind:checked={$privacy_config.require_consensus}
                class="text-yellow-400 focus:ring-yellow-400"
              />
              <label class="text-sm text-gray-300">
                Require full consensus validation (all 4 proofs)
              </label>
            </div>
          </div>
        </div>
      </div>
    </Card>

    <!-- Advanced Settings -->
    {#if showAdvancedSettings}
      <Card class="p-6 border-yellow-400/20 bg-yellow-400/5">
        <h2 class="text-xl font-semibold mb-4 text-yellow-400">Advanced Configuration</h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <!-- Quantum Security -->
          <div>
            <h3 class="font-medium text-white mb-3">Quantum-Resistant Security</h3>
            <div class="space-y-2 text-sm">
              <div class="flex items-center justify-between">
                <span class="text-gray-300">FALCON-1024 Signatures</span>
                <span class="text-green-400">Enabled</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-gray-300">Kyber Encryption</span>
                <span class="text-green-400">Enabled</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-gray-300">Remote Proxy NAT</span>
                <span class="text-blue-400">Active</span>
              </div>
            </div>
          </div>

          <!-- Integration Status -->
          <div>
            <h3 class="font-medium text-white mb-3">Ecosystem Integration</h3>
            <div class="space-y-2 text-sm">
              <div class="flex items-center justify-between">
                <span class="text-gray-300">TrustChain Certificates</span>
                <span class="text-green-400">Validated</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-gray-300">STOQ Protocol</span>
                <span class="text-green-400">Connected</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-gray-300">Catalog JuliaVM</span>
                <span class="text-green-400">Ready</span>
              </div>
            </div>
          </div>
        </div>

        <div class="mt-6 p-4 bg-gray-800/50 rounded-lg">
          <div class="flex items-start space-x-3">
            <Shield class="w-5 h-5 text-yellow-400 mt-0.5" />
            <div>
              <p class="text-sm font-medium text-white">Security Notice</p>
              <p class="text-xs text-gray-400 mt-1">
                All HyperMesh resource sharing uses quantum-resistant encryption and requires consensus validation. 
                Your privacy settings are enforced at the protocol level with remote proxy addressing for maximum security.
              </p>
            </div>
          </div>
        </div>
      </Card>
    {/if}

    <!-- Earnings Summary -->
    <Card class="p-6 bg-gradient-to-r from-green-500/10 to-green-600/10 border-green-400/20">
      <div class="flex items-center space-x-4">
        <div class="w-12 h-12 bg-green-400/20 rounded-lg flex items-center justify-center">
          <TrendingUp class="w-6 h-6 text-green-400" />
        </div>
        <div>
          <h3 class="font-semibold text-white">Resource Sharing Economics</h3>
          <p class="text-green-400 text-lg font-bold">
            {$hypermesh_allocations.earnings_rate} CAESAR/hour
          </p>
          <p class="text-sm text-gray-300">
            Estimated monthly earnings: {($hypermesh_allocations.earnings_rate * 24 * 30).toFixed(2)} CAESAR
          </p>
        </div>
      </div>
    </Card>
  {/if}
</div>

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .slider::-webkit-slider-thumb {
    appearance: none;
    height: 20px;
    width: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    cursor: pointer;
    box-shadow: 0 0 10px rgba(245, 158, 11, 0.5);
  }

  .slider::-moz-range-thumb {
    height: 20px;
    width: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    cursor: pointer;
    border: none;
    box-shadow: 0 0 10px rgba(245, 158, 11, 0.5);
  }
</style>
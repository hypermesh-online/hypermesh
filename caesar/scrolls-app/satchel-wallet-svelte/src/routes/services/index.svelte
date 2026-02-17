<script>
  import { onMount } from 'svelte';
  import { 
    service_subscriptions,
    caesar_economy,
    assetStore,
    asset_manager_connected
  } from '../../lib/stores/assets.js';
  import { 
    Zap,
    Wifi,
    Cloud,
    Shield,
    CreditCard,
    Calendar,
    TrendingUp,
    Plus,
    Settings,
    CheckCircle
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';

  function getServiceIcon(service) {
    if (service.includes('Storage')) return Cloud;
    if (service.includes('Internet')) return Wifi;
    if (service.includes('Security')) return Shield;
    return Zap;
  }

  function formatDate(dateString) {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric'
    });
  }

  onMount(() => {
    if ($asset_manager_connected) {
      assetStore.loadServiceSubscriptions();
    }
  });
</script>

<svelte:head>
  <title>Services - HyperMesh Asset Manager</title>
  <meta name="description" content="Manage your service subscriptions and utility payments with CAESAR tokens" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Service Management</h1>
      <p class="text-gray-400 mt-2">
        Manage subscriptions and pay for real services with CAESAR tokens
      </p>
    </div>
    
    <Button>
      <Plus class="w-4 h-4 mr-2" />
      Add Service
    </Button>
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <Zap class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to manage your service subscriptions and payments.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Service Overview -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <!-- Total Monthly Costs -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Monthly Service Costs</p>
            <p class="text-2xl font-bold text-white">
              {$service_subscriptions.reduce((total, service) => total + service.monthly_cost, 0).toFixed(2)} CAESAR
            </p>
          </div>
          <CreditCard class="w-8 h-8 text-yellow-400" />
        </div>
      </Card>

      <!-- Active Services -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">Active Services</p>
            <p class="text-2xl font-bold text-white">{$service_subscriptions.length}</p>
          </div>
          <CheckCircle class="w-8 h-8 text-green-400" />
        </div>
      </Card>

      <!-- Caesar Balance -->
      <Card class="p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-400">CAESAR Balance</p>
            <p class="text-2xl font-bold text-white">{$caesar_economy.balance} CAESAR</p>
          </div>
          <TrendingUp class="w-8 h-8 text-yellow-400" />
        </div>
        <div class="mt-4 flex items-center text-sm">
          <span class="text-green-400">Anti-speculation score: {$caesar_economy.anti_speculation_score}%</span>
        </div>
      </Card>
    </div>

    <!-- Service Categories -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Cloud Services -->
      <Card class="p-6">
        <div class="flex items-center space-x-3 mb-4">
          <Cloud class="w-6 h-6 text-blue-400" />
          <h2 class="text-xl font-semibold">Cloud & Storage Services</h2>
        </div>
        
        {#each $service_subscriptions.filter(s => s.service.includes('Storage')) as service}
          <div class="bg-gray-800/50 rounded-lg p-4 mb-4">
            <div class="flex items-start justify-between mb-3">
              <div>
                <h3 class="font-semibold text-white">{service.service}</h3>
                <p class="text-sm text-gray-400">{service.provider}</p>
              </div>
              <span class="text-lg font-bold text-white">{service.monthly_cost} CAESAR</span>
            </div>
            
            <div class="grid grid-cols-2 gap-4 mb-3">
              <div>
                <p class="text-xs text-gray-400">Plan</p>
                <p class="text-sm text-white">{service.plan}</p>
              </div>
              <div>
                <p class="text-xs text-gray-400">Usage</p>
                <p class="text-sm text-white">{service.usage.used}GB / {service.usage.available}GB</p>
              </div>
            </div>

            <!-- Usage Bar -->
            <div class="w-full bg-gray-700 rounded-full h-2 mb-3">
              <div 
                class="bg-blue-400 h-2 rounded-full"
                style="width: {(service.usage.used / service.usage.available) * 100}%"
              ></div>
            </div>

            <!-- Features -->
            <div class="mb-4">
              <p class="text-xs text-gray-400 mb-2">Features</p>
              <div class="flex flex-wrap gap-2">
                {#each service.features as feature}
                  <span class="px-2 py-1 text-xs bg-blue-500/20 text-blue-400 rounded-full">
                    {feature}
                  </span>
                {/each}
              </div>
            </div>

            <div class="flex space-x-2">
              <Button size="sm" variant="outline" class="flex-1">
                <Settings class="w-4 h-4 mr-1" />
                Manage
              </Button>
              <Button size="sm" variant="outline" class="flex-1">
                Usage Details
              </Button>
            </div>
          </div>
        {/each}
        
        {#if $service_subscriptions.filter(s => s.service.includes('Storage')).length === 0}
          <div class="text-center py-8">
            <Cloud class="w-12 h-12 text-gray-400 mx-auto mb-3" />
            <p class="text-gray-400">No cloud services</p>
            <Button size="sm" class="mt-2">
              Add Cloud Service
            </Button>
          </div>
        {/if}
      </Card>

      <!-- Utility Services -->
      <Card class="p-6">
        <div class="flex items-center space-x-3 mb-4">
          <Zap class="w-6 h-6 text-yellow-400" />
          <h2 class="text-xl font-semibold">Utility & Infrastructure</h2>
        </div>
        
        <!-- Placeholder for utility services (from contracts) -->
        <div class="bg-gradient-to-r from-yellow-400/10 to-yellow-600/10 border border-yellow-400/20 rounded-lg p-4 mb-4">
          <div class="flex items-center space-x-3 mb-3">
            <Zap class="w-6 h-6 text-yellow-400" />
            <div>
              <h3 class="font-semibold text-white">Utility Payments</h3>
              <p class="text-sm text-gray-400">Managed via Contracts</p>
            </div>
          </div>
          
          <p class="text-sm text-gray-300 mb-3">
            Electricity, water, gas, and internet services are managed through the Contracts section 
            with automatic CAESAR payments.
          </p>
          
          <Button size="sm" variant="outline">
            <Calendar class="w-4 h-4 mr-1" />
            View Contracts
          </Button>
        </div>

        <!-- HyperMesh Services -->
        <div class="bg-gradient-to-r from-green-400/10 to-green-600/10 border border-green-400/20 rounded-lg p-4">
          <div class="flex items-center space-x-3 mb-3">
            <Shield class="w-6 h-6 text-green-400" />
            <div>
              <h3 class="font-semibold text-white">HyperMesh Network</h3>
              <p class="text-sm text-gray-400">Core Infrastructure</p>
            </div>
          </div>
          
          <div class="grid grid-cols-2 gap-4 mb-3 text-sm">
            <div>
              <p class="text-gray-400">Consensus Validation</p>
              <p class="text-green-400">Active</p>
            </div>
            <div>
              <p class="text-gray-400">Quantum Security</p>
              <p class="text-green-400">Enabled</p>
            </div>
            <div>
              <p class="text-gray-400">Remote Proxy</p>
              <p class="text-blue-400">Connected</p>
            </div>
            <div>
              <p class="text-gray-400">Resource Sharing</p>
              <p class="text-yellow-400">Earning</p>
            </div>
          </div>
          
          <Button size="sm" variant="outline">
            <Settings class="w-4 h-4 mr-1" />
            Configure HyperMesh
          </Button>
        </div>
      </Card>
    </div>

    <!-- Service Benefits & Economics -->
    <Card class="p-6 bg-gradient-to-r from-purple-500/10 to-purple-600/10 border-purple-400/20">
      <div class="flex items-start space-x-4">
        <div class="w-12 h-12 bg-purple-400/20 rounded-lg flex items-center justify-center">
          <TrendingUp class="w-6 h-6 text-purple-400" />
        </div>
        <div>
          <h3 class="font-semibold text-white mb-2">CAESAR Service Economy Benefits</h3>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <p class="text-purple-400 font-medium">Anti-Speculation Design</p>
              <p class="text-gray-300">
                CAESAR tokens have demurrage (holding tax) encouraging real utility spending over speculation.
              </p>
            </div>
            <div>
              <p class="text-purple-400 font-medium">Real Economy Focus</p>
              <p class="text-gray-300">
                Pay for actual services like electricity, internet, and cloud storage, not speculative assets.
              </p>
            </div>
            <div>
              <p class="text-purple-400 font-medium">HyperMesh Integration</p>
              <p class="text-gray-300">
                Earn CAESAR by sharing your resources (CPU, storage, bandwidth) with the HyperMesh network.
              </p>
            </div>
          </div>
          
          <div class="mt-4 p-3 bg-gray-800/50 rounded-lg">
            <p class="text-xs text-gray-400">
              Your anti-speculation score: <span class="text-green-400 font-medium">{$caesar_economy.anti_speculation_score}%</span>
              • Higher scores earn better rates and priority access to services.
            </p>
          </div>
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
</style>
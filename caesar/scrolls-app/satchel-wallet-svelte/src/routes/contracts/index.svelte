<script>
  import { onMount } from 'svelte';
  import { 
    active_contracts,
    assetStore,
    asset_manager_connected,
    loading_states
  } from '../../lib/stores/assets.js';
  import { 
    FileText,
    Zap,
    Phone,
    Home,
    Car,
    Calendar,
    DollarSign,
    CheckCircle,
    AlertCircle,
    Play,
    Settings
  } from 'lucide-svelte';
  import Card from '../../lib/components/ui/Card.svelte';
  import Button from '../../lib/components/ui/Button.svelte';

  let selectedContractType = 'all';

  // Filter contracts by type
  $: filteredContracts = $active_contracts.filter(contract => {
    if (selectedContractType === 'all') return true;
    return contract.type === selectedContractType;
  });

  function getContractIcon(type) {
    switch(type) {
      case 'utility': return Zap;
      case 'telecommunications': return Phone;
      case 'employment': return FileText;
      case 'real_estate': return Home;
      case 'vehicle': return Car;
      default: return FileText;
    }
  }

  function getStatusColor(status) {
    switch(status) {
      case 'active': return 'text-green-400';
      case 'pending': return 'text-yellow-400';
      case 'expired': return 'text-red-400';
      default: return 'text-gray-400';
    }
  }

  function formatDate(dateString) {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric'
    });
  }

  function getNextPaymentStatus(nextDue) {
    const dueDate = new Date(nextDue);
    const now = new Date();
    const daysUntilDue = Math.ceil((dueDate - now) / (1000 * 60 * 60 * 24));
    
    if (daysUntilDue < 0) return { status: 'overdue', text: 'Overdue', class: 'text-red-400' };
    if (daysUntilDue <= 3) return { status: 'due-soon', text: `Due in ${daysUntilDue} days`, class: 'text-yellow-400' };
    return { status: 'upcoming', text: `Due in ${daysUntilDue} days`, class: 'text-gray-400' };
  }

  async function executeContract(contractId) {
    try {
      await assetStore.executeContract(contractId, {});
      // TODO: Show success notification
    } catch (error) {
      console.error('Failed to execute contract:', error);
      // TODO: Show error notification  
    }
  }

  onMount(() => {
    if ($asset_manager_connected) {
      assetStore.loadActiveContracts();
    }
  });
</script>

<svelte:head>
  <title>Contracts - HyperMesh Asset Manager</title>
  <meta name="description" content="Manage real-world contracts including utilities, employment, and services" />
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold caesar-gradient">Contract Management</h1>
      <p class="text-gray-400 mt-2">
        Execute and monitor real-world contracts through HyperMesh
      </p>
    </div>
    
    <Button>
      <FileText class="w-4 h-4 mr-2" />
      New Contract
    </Button>
  </div>

  <!-- Contract Type Filter -->
  <div class="flex space-x-2">
    <Button 
      variant={selectedContractType === 'all' ? 'default' : 'outline'} 
      size="sm"
      on:click={() => selectedContractType = 'all'}
    >
      All Contracts ({$active_contracts.length})
    </Button>
    <Button 
      variant={selectedContractType === 'utility' ? 'default' : 'outline'} 
      size="sm"
      on:click={() => selectedContractType = 'utility'}
    >
      Utilities ({$active_contracts.filter(c => c.type === 'utility').length})
    </Button>
    <Button 
      variant={selectedContractType === 'telecommunications' ? 'default' : 'outline'} 
      size="sm"
      on:click={() => selectedContractType = 'telecommunications'}
    >
      Telecom ({$active_contracts.filter(c => c.type === 'telecommunications').length})
    </Button>
    <Button 
      variant={selectedContractType === 'employment' ? 'default' : 'outline'} 
      size="sm"
      on:click={() => selectedContractType = 'employment'}
    >
      Employment ({$active_contracts.filter(c => c.type === 'employment').length})
    </Button>
  </div>

  {#if !$asset_manager_connected}
    <!-- Connection Prompt -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <FileText class="w-16 h-16 text-yellow-400 mx-auto" />
        <h2 class="text-xl font-semibold">Connect to HyperMesh</h2>
        <p class="text-gray-400 max-w-md mx-auto">
          Connect to the HyperMesh ecosystem to manage your contracts and execute real-world agreements.
        </p>
        <Button on:click={() => assetStore.connectAssetManager()} class="mt-4">
          Connect HyperMesh
        </Button>
      </div>
    </Card>
  {:else if filteredContracts.length === 0}
    <!-- No Contracts -->
    <Card class="text-center py-12">
      <div class="space-y-4">
        <FileText class="w-16 h-16 text-gray-400 mx-auto" />
        <h2 class="text-xl font-semibold text-gray-300">No Contracts Found</h2>
        <p class="text-gray-400">
          {selectedContractType === 'all' ? 'You have no active contracts' : `No ${selectedContractType} contracts found`}
        </p>
        <Button class="mt-4">
          Create Contract
        </Button>
      </div>
    </Card>
  {:else}
    <!-- Contract List -->
    <div class="space-y-4">
      {#each filteredContracts as contract (contract.id)}
        <Card class="p-6">
          <div class="flex items-start justify-between">
            <!-- Contract Info -->
            <div class="flex items-start space-x-4">
              <div class="w-12 h-12 bg-gray-800 rounded-lg flex items-center justify-center">
                <svelte:component this={getContractIcon(contract.type)} class="w-6 h-6 text-yellow-400" />
              </div>
              
              <div class="flex-1">
                <div class="flex items-center space-x-3 mb-2">
                  <h3 class="text-lg font-semibold text-white">{contract.service}</h3>
                  <span class="px-2 py-1 text-xs rounded-full bg-green-500/20 text-green-400 capitalize">
                    {contract.type.replace('_', ' ')}
                  </span>
                </div>
                
                <p class="text-gray-400 mb-3">{contract.provider}</p>
                
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                  <!-- Monthly Cost -->
                  <div>
                    <p class="text-sm text-gray-400">Monthly Cost</p>
                    <p class="font-semibold text-white">
                      ${contract.monthly_cost}
                      {#if contract.payment_method === 'CAESAR'}
                        <span class="text-sm text-yellow-400 ml-1">CAESAR</span>
                      {/if}
                    </p>
                  </div>
                  
                  <!-- Next Payment -->
                  <div>
                    <p class="text-sm text-gray-400">Next Payment</p>
                    <p class="font-semibold {getNextPaymentStatus(contract.next_due).class}">
                      {formatDate(contract.next_due)}
                    </p>
                    <p class="text-xs {getNextPaymentStatus(contract.next_due).class}">
                      {getNextPaymentStatus(contract.next_due).text}
                    </p>
                  </div>
                  
                  <!-- Last Payment -->
                  {#if contract.last_payment}
                    <div>
                      <p class="text-sm text-gray-400">Last Payment</p>
                      <p class="font-semibold text-white">
                        ${contract.last_payment.amount}
                      </p>
                      <p class="text-xs text-gray-500">
                        {formatDate(contract.last_payment.date)}
                      </p>
                    </div>
                  {/if}
                </div>

                <!-- Special Features -->
                {#if contract.usage_sharing}
                  <div class="mt-4 p-3 bg-blue-500/10 rounded-lg">
                    <div class="flex items-center space-x-2 mb-2">
                      <Zap class="w-4 h-4 text-blue-400" />
                      <span class="text-sm font-medium text-blue-400">HyperMesh Sharing Active</span>
                    </div>
                    <div class="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <p class="text-gray-400">Bandwidth Shared</p>
                        <p class="text-white">{contract.usage_sharing.bandwidth_shared} Mbps</p>
                      </div>
                      <div>
                        <p class="text-gray-400">Monthly Earnings</p>
                        <p class="text-green-400">+{contract.usage_sharing.earnings_per_month} CAESAR</p>
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            </div>

            <!-- Actions -->
            <div class="flex flex-col space-y-2">
              {#if contract.auto_pay}
                <div class="flex items-center space-x-2">
                  <CheckCircle class="w-4 h-4 text-green-400" />
                  <span class="text-sm text-green-400">Auto-pay enabled</span>
                </div>
              {:else}
                <div class="flex items-center space-x-2">
                  <AlertCircle class="w-4 h-4 text-yellow-400" />
                  <span class="text-sm text-yellow-400">Manual payment</span>
                </div>
              {/if}
              
              <div class="flex space-x-2">
                <Button 
                  size="sm" 
                  variant="outline"
                  disabled={$loading_states.processing_contract}
                >
                  <Settings class="w-4 h-4" />
                </Button>
                
                {#if !contract.auto_pay && getNextPaymentStatus(contract.next_due).status === 'due-soon'}
                  <Button 
                    size="sm"
                    on:click={() => executeContract(contract.id)}
                    disabled={$loading_states.processing_contract}
                  >
                    {#if $loading_states.processing_contract}
                      Processing...
                    {:else}
                      <Play class="w-4 h-4 mr-1" />
                      Pay Now
                    {/if}
                  </Button>
                {/if}
              </div>
            </div>
          </div>
        </Card>
      {/each}
    </div>
  {/if}

  <!-- Contract Execution via JuliaVM Info -->
  <Card class="p-6 bg-gradient-to-r from-yellow-400/10 to-yellow-600/10 border-yellow-400/20">
    <div class="flex items-start space-x-4">
      <div class="w-10 h-10 bg-yellow-400/20 rounded-lg flex items-center justify-center">
        <Zap class="w-5 h-5 text-yellow-400" />
      </div>
      <div>
        <h3 class="font-semibold text-white mb-2">Secure Contract Execution</h3>
        <p class="text-gray-300 text-sm mb-3">
          All contracts are executed through Catalog's JuliaVM with full consensus validation 
          (PoSpace + PoStake + PoWork + PoTime) ensuring secure, verifiable real-world transactions.
        </p>
        <div class="flex space-x-4 text-xs text-gray-400">
          <span>• Quantum-resistant security</span>
          <span>• Anti-speculation mechanics</span>
          <span>• Real utility focus</span>
        </div>
      </div>
    </div>
  </Card>
</div>

<style>
  .caesar-gradient {
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
</style>
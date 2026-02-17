<script>
  import { url } from '@roxi/routify';
  import { 
    Home,
    Package,
    FileText,
    Settings,
    Zap,
    LogOut,
    ChevronDown,
    ShoppingCart,
    Shield
  } from 'lucide-svelte';
  import Button from './ui/Button.svelte';
  import { assetStore, asset_manager_connected, user_identity } from '../stores/assets.js';
  
  let dropdownOpen = false;
  
  function truncateIdentity(identity) {
    if (!identity) return '';
    return `${identity.slice(0, 6)}...${identity.slice(-4)}`;
  }
  
  async function connectAssetManager() {
    try {
      await assetStore.connectAssetManager();
    } catch (error) {
      console.error('Failed to connect asset manager:', error);
      // TODO: Show error toast
    }
  }
  
  async function disconnectAssetManager() {
    await assetStore.disconnectAssetManager();
    dropdownOpen = false;
  }
</script>

<nav class="border-b border-gray-800 bg-gray-900/50 backdrop-blur-sm sticky top-0 z-50">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="flex justify-between items-center h-16">
      <!-- Logo -->
      <div class="flex items-center space-x-4">
        <a href={$url('/')} class="flex items-center space-x-3">
          <div class="w-10 h-10 rounded-full overflow-hidden bg-gradient-to-r from-yellow-400 to-yellow-600 p-1">
            <img src="/caesar-logo.png" alt="Caesar Logo" class="w-full h-full object-cover rounded-full" />
          </div>
          <div class="flex flex-col">
            <span class="text-xl font-bold caesar-gradient">Caesar Assets</span>
            <span class="text-xs text-gray-400 -mt-1">HyperMesh Universal Economy</span>
          </div>
        </a>
      </div>

      <!-- Navigation Links -->
      <div class="hidden md:flex items-center space-x-1">
        <a 
          href={$url('/')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <Home class="w-4 h-4" />
          <span>Dashboard</span>
        </a>
        <a 
          href={$url('/assets')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <Package class="w-4 h-4" />
          <span>Assets</span>
        </a>
        <a 
          href={$url('/contracts')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <FileText class="w-4 h-4" />
          <span>Contracts</span>
        </a>
        <a 
          href={$url('/services')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <Zap class="w-4 h-4" />
          <span>Services</span>
        </a>
        <a 
          href={$url('/trade')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <ShoppingCart class="w-4 h-4" />
          <span>Trade</span>
        </a>
        <a 
          href={$url('/hypermesh')} 
          class="px-3 py-2 rounded-md text-sm font-medium text-gray-300 hover:text-yellow-400 hover:bg-gray-800 transition-colors flex items-center space-x-1"
        >
          <Shield class="w-4 h-4" />
          <span>HyperMesh</span>
        </a>
      </div>

      <!-- Asset Manager Connection -->
      <div class="flex items-center">
        {#if $asset_manager_connected}
          <div class="relative">
            <button
              on:click={() => dropdownOpen = !dropdownOpen}
              class="flex items-center space-x-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-md transition-colors"
            >
              <div class="w-3 h-3 bg-green-400 rounded-full"></div>
              <span class="text-sm font-medium">{truncateIdentity($user_identity)}</span>
              <ChevronDown class="w-4 h-4" />
            </button>
            
            {#if dropdownOpen}
              <div class="absolute right-0 mt-2 w-48 bg-gray-800 rounded-md shadow-lg border border-gray-700 z-10">
                <div class="py-1">
                  <a
                    href={$url('/settings')}
                    class="flex items-center space-x-2 w-full px-4 py-2 text-sm text-gray-300 hover:text-yellow-400 hover:bg-gray-700 transition-colors"
                  >
                    <Settings class="w-4 h-4" />
                    <span>Settings</span>
                  </a>
                  <button
                    on:click={disconnectAssetManager}
                    class="flex items-center space-x-2 w-full px-4 py-2 text-sm text-gray-300 hover:text-red-400 hover:bg-gray-700 transition-colors"
                  >
                    <LogOut class="w-4 h-4" />
                    <span>Disconnect</span>
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <Button on:click={connectAssetManager} size="sm">
            Connect HyperMesh
          </Button>
        {/if}
      </div>
    </div>
  </div>
</nav>

<!-- Mobile Navigation (TODO: Implement responsive mobile menu) -->